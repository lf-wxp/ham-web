import type { Page } from "@playwright/test";

import { expect, test, waitSettled } from "./fixtures";

// 现代风的断点回归。
//
// 为什么单独一个文件：其余用例只覆盖 320 / 390（手机）与 1280 / 1440（桌面），
// 而实际出过的问题恰好都在中间 —— 808px 卡片已经三列、导航还是手机 dock；
// 390px 考试页的 dock 压在底栏上，把「交卷」挡成了导航项。
// 判据与 `crates/app/style/modern/kit.css` 头部的断点表一一对应。

async function seed(page: Page) {
  await page.addInitScript(() => {
    if (window !== window.top) return;
    localStorage.setItem("ui:uiStyle", "modern");
    localStorage.setItem("theme", "dark");
    localStorage.setItem("locale", "zh");
  });
}

async function open(page: Page, url: string) {
  await page.goto(url);
  await page.waitForSelector("main", { timeout: 15_000 });
  await waitSettled(page);
}

/**
 * 某个网格实际分了几列。
 *
 * 不看 `grid-template-columns` 的字符串：单列时浏览器会给 `minmax(0px, 1fr)`，
 * 里面本身就带一个空格，按空格拆会把「一列」数成两列。改用几何判据 ——
 * 与第一张卡片顶端对齐的卡片数就是列数，量的是真正排出来的版式。
 */
function gridColumns(page: Page, selector: string) {
  return page.evaluate((s) => {
    const el = document.querySelector(s);
    if (!el) return -1;
    const tops = Array.from(el.children)
      .filter((child) => child.getBoundingClientRect().height > 0)
      .map((child) => Math.round(child.getBoundingClientRect().top));
    if (tops.length === 0) return 0;
    const first = tops[0];
    return tops.filter((top) => Math.abs(top - first) <= 2).length;
  }, selector);
}

async function navListVisible(page: Page) {
  return page.evaluate(() => {
    const el = document.querySelector(".md-nav-list");
    return el ? getComputedStyle(el).display !== "none" : false;
  });
}

