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
  // 全局快捷键监听是页面（WASM）挂载时注册的，`goto` 只等到 `load`：没等页面出来就按键，
  // 这一次按键直接落空，后面的断言只能等超时。设置按钮就在同一个页面上，等它即等挂载。
  await expect(page.locator('button[aria-label="设置"]').first()).toBeVisible();
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

// 行动橙（`--md-orange`）是**填充色**，直接当文字在浅色底上只有 2.8–3.0:1。
// axe 对这批文字零报告 —— 它们落在渐变 / 半透明底上、或要滚进视口才渲染，算不出稳定的背景色，
// 所以不能指望全站 axe 兜底，必须点名量**真实渲染出来的文字**。
// 取色按元素自身向上合成半透明背景（与审查时的量测同一口径），阈值取 WCAG 对「大字」的 3 / 其余 4.5。
test.describe("橙色文字的对比度（浅色）", () => {
  const cases = [
    // [路由, 选择器, 说明, 阈值]
    ["/reference", ".md-area-digit", "分区数字（17px 粗体）", 4.5],
    ["/reference", ".md-rst-key", "RST 的 R / S / T", 4.5],
    ["/", ".md-display-gradient", "hero 第二行（36px+ 大字）", 3],
  ] as const;

  for (const [route, selector, label, need] of cases) {
    test(`${label}：${selector} 达到 ${need}:1`, async ({ page }) => {
      await seed(page, "light");
      await page.goto(route);
      await expect(page.locator(selector).first()).toBeVisible();
      await settleFrames(page);

      const r = await page.evaluate((sel) => {
        const parse = (c: string) => {
          // `color-mix()` 的计算值是 `color(srgb 0.87 0.26 0.1)`（0–1 的小数），不是 `rgb()`：
          // 只认 `rgb()` 会在恰好有补丁的元素上解析失败。两种形态都要认。
          const srgb = c.match(/color\(srgb\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)(?:\s*\/\s*([\d.]+))?\)/);
          if (srgb) {
            return {
              r: Number(srgb[1]) * 255,
              g: Number(srgb[2]) * 255,
              b: Number(srgb[3]) * 255,
              a: srgb[4] === undefined ? 1 : Number(srgb[4]),
            };
          }
          const m = c.match(/rgba?\(([^)]+)\)/);
          if (!m) return null;
          const p = m[1].split(/[ ,/]+/).filter(Boolean).map(Number);
          return { r: p[0], g: p[1], b: p[2], a: p.length > 3 ? p[3] : 1 };
        };
        const lin = (v: number) => {
          const c = v / 255;
          return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
        };
        const lum = (c: { r: number; g: number; b: number }) => 0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b);
        const over = (t: { r: number; g: number; b: number; a: number }, b: { r: number; g: number; b: number }) => ({
          r: t.r * t.a + b.r * (1 - t.a),
          g: t.g * t.a + b.g * (1 - t.a),
          b: t.b * t.a + b.b * (1 - t.a),
        });
        const el = document.querySelector(sel) as HTMLElement;
        const layers: { r: number; g: number; b: number; a: number }[] = [];
        for (let p: HTMLElement | null = el; p; p = p.parentElement) {
          const c = parse(getComputedStyle(p).backgroundColor);
          if (c && c.a > 0) {
            layers.push(c);
            if (c.a >= 1) break;
          }
        }
        let bg = layers.length && layers[layers.length - 1].a >= 1 ? layers.pop()! : { r: 244, g: 243, b: 238, a: 1 };
        let cur: { r: number; g: number; b: number } = bg;
        while (layers.length) cur = over(layers.pop()!, cur);
        const fg = parse(getComputedStyle(el).color)!;
        const f = over(fg, cur);
        const [hi, lo] = [lum(f), lum(cur)].sort((x, y) => y - x);
        return { ratio: (hi + 0.05) / (lo + 0.05), color: getComputedStyle(el).color };
      }, selector);

      expect(r.ratio, `${label} 的文字色 ${r.color} 对比度只有 ${r.ratio.toFixed(2)}:1`).toBeGreaterThanOrEqual(need);
    });
  }
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
