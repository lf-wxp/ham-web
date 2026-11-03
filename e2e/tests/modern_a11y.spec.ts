import AxeBuilder from "@axe-core/playwright";
import type { Page } from "@playwright/test";
import { expect, settleFrames, test } from "./fixtures";

// 现代风的 a11y 回归：WCAG 2.1 A/AA 零违规。
// 覆盖默认页面 × 深浅主题 + 三个「打开态」表面（调谐台 / 确认弹层 / 移动端菜单）。

async function seed(page: Page, theme: "dark" | "light" = "dark") {
  await page.addInitScript(([t]) => {
    if (window !== window.top) return;
    localStorage.setItem("ui:uiStyle", "modern");
    localStorage.setItem("theme", t);
    localStorage.setItem("ui:shortcutsHelpSeen:practice", "1");
    localStorage.setItem("ui:shortcutsHelpSeen:exam", "1");
  }, [theme]);
}

/** 等页面「静下来」：字体就绪 + 没有正在跑的动画（入场动画会把文字短暂压到低不透明度，
 *  并行跑时 axe 撞上动画中段就会误报 color-contrast）。 */
async function settle(page: Page) {
  // 不用 `networkidle`：首页等页面有定时轮询的接口，网络永远「不空闲」。
  // 以「应用已挂载」为准，再等字体与动画。
  await expect(page.locator("#route-content[data-sig]")).toBeAttached();
  await page.evaluate(async () => {
    await document.fonts.ready;
    const deadline = performance.now() + 1500;
    while (performance.now() < deadline) {
      const running = document
        .getAnimations()
        .filter((a) => a.playState === "running" && Number(a.effect?.getTiming().duration ?? 0) > 0);
      if (!running.length) break;
      await new Promise((r) => setTimeout(r, 60));
    }
  });
  await settleFrames(page);
}

async function expectClean(page: Page) {
  const results = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
    .analyze();
  const summary = results.violations
    .map((v) => `${v.id}(${v.nodes.length}): ${v.help}`)
    .join(" | ");
  expect(summary).toBe("");
}

const routes = ["/", "/practice", "/tools", "/log", "/glossary", "/map"];

test("现代风 · 深色：六个主页面零 a11y 违规", async ({ page }) => {
  await seed(page, "dark");
  for (const route of routes) {
    await page.goto(route);
    await settle(page);
    await expectClean(page);
  }
});

test("现代风 · 亮色：三个主页面零 a11y 违规", async ({ page }) => {
  await seed(page, "light");
  for (const route of ["/", "/log", "/glossary"]) {
    await page.goto(route);
    await settle(page);
    await expectClean(page);
  }
});

test("打开态表面：调谐台 / 确认弹层 / 移动端菜单零违规", async ({ page }) => {
  await seed(page, "dark");
  await page.goto("/");
  await page.keyboard.press("Control+k");
  // 必须断言表面真的打开了：没打开时 axe 扫的是底层页面，同样「零违规」。
  await expect(page.locator(".md-palette")).toBeVisible();
  await settle(page);
  await expectClean(page);
  await page.keyboard.press("Escape");

  await page.goto("/bookmarks");
  await page.getByRole("textbox").first().fill("临时组");
  await page.getByRole("button", { name: "新建分组" }).first().click();
  await page.getByRole("button", { name: "删除该分组" }).first().click();
  await expect(page.locator(".md-modal-panel")).toBeVisible();
  await settle(page);
  await expectClean(page);
  await page.keyboard.press("Escape");

  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  await page.locator('.md-dock button:has-text("更多")').click();
  await expect(page.locator(".md-menu-panel")).toBeVisible();
  await settle(page);
  await expectClean(page);
});

