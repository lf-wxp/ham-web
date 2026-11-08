// 打开态表面的隔离扫描：弹层 / 抽屉 / 气泡这类「按了才出现」的表面，默认路径扫不到它们。
//
//   node e2e/audit/surface_sweep.mjs modern    # 现代风：不许有 `pxl-*`
//   node e2e/audit/surface_sweep.mjs pixel     # 像素风：不许有 `md-*`
//
// 每个表面都带一个 `ready` 选择器：操作之后必须真的等到它，否则算失败 —— 键盘事件早于挂载就是
// 空操作，扫到的是底层页面，会把「没打开」误报成「没串风格」。前置与退出码同 `dual_sweep.mjs`。

import { chromium } from "@playwright/test";
import { baseUrl, parseMode, seedScript, waitMounted } from "./common.mjs";

const mode = parseMode(process.argv);
const isModern = mode === "modern";

const MODAL = "[data-modal-root]";

const surfaces = [
  {
    name: "设置弹层",
    url: "/",
    ready: MODAL,
    prep: (p) => p.locator('button[aria-label="通用设置"]').first().click(),
  },
  {
    name: "调谐台 ⌘K",
    url: "/",
    ready: MODAL,
    // 调谐台是现代风专有的表面：像素风里没有它，"必须出现"的规则会把它判成失败。
    only: "modern",
    prep: (p) => p.keyboard.press("Control+k"),
  },
  {
    name: "全站搜索 /",
    url: "/",
    ready: MODAL,
    prep: (p) => p.keyboard.press("/"),
  },
  {
    name: "下拉展开",
    url: "/log",
    ready: '[role="listbox"], [role="option"]',
    prep: (p) => p.locator('main [role="combobox"]').first().click(),
  },
  {
    name: "悬停气泡",
    url: "/morse",
    ready: ".md-tip-popup, .pxl-tip-popup",
    prep: (p) => p.locator(".md-morse-card, .pxl-window").first().hover(),
  },
  {
    name: "快捷键帮助",
    url: "/practice",
    ready: MODAL,
    prep: (p) => p.keyboard.press("?"),
  },
  {
    name: "确认弹层",
    url: "/bookmarks",
    ready: MODAL,
    prep: async (p) => {
      await p.getByRole("textbox").first().fill("临时组");
      await p.getByRole("button", { name: "新建分组" }).first().click();
      await p.getByRole("button", { name: "删除该分组" }).first().click();
    },
  },
  {
    // 现代风是底部 dock 的「更多」；像素风是页面内的分类抽屉（MobileNavDrawer）。
    name: "移动端菜单",
    url: isModern ? "/" : "/tools",
    viewport: { width: 390, height: 844 },
    ready: MODAL,
    prep: (p) => {
      const sel = isModern
        ? '.md-dock button:has-text("更多")'
        : 'button[aria-label*="目录"], button[aria-label*="分类"]';
      return p.locator(sel).first().click();
    },
  },
  {
    name: "答题卡抽屉",
    url: "/exam",
    ready: MODAL,
    prep: (p) => p.getByRole("button", { name: /答题卡/ }).first().click(),
  },
];

const browser = await chromium.launch();
const bad = [];
console.log(`mode ${mode} · ${baseUrl}`);

for (const s of surfaces) {
  if (s.only && s.only !== mode) continue;
  const ctx = await browser.newContext({ viewport: s.viewport ?? { width: 1280, height: 950 } });
  const page = await ctx.newPage();
  await page.addInitScript(seedScript, [mode]);
  try {
    await page.goto(baseUrl + s.url, { waitUntil: "domcontentloaded" });
    await waitMounted(page);
    await s.prep(page);
    // 必须等到表面真的出现；等不到就抛，落进下面的 catch 记为失败。
    await page.waitForSelector(s.ready, { state: "attached", timeout: 8_000 });
    const info = await page.evaluate(([modern]) => {
      const foreign = new Set();
      for (const el of document.querySelectorAll("[class]")) {
        for (const c of el.classList) {
          if (modern && c.startsWith("pxl-")) foreign.add(c);
          if (!modern && c.startsWith("md-")) foreign.add(c);
        }
      }
      return [...foreign];
    }, [isModern]);
    const foreign = info;
    console.log(`${s.name.padEnd(14)} ${foreign.length ? "✗ " + JSON.stringify(foreign) : "✓"}`);
    if (foreign.length) bad.push({ name: s.name, foreign });
  } catch (e) {
    // 没打开 / 没找到触发器：这是失败，不是「跳过」。
    console.log(`${s.name.padEnd(14)} ✗ 表面没出现：${String(e).split("\n")[0].slice(0, 80)}`);
    bad.push({ name: s.name, error: String(e).slice(0, 80) });
  }
  await ctx.close();
}
await browser.close();

console.log(`\n${mode} 有问题的表面: ${bad.length}`);
if (bad.length) process.exitCode = 1;