test.describe("现代风断点", () => {
  test("808px：平板紧凑档——导航已回顶栏，栅格仍是单列", async ({ page }) => {
    await seed(page);
    await page.setViewportSize({ width: 808, height: 900 });
    await open(page, "/");

    // 808 属于「平板紧凑档」：导航已经回顶栏（768 起），但完整 6 项要 900 才放得下。
    expect(await navListVisible(page), "808px 应有顶栏导航").toBe(true);

    // 关键回归：`.md-bento` / `.md-modules` 曾同时有「视口 900」与「容器 760」两套阈值，
    // 容器宽 = 视口 − 2×24，于是 808px 就变成三列（卡片内容宽只剩 129px），
    // 而导航这一刻还是手机态的 dock —— 一屏里两套布局语言。
    expect(await gridColumns(page, ".md-bento"), "808px 的行动卡必须与导航态同步为单列").toBe(1);

    // 模块区在 640–899 是**有意的横向滑动轨道**（`interaction.css` 的
    // `.md-modules.md-track`：display flex + 每张 78% 宽），不是三列网格。
    // 这里断言的正是「它没有退回网格」——两者混用会出现「能滑又能排两行」的怪状态。
    const modulesDisplay = await page.evaluate(() => {
      const el = document.querySelector(".md-modules");
      return el ? getComputedStyle(el).display : null;
    });
    expect(modulesDisplay, "640–899 的模块区应为可横滑轨道").toBe("flex");

    // 紧凑档的前提是「顶栏放得下」，否则隐藏一项换来的只是溢出。
    const navOverflow = await page.evaluate(() => {
      const el = document.querySelector(".md-nav-inner");
      return el ? el.scrollWidth - el.clientWidth : -1;
    });
    expect(navOverflow, "顶栏在 808px 不得横向溢出").toBeLessThanOrEqual(1);
  });

  test("768px：平板改用顶栏导航，底部 dock 退场", async ({ page }) => {
    await seed(page);
    await page.setViewportSize({ width: 768, height: 1024 });
    await open(page, "/");

    await expect(page.locator(".md-nav-list")).toBeVisible();
    await expect(page.locator(".md-dock")).toBeHidden();
    // 紧凑档：完整的 6 项放不下西语，所以「日志」在这一档收进「更多」面板。
    await expect(page.locator('.md-nav-list a[href="/log"]')).toBeHidden();
  });

  test("900px：顶栏回到完整导航，行动卡三列", async ({ page }) => {
    await seed(page);
    await page.setViewportSize({ width: 900, height: 900 });
    await open(page, "/");

    await expect(page.locator('.md-nav-list a[href="/log"]')).toBeVisible();
    expect(await gridColumns(page, ".md-bento")).toBe(3);
  });

  test("390px 考试页：底栏按钮不被 dock 覆盖", async ({ page }) => {
    await seed(page);
    await page.setViewportSize({ width: 390, height: 844 });
    await open(page, "/exam");

    // 规则页 / 考前页：能点就点，已经在考试中就跳过。
    for (const name of [/我已了解/, /开始考试/]) {
      const button = page.getByRole("button", { name }).first();
      if ((await button.count()) > 0) {
        await button.click();
        await waitSettled(page);
        break;
      }
    }

    const bar = page.locator(".md-bar");
    await expect(bar).toBeVisible();

    // 两个层都 `position: fixed` 在同一个底边：dock 的 z-index 更高（55 vs 40），
    // 只要它没被收起，就会整条压住考试底栏 —— 交卷按钮的中心点会命中 dock 的格子。
    const probe = await page.evaluate(() => {
      const barEl = document.querySelector(".md-bar");
      if (!barEl) return { ok: false, reason: "没有考试底栏", dockVisible: false };
      const dock = document.querySelector(".md-dock");
      const dockVisible = dock ? getComputedStyle(dock).display !== "none" : false;
      const submit = Array.from(barEl.querySelectorAll("button")).find((b) =>
        /交卷/.test(b.textContent ?? ""),
      );
      if (!submit) return { ok: false, reason: "没有交卷按钮", dockVisible };
      const rect = submit.getBoundingClientRect();
      const hit = document.elementFromPoint(
        rect.left + rect.width / 2,
        Math.min(rect.top + rect.height / 2, window.innerHeight - 1),
      );
      return {
        ok: !!hit && submit.contains(hit),
        reason: hit ? String(hit.className).slice(0, 40) : "视口外",
        dockVisible,
      };
    });

    expect(probe.dockVisible, "考试页必须收起 dock").toBe(false);
    expect(probe.ok, `交卷按钮被 ${probe.reason} 覆盖`).toBe(true);
  });

  // `.md-root` 是「字体作用域」，页面里、页脚里被反复嵌套；让位用的 `padding-bottom`
  // 以前写在它身上，嵌一层叠一次 —— 普通页尾多出 168–252px 空白，考试页（184px 一档）叠到 736px。
  // 判据不写死像素：看「同一条祖先链上所有 `.md-root` 的 padding-bottom 之和」不超过最大的单层，
  // 即只有一层承担让位；同时断言末行没被固定层盖住（只留一层也不能漏让位）。
  for (const [name, url, width, height] of [
    ["普通页（有页脚）", "/stats", 390, 844],
    ["普通页（有页脚，嵌套页根）", "/glossary", 390, 844],
    ["考试页（页脚隐藏，底栏两排）", "/exam", 320, 568],
    ["每日挑战（页脚隐藏，dock 仍在）", "/daily-challenge", 390, 844],
  ] as const) {
    test(`底部让位只算一次：${name}`, async ({ page }) => {
      await seed(page);
      await page.setViewportSize({ width, height });
      await open(page, url);
      // 练习 / 考试进入后有「规则页 / 考前页」：能点就点，已在作答中就跳过。
      for (const label of [/我已了解/, /开始考试/]) {
        const button = page.getByRole("button", { name: label }).first();
        if ((await button.count()) > 0) {
          await button.click();
          await waitSettled(page);
          break;
        }
      }
      await page.evaluate(() => window.scrollTo(0, 1e6));
      const r = await page.evaluate(() => {
        const pads = (el: Element) => {
          const chain: number[] = [];
          for (let p: Element | null = el; p; p = p.parentElement) {
            if (p.classList.contains("md-root")) {
              chain.push(parseFloat(getComputedStyle(p).paddingBottom));
            }
          }
          return chain;
        };
        const roots = Array.from(document.querySelectorAll(".md-root"));
        const chains = roots.map(pads);
        const worst = chains.reduce(
          (acc, c) => {
            const sum = c.reduce((a, b) => a + b, 0);
            return sum > acc.sum ? { sum, max: Math.max(...c, 0) } : acc;
          },
          { sum: 0, max: 0 },
        );
        // 固定层（dock / 底栏）的顶边与「最后一个有内容的叶子」的底边。
        const covers = Array.from(document.querySelectorAll(".md-dock, .md-bar, .md-bar-practice")).filter(
          (b) => getComputedStyle(b).display !== "none" && getComputedStyle(b).position === "fixed",
        );
        const coverTop = covers.length
          ? Math.min(...covers.map((b) => b.getBoundingClientRect().top))
          : window.innerHeight;
        const leaves = Array.from(document.querySelectorAll("body *")).filter((e) => {
          if (e.closest(".md-bar, .md-dock, [data-modal-root], nav.md-nav, [data-skip-link]")) return false;
          const cs = getComputedStyle(e);
          if (cs.display === "none" || cs.visibility === "hidden" || cs.position === "fixed") return false;
          const b = e.getBoundingClientRect();
          if (b.height < 6 || b.width < 6) return false;
          return (
            Array.from(e.childNodes).some((n) => n.nodeType === 3 && (n.nodeValue ?? "").trim()) ||
            /^(BUTTON|A|INPUT|TEXTAREA|SELECT)$/.test(e.tagName)
          );
        });
        const lastBottom = Math.max(...leaves.map((e) => e.getBoundingClientRect().bottom));
        return { ...worst, clear: Math.round(coverTop - lastBottom), roots: roots.length };
      });
      expect(r.roots, "没找到 .md-root，用例失去意义").toBeGreaterThan(0);
      expect(
        r.sum,
        `同一条祖先链上 .md-root 的 padding-bottom 叠加了 ${r.sum}px（单层最大 ${r.max}px）：让位只能算一次`,
      ).toBeLessThanOrEqual(r.max);
      expect(r.clear, "页面末行被固定层（dock / 底栏）盖住了").toBeGreaterThanOrEqual(0);
    });
  }

  test("320px：窄屏表格在自己的容器里滚，不撑破页面", async ({ page }) => {
    await seed(page);
    await page.setViewportSize({ width: 320, height: 720 });
    await open(page, "/reference");

    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - window.innerWidth,
    );
    expect(overflow, "整页不得横向滚动").toBeLessThanOrEqual(1);

    // 有滚动容器的表格必须有列宽下限，否则会先被挤扁再滚。
    const table = await page.evaluate(() => {
      const wrap = document.querySelector(".md-table-scroll");
      const el = wrap?.querySelector(".md-table");
      if (!el) return null;
      return {
        min: parseFloat(getComputedStyle(el).minWidth || "0"),
        scrollable: wrap ? getComputedStyle(wrap).overflowX : null,
      };
    });
    // 不用 `if (table)` 跳过：页面上没有滚动容器里的表格，这条用例就什么也没测，应当红。
    expect(table, "/reference 应有 .md-table-scroll 里的表格").not.toBeNull();
    expect(table!.scrollable).toBe("auto");
    // 520 = 最宽的列组在 320 视口下的可读下限（见 `kit.css` 的 `.md-table` 最小宽度）。
    expect(table!.min).toBeGreaterThanOrEqual(520);
  });

  // 台账 R1：320 下顶栏右侧的余量是 0，任何一个新入口都会立刻溢出。
  // 360 那一档把品牌**文字**收掉、只留站标，并靠 `aria-label` 保住可访问名。
  test("320px：顶栏品牌只留站标、链接仍可被读屏识别", async ({ page }) => {
    await seed(page);
    await page.setViewportSize({ width: 320, height: 720 });
    await open(page, "/");

    const narrow = await page.evaluate(() => {
      const brand = document.querySelector(".md-brand");
      const text = document.querySelector(".md-brand-text");
      const actions = document.querySelector(".md-nav-actions");
      const inner = document.querySelector(".md-nav-inner, .md-shell");
      const r = (el: Element | null) => (el ? el.getBoundingClientRect() : null);
      const a = r(actions);
      const i = r(inner);
      return {
        textShown: text ? getComputedStyle(text).display !== "none" : true,
        brandWidth: r(brand)?.width ?? 0,
        slack: a && i ? Math.round(i.right - a.right) : -1,
        overflow: document.documentElement.scrollWidth - window.innerWidth,
      };
    });
    expect(narrow.textShown, "320 下品牌文字应收起").toBe(false);
    expect(narrow.brandWidth, "只剩站标，宽度应远小于带文字时（约 131px）").toBeLessThan(60);
    expect(narrow.slack, "右侧操作区与内容边界之间必须留出余量").toBeGreaterThanOrEqual(12);
    expect(narrow.overflow).toBeLessThanOrEqual(1);
    // 可访问名不能依赖被 `display:none` 的文字。
    await expect(page.getByRole("link", { name: "业余无线电" })).toHaveCount(1);
  });

  test("390px：品牌文字仍然显示（折叠只在 360 以下）", async ({ page }) => {
    await seed(page);
    await page.setViewportSize({ width: 390, height: 720 });
    await open(page, "/");
    const shown = await page.evaluate(() => {
      const text = document.querySelector(".md-brand-text");
      return text ? getComputedStyle(text).display !== "none" : false;
    });
    expect(shown, "390 是主流手机宽度，品牌名不该消失").toBe(true);
  });
});