test("打开态表面：设置 / 搜索 / 快捷键帮助 / 练习设置 / 答题卡 零违规", async ({ page }) => {
  await seed(page, "dark");

  // 设置抽屉
  await page.goto("/");
  await page.locator('button[aria-label="外观与动效"]').first().click();
  await expect(page.locator('[role="dialog"]')).toHaveCount(1);
  await settle(page);
  await expectClean(page);
  await page.keyboard.press("Escape");

  // 全站搜索（`/` 唤起，与调谐台是两套弹层）
  await page.keyboard.press("/");
  await expect(page.locator('[role="dialog"]')).toHaveCount(1);
  await settle(page);
  await expectClean(page);
  await page.keyboard.press("Escape");

  // 快捷键帮助
  await page.goto("/practice");
  await page.keyboard.press("?");
  await expect(page.locator(".md-modal-panel")).toHaveCount(1);
  await settle(page);
  await expectClean(page);
  await page.keyboard.press("Escape");

  // 练习设置
  await page.locator('button[aria-label="设置"]').first().click();
  await expect(page.locator(".md-modal-panel")).toHaveCount(1);
  await settle(page);
  await expectClean(page);
  await page.keyboard.press("Escape");

  // 答题卡抽屉
  await page.goto("/exam");
  await page.getByRole("button", { name: /答题卡/ }).first().click();
  await expect(page.locator(".md-sheet-panel")).toHaveCount(1);
  await settle(page);
  await expectClean(page);
});

// 状态色 token 的对比度（红 / 琥珀 / 金与纹章前景）。
//
// 这些颜色以前写死成「按深色底调的浅粉红 / 亮金」，浅色下红字只有 2.1:1、金字 1.2:1，而 axe 扫不到它们 ——
// 只有答错 / 告急 / 解锁这类状态才会出现。所以直接量 token 本身：对页面真实的四个底面各算一遍。
// 阈值：文字类 4.5（AA），图形类 3（WCAG 非文本对比度），实心面上的字 4.5。
test.describe("状态色 token 的对比度", () => {
  for (const theme of ["light", "dark"] as const) {
    test(`${theme === "light" ? "浅色" : "深色"}：文字 / 图形 / 实心面上的字都达标`, async ({ page }) => {
      await seed(page, theme);
      await page.goto("/");
      await expect(page.locator(".md-root").first()).toBeAttached();

      const result = await page.evaluate(() => {
        const cs = getComputedStyle(document.documentElement);
        const get = (n: string) => cs.getPropertyValue(n).trim();
        const lin = (v: number) => {
          const c = v / 255;
          return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
        };
        const lum = (hex: string) => {
          let h = hex.replace("#", "");
          if (h.length === 3) h = [...h].map((c) => c + c).join("");
          const [r, g, b] = [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16));
          return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
        };
        const ratio = (a: string, b: string) => {
          const [hi, lo] = [lum(a), lum(b)].sort((x, y) => y - x);
          return (hi + 0.05) / (lo + 0.05);
        };
        const surfaces = ["--md-bg", "--md-surface", "--md-surface-2", "--md-surface-3"];
        const rows: { name: string; min: number; need: number; value: string }[] = [];
        const onSurfaces = (token: string, need: number) => {
          const v = get(token);
          const min = Math.min(...surfaces.map((s) => ratio(v, get(s))));
          rows.push({ name: `${token} 对四个底面`, min, need, value: v });
        };
        const pair = (fg: string, bg: string, need: number) => {
          rows.push({ name: `${fg} 在 ${bg} 上`, min: ratio(get(fg), get(bg)), need, value: `${get(fg)} / ${get(bg)}` });
        };
        // 文字类
        for (const t of ["--md-danger-ink", "--md-warn-ink", "--md-gold-ink"]) onSurfaces(t, 4.5);
        // 图形类（描边、纹章图标）
        for (const t of ["--md-danger", "--md-blue-ink", "--md-violet-ink", "--md-green-ink"]) onSurfaces(t, 3);
        // 实心面上的字
        pair("--md-danger-fill-fg", "--md-danger-fill", 4.5);
        pair("--md-danger-fill-fg", "--md-danger-fill-hover", 4.5);
        pair("--md-danger-face-fg", "--md-danger-face", 4.5);
        pair("--md-warn-fill-fg", "--md-warn-fill", 4.5);
        pair("--md-green-fg", "--md-green", 4.5);
        pair("--md-blue-fg", "--md-blue", 4.5);
        return rows;
      });

      // 解析不到颜色会得到 NaN，而 `NaN < need` 为假：先排除，免得「全是 NaN」也算通过。
      for (const r of result) expect(Number.isFinite(r.min), `${r.name} 解析失败：${r.value}`).toBe(true);
      expect(result.length, "应量到全部 token").toBeGreaterThanOrEqual(13);
      const bad = result.filter((r) => r.min < r.need).map((r) => `${r.name} = ${r.min.toFixed(2)} < ${r.need}（${r.value}）`);
      expect(bad, bad.join("\n")).toEqual([]);
    });
  }
});
