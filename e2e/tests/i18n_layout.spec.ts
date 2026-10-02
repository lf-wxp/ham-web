import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { expect, test } from "./fixtures";

// 路由表以 main_content.rs 为准，与 smoke.spec.ts 保持一致
const source = readFileSync(join(__dirname, "../../crates/app/src/app/main_content.rs"), "utf8");
const routes = [...source.matchAll(/path!\("([^"]+)"\)/g)]
  .map((m) => m[1])
  .filter((p) => !p.includes(":") && !p.includes("*"));

const LANGS = [
  { code: "zh", htmlLang: "zh-CN", name: "中文" },
  { code: "en", htmlLang: "en", name: "English" },
  { code: "es", htmlLang: "es", name: "Español" },
] as const;

const OUT_DIR = join(__dirname, "../test-results/i18n-layout");
const limit = Number(process.env.I18N_MAX_FINDINGS ?? 40);

/**
 * 页面内布局审计：
 * 1. 文档横向滚动 —— 任何语言都不该出现（硬失败）
 * 2. 文本被裁掉 —— 自身内容宽于可视宽，且不是刻意省略（ellipsis / line-clamp）
 * 3. 元素越过视口右边界 —— 排除地图 / 横向可滚容器
 * SVG 内部元素由 viewBox 决定坐标，不遵循 HTML 布局，仅统计不判定。
 */
const AUDIT = () => {
  const doc = document.documentElement;
  const vw = window.innerWidth;
  const hScroll = Math.max(0, doc.scrollWidth - doc.clientWidth);
  const clipped: { sel: string; detail: string }[] = [];
  const overRight: { sel: string; detail: string }[] = [];
  const svgOver: string[] = [];

  const describe = (el: Element) => {
    const id = el.id ? `#${el.id}` : "";
    const cls = (el.getAttribute("class") || "").split(/\s+/).filter(Boolean).slice(0, 4).join(".");
    const text = (el.textContent || "").trim().replace(/\s+/g, " ").slice(0, 70);
    return `${el.tagName.toLowerCase()}${id}${cls ? "." + cls : ""} :: ${text}`;
  };

  for (const el of Array.from(document.querySelectorAll("main *"))) {
    if ((el as SVGElement).ownerSVGElement) {
      const r = el.getBoundingClientRect();
      if (r.width > 0 && r.right > vw + 1) svgOver.push(describe(el));
      continue;
    }
    const cs = getComputedStyle(el);
    if (cs.display === "none" || cs.visibility === "hidden") continue;
    const rect = el.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) continue;
    if (!(el.textContent || "").trim()) continue;
    // sr-only / 1px 视觉隐藏元素（clip 技巧）不参与判定
    if (el.clientWidth <= 2 || el.clientHeight <= 2) continue;

    // 刻意截断（ellipsis / line-clamp）属设计意图，不计入
    const lineClamp = (cs as unknown as { webkitLineClamp?: string }).webkitLineClamp;
    const intentional = cs.textOverflow === "ellipsis" || (!!lineClamp && lineClamp !== "none");

    if (!intentional && el.clientWidth > 0 && el.scrollWidth > el.clientWidth + 2) {
      clipped.push({
        sel: describe(el),
        detail: `scrollW=${el.scrollWidth} clientW=${el.clientWidth}`,
      });
    }

    if (
      rect.right > vw + 1 &&
      cs.position !== "fixed" &&
      cs.position !== "sticky" &&
      cs.overflowX !== "auto" &&
      cs.overflowX !== "scroll"
    ) {
      overRight.push({
        sel: describe(el),
        detail: `right=${Math.round(rect.right)} viewport=${vw}`,
      });
    }
  }
  // 未取整的浮点数（格式化丢失的典型特征）：文本里出现 6 位以上小数
  const longDecimals: string[] = [];
  for (const el of Array.from(document.querySelectorAll("main *"))) {
    if (el.children.length > 0) continue;
    const m = (el.textContent || "").trim().match(/\d+\.\d{6,}/);
    if (m) longDecimals.push(`${describe(el)} → ${m[0]}`);
  }

  return { hScroll, clipped, overRight, svgOver, longDecimals };
};

for (const lang of LANGS) {
  test.describe(`双语布局回归（${lang.name}）`, () => {
    test.setTimeout(900_000);

    test(`全部 ${routes.length} 个路由：无横向溢出 / 文本裁剪 / 越界`, async ({ page }) => {
      await page.addInitScript((code) => localStorage.setItem("locale", code), lang.code);

      const findings: { sel: string; detail: string }[] = [];
      const svgNotes: string[] = [];

      for (const route of routes) {
        await page.goto(route, { waitUntil: "domcontentloaded" });
        try {
          // Leptos 客户端渲染：等主内容挂载
          await page.waitForSelector("main", { timeout: 15_000 });
          await page.waitForTimeout(120);
        } catch {
          findings.push({ sel: `${route} :: main 未挂载`, detail: "页面渲染超时" });
          continue;
        }

        const attr = await page.getAttribute("html", "lang");
        if (attr !== lang.htmlLang) {
          findings.push({
            sel: `${route} :: <html lang>`,
            detail: `实为 ${attr}，期望 ${lang.htmlLang}`,
          });
        }

        const r = await page.evaluate(AUDIT);
        if (r.hScroll > 1) {
          findings.push({ sel: `${route} :: 文档横向滚动`, detail: `溢出 ${r.hScroll}px` });
        }
        for (const c of r.clipped) {
          findings.push({ sel: `${route} :: 文本裁剪`, detail: `${c.sel}（${c.detail}）` });
        }
        for (const o of r.overRight) {
          findings.push({ sel: `${route} :: 越过右边界`, detail: `${o.sel}（${o.detail}）` });
        }
        for (const d of r.longDecimals) {
          findings.push({ sel: `${route} :: 浮点未取整`, detail: d });
        }
        for (const s of r.svgOver) svgNotes.push(`${route} :: ${s}`);
      }

      mkdirSync(OUT_DIR, { recursive: true });
      writeFileSync(
        join(OUT_DIR, `${lang.code}.json`),
        JSON.stringify({ lang: lang.code, findings, svgNotes }, null, 2),
      );

      const shown = findings.slice(0, limit);
      const report =
        shown.map((f) => `  - ${f.sel}\n      ${f.detail}`).join("\n") +
        (findings.length > shown.length ? `\n  … 另有 ${findings.length - shown.length} 条` : "");

      expect(
        findings,
        `共 ${findings.length} 处布局问题（SVG 内部另有 ${svgNotes.length} 条，见 test-results/i18n-layout/${lang.code}.json）：\n${report}`,
      ).toEqual([]);
    });
  });
}
