import { expect, test } from "./fixtures";
import type { Page } from "@playwright/test";

// 双风格运行时隔离回归。
//
// 静态门禁（`check-structure` 的反向隔离）只看源码：共享组件里硬编码某一种风格的类名，
// 或「按皮肤分叉但漏了一处」，只有真正渲染出来才暴露 —— 已经踩过三次：
// 术语卡的 `md-mono`、莫尔斯字符卡的 `md-morse-play`、全站搜索的 `md-search-text`。
//
// 判据：
// - 现代风：整页（含 Portal 出来的表面）不允许出现 `pxl-*`；
// - 像素风：整页不允许出现 `md-*`。
//
// 两条「扫不到东西也会通过」的坑都在这里堵上：
// 1. 扫描前先等应用挂载（`#route-content[data-sig]`），并做**正向对照** —— 现代风下必须真的有
//    `.md-root`、像素风下必须真的有 `.pxl-*`；否则 wasm 没起来时 DOM 里只有启动页，扫出来必然是空；
// 2. 打开态表面每一个都带 `ready` 选择器，操作之后必须等到它出现 —— 键盘事件早于挂载就是空操作，
//    扫到的是底层页面。
//
// 全量审计（180 条路由 × 两套皮肤）用 `e2e/audit/dual_sweep.mjs`，
// 打开态表面用 `e2e/audit/surface_sweep.mjs`；这里保留一份有代表性的快速回归。

type Mode = "modern" | "pixel";

async function foreignClasses(page: Page, mode: Mode) {
  return page.evaluate((m) => {
    const foreign = new Set<string>();
    for (const el of document.querySelectorAll("[class]")) {
      for (const c of el.classList) {
        if (m === "modern" && c.startsWith("pxl-")) foreign.add(c);
        if (m === "pixel" && c.startsWith("md-")) foreign.add(c);
      }
    }
    return [...foreign];
  }, mode);
}

async function seed(page: Page, mode: Mode) {
  await page.addInitScript(([m]) => {
    if (window !== window.top) return;
    localStorage.setItem("ui:uiStyle", m);
    localStorage.setItem("theme", m === "modern" ? "dark" : "light");
    localStorage.setItem("ui:shortcutsHelpSeen:practice", "1");
    localStorage.setItem("ui:shortcutsHelpSeen:exam", "1");
  }, [mode]);
}

/** 打开页面并等应用挂载 + 正向对照：确认这一页真的按目标风格渲染了。 */
async function openMounted(page: Page, route: string, mode: Mode) {
  await page.goto(route);
  await expect(page.locator("#route-content[data-sig]")).toBeAttached();
  if (mode === "modern") {
    await expect(page.locator(".md-root").first()).toBeAttached();
  } else {
    await expect(page.locator('[class*="pxl-"]').first()).toBeAttached();
  }
}

const SAMPLE = ["/", "/log", "/tools", "/glossary", "/morse", "/bands", "/practice", "/exam"];

const MODAL = "[data-modal-root]";
const SURFACES: {
  name: string;
  url: string;
  ready: string;
  open: (page: Page) => Promise<void>;
  /** 只在这些风格里存在（调谐台是现代风专有）。 */
  modes: Mode[];
}[] = [
  { name: "搜索弹层", url: "/", ready: MODAL, modes: ["modern", "pixel"], open: (p) => p.keyboard.press("/") },
  { name: "调谐台", url: "/", ready: MODAL, modes: ["modern"], open: (p) => p.keyboard.press("Control+k") },
  {
    name: "设置",
    url: "/",
    ready: MODAL,
    modes: ["modern", "pixel"],
    open: (p) => p.locator('button[aria-label="通用设置"]').first().click(),
  },
  { name: "快捷键帮助", url: "/practice", ready: MODAL, modes: ["modern", "pixel"], open: (p) => p.keyboard.press("?") },
  {
    name: "下拉",
    url: "/log",
    ready: '[role="listbox"], [role="option"]',
    modes: ["modern", "pixel"],
    open: (p) => p.locator('main [role="combobox"]').first().click(),
  },
];

for (const mode of ["modern", "pixel"] as const) {
  const foreignName = mode === "modern" ? "pxl-*" : "md-*";

  test(`${mode === "modern" ? "现代风" : "像素风"}：页面与打开态都不渲染 ${foreignName}`, async ({ page }) => {
    await seed(page, mode);
    for (const route of SAMPLE) {
      await openMounted(page, route, mode);
      expect({ route, foreign: await foreignClasses(page, mode) }).toEqual({ route, foreign: [] });
    }

    for (const s of SURFACES.filter((x) => x.modes.includes(mode))) {
      await openMounted(page, s.url, mode);
      await s.open(page);
      // 必须等到表面真的出现，再扫：否则扫到的只是底层页面。
      await expect(page.locator(s.ready).first(), `${s.name} 没有打开`).toBeAttached();
      expect({ surface: s.name, foreign: await foreignClasses(page, mode) }).toEqual({
        surface: s.name,
        foreign: [],
      });
      await page.keyboard.press("Escape");
    }
  });
}
