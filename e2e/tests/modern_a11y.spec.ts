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

// `/morse` 与 `/formulas` 是 C1 刚真迁移的两页（内嵌小块 / 公式读数），一并纳入深色档扫描。
const routes = ["/", "/practice", "/tools", "/log", "/glossary", "/map", "/morse", "/formulas"];

test("现代风 · 深色：八个主页面零 a11y 违规", async ({ page }) => {
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

// 桥接页抽样（台账 A4）：上面那六条覆盖的是真迁移的页面，而全站 90% 的页面走的是
// `PageHeader` + 工具类桥接，从来没被 axe 扫过。这里挑四个形态不同的桥接页：
// 表单密集（/backup）、图表（/stats）、Canvas + 表单（/nec）、多卡 + 外部数据（/solar）。
test("现代风 · 桥接页抽样：四个非主页面零违规（深色）", async ({ page }) => {
  await seed(page, "dark");
  for (const route of ["/backup", "/stats", "/nec", "/solar"]) {
    await page.goto(route);
    await settle(page);
    await expectClean(page);
  }
});

test("打开态表面：调谐台 / 确认弹层 / 移动端菜单零违规", async ({ page }) => {
  await seed(page, "dark");
  await page.goto("/");
  // 快捷键挂在应用挂载之后：`goto` 在 `load` 就返回，此时 wasm 可能还没渲染出外壳，
  // 直接按键会打在空页面上（表现为「调谐台打不开」，与实现无关）。先等顶栏。
  await page.locator("nav.md-nav").waitFor();
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
  await page.locator('button[aria-label="通用设置"]').first().click();
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

// 语义文字色必须走现代 token，而不是像素色（台账 C7）。
//
// 这条曾经「写了但没生效」：`--color-hp-text` 等四个变量在 `legacy-utilities.css` 里映射过，
// 但像素风的色彩 token 声明在 Tailwind 的 `@theme inline` 里 —— 生成工具类时值已内联，
// 运行时不再读变量。于是现代风一直拿像素风的 `#8f2a3f` / `#f7a98f` 当状态色，
// 而对比度只按像素风的底面量过。axe 也扫不出来（它只是「另一个也是合格的颜色」）。
test.describe("语义文字色走现代 token", () => {
  const cases = [
    ["text-hp-text", "--md-danger-ink"],
    // 文字档走 `-ink`（AA 的那一档）：深色下 `--md-green` 是给图形 / 实心面用的亮绿。
    ["text-win-text", "--md-green-ink"],
    ["text-xp-text", "--md-blue-ink"],
    ["text-gold-text", "--md-gold-ink"],
  ] as const;

  for (const theme of ["light", "dark"] as const) {
    test(`${theme === "light" ? "浅色" : "深色"}：四个语义文字类都等于对应的 --md-* token`, async ({ page }) => {
      await seed(page, theme);
      await page.goto("/");
      await expect(page.locator(".md-root").first()).toBeAttached();

      const got = await page.evaluate((pairs) => {
        // `--md-*` 是十六进制；`color` 的计算值是 `rgb()` / `color(srgb …)`，统一成 RGB 数组比较。
        const parse = (c: string): number[] | null => {
          const srgb = c.match(/color\(srgb\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)/);
          if (srgb) return [Number(srgb[1]) * 255, Number(srgb[2]) * 255, Number(srgb[3]) * 255];
          const rgb = c.match(/rgba?\(([^)]+)\)/);
          if (!rgb) return null;
          const p = rgb[1].split(/[ ,/]+/).filter(Boolean).map(Number);
          return [p[0], p[1], p[2]];
        };
        const toRgb = (value: string) => {
          const canvas = document.createElement("canvas").getContext("2d")!;
          canvas.fillStyle = value;
          canvas.fillRect(0, 0, 1, 1);
          return Array.from(canvas.getImageData(0, 0, 1, 1).data).slice(0, 3);
        };
        const cs = getComputedStyle(document.documentElement);
        // 探针必须挂在 `.md-root` 里：现代风的桥接覆盖有 `.md-root` 作用域（层外规则），
        // 挂在 body 上会绕过它们 —— 曾经因此测到一个页面上根本不出现的值（假绿）。
        const host = document.querySelector(".md-root") ?? document.body;
        return pairs.map(([cls, token]) => {
          const el = document.createElement("span");
          el.className = cls;
          el.textContent = "x";
          host.appendChild(el);
          const actual = parse(getComputedStyle(el).color);
          el.remove();
          return { cls, token, actual, want: toRgb(cs.getPropertyValue(token).trim()) };
        });
      }, cases.map(([c, t]) => [c, t] as const));

      const bad = got.filter((g) => !g.actual || g.actual.some((v, i) => Math.abs(v - g.want[i]) > 3));
      expect(bad.map((g) => `${g.cls} = ${g.actual}，应为 ${g.token} = ${g.want}`), "语义文字色仍是像素色").toEqual([]);
    });
  }
});

// 焦点环的对比度（WCAG 2.2 SC 1.4.11 非文本对比，≥3:1）。
//
// axe **扫不到**它：焦点环是 `outline`，只有键盘聚焦时才出现，且不是文字。
// 浅色档曾经是 50% 透明度的蓝，合成到 `--md-bg` 上只有 2.33:1 —— 键盘用户看不到自己在哪。
test.describe("焦点环的对比度", () => {
  for (const theme of ["light", "dark"] as const) {
    test(`${theme === "light" ? "浅色" : "深色"}：焦点环对四个底面都达到 3:1`, async ({ page }) => {
      await seed(page, theme);
      await page.goto("/");
      await expect(page.locator(".md-root").first()).toBeAttached();

      const result = await page.evaluate(() => {
        // `--md-ring` 是 `color-mix()`，自定义属性取到的是原文；换成 `outline-color` 才拿得到计算值。
        const probe = document.createElement("span");
        probe.style.outlineColor = "var(--md-ring)";
        document.body.appendChild(probe);
        const raw = getComputedStyle(probe).outlineColor;
        probe.remove();

        const color = (() => {
          const srgb = raw.match(/color\(srgb\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)(?:\s*\/\s*([\d.]+))?\)/);
          if (srgb) {
            return {
              r: Number(srgb[1]) * 255,
              g: Number(srgb[2]) * 255,
              b: Number(srgb[3]) * 255,
              a: srgb[4] === undefined ? 1 : Number(srgb[4]),
            };
          }
          const rgb = raw.match(/rgba?\(([^)]+)\)/);
          if (!rgb) return null;
          const p = rgb[1].split(/[ ,/]+/).filter(Boolean).map(Number);
          return { r: p[0], g: p[1], b: p[2], a: p.length > 3 ? p[3] : 1 };
        })();

        const lin = (v: number) => {
          const c = v / 255;
          return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
        };
        const lum = (c: { r: number; g: number; b: number }) => 0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b);
        const hex = (h: string) => {
          let s = h.replace("#", "");
          if (s.length === 3) s = [...s].map((c) => c + c).join("");
          return { r: parseInt(s.slice(0, 2), 16), g: parseInt(s.slice(2, 4), 16), b: parseInt(s.slice(4, 6), 16) };
        };

        const cs = getComputedStyle(document.documentElement);
        const rows = ["--md-bg", "--md-surface", "--md-surface-2", "--md-surface-3"].map((token) => {
          const bg = hex(cs.getPropertyValue(token).trim());
          // 半透明环要按合成后的颜色比，不能比 token 本身。
          const mixed = {
            r: color!.r * color!.a + bg.r * (1 - color!.a),
            g: color!.g * color!.a + bg.g * (1 - color!.a),
            b: color!.b * color!.a + bg.b * (1 - color!.a),
          };
          const [hi, lo] = [lum(mixed), lum(bg)].sort((x, y) => y - x);
          return { token, ratio: (hi + 0.05) / (lo + 0.05), raw };
        });
        return rows;
      });

      const bad = result.filter((r) => !Number.isFinite(r.ratio) || r.ratio < 3);
      expect(bad.map((r) => `${r.token} = ${r.ratio.toFixed(2)}:1（${r.raw}）`), "焦点环对比度不足").toEqual([]);
    });
  }
});
