import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { expect, test, waitSettled } from "./fixtures";
import { AUDIT } from "./layout_audit";

// 路由表以 main_content.rs 为准，与 smoke.spec.ts 保持一致
const source = readFileSync(join(__dirname, "../../crates/app/src/app/main_content.rs"), "utf8");
const allRoutes = [...source.matchAll(/path!\("([^"]+)"\)/g)]
  .map((m) => m[1])
  .filter((p) => !p.includes(":") && !p.includes("*"));

// 逐页修复时用：I18N_ONLY=/mfsk-decode,/thor-decode 只扫子串命中的路由（默认全站，与 MOBILE_ONLY 同一用法）
const only = (process.env.I18N_ONLY ?? "")
  .split(",")
  .map((s) => s.trim())
  .filter(Boolean);
const routes = only.length ? allRoutes.filter((r) => only.some((o) => r.includes(o))) : allRoutes;

/** 被测界面风格：`E2E_UI_STYLE=modern` 切到现代风，其余一律像素风。 */
const uiStyle = process.env.E2E_UI_STYLE === "modern" ? "modern" : "pixel";

const LANGS = [
  { code: "zh", htmlLang: "zh-CN", name: "中文" },
  { code: "en", htmlLang: "en", name: "English" },
  { code: "es", htmlLang: "es", name: "Español" },
] as const;

const OUT_DIR = join(__dirname, "../test-results/i18n-layout");
const limit = Number(process.env.I18N_MAX_FINDINGS ?? 40);


for (const lang of LANGS) {
  test.describe(`双语布局回归（${lang.name}）`, () => {
    test.setTimeout(900_000);

    test(`全部 ${routes.length} 个路由：无横向溢出 / 文本裁剪 / 越界`, async ({ page }) => {
      await page.addInitScript((code) => localStorage.setItem("locale", code), lang.code);
      // E2E_UI_STYLE=modern 把整套全站审计切到现代风（默认像素风）：dock、768 / 900 断点与大量
      // `md-*` 版式只在现代风下出现，默认只跑像素风就等于现代风没有被「全部路由 × 三语」覆盖。
      await page.addInitScript((style) => {
        if (window !== window.top) return;
        localStorage.setItem("ui:uiStyle", style);
        if (style === "modern") localStorage.setItem("theme", "dark");
      }, uiStyle);

      const findings: { sel: string; detail: string }[] = [];
      const svgNotes: string[] = [];

      for (const [index, route] of routes.entries()) {
        await page.goto(route, { waitUntil: "domcontentloaded" });
        try {
          // Leptos 客户端渲染：等主内容挂载。第一个路由要承担 wasm 的冷启动（三个 worker 同时起时尤其慢，
          // 曾在 `/map`、`/` 上各因 15 秒不够误报过「main 未挂载」），后面的路由是热的。
          await page.waitForSelector("main", { timeout: index === 0 ? 45_000 : 15_000 });
          // 再等异步内容到位（骨架 / spinner 消失）：否则量到的是「还没渲染完」的中间态，
          // 布局结论既可能漏报也可能误报。原先固定 sleep 120ms 同样是在赌。
          await waitSettled(page);
        } catch {
          findings.push({ sel: `${route} :: main 未挂载`, detail: "页面渲染超时" });
          continue;
        }

        // 同 `mobile_overflow`：语言包异步加载，`<html lang>` 比 `main` 挂载稍晚落定，
        // 直接读会偶发抓到模板里的 zh-CN。等一会儿再判（等不到才算问题）。
        const applied = await page
          .waitForFunction((l) => document.documentElement.lang === l, lang.htmlLang, {
            timeout: 5_000,
            polling: 50,
          })
          .then(() => true)
          .catch(() => false);
        if (!applied) {
          const attr = await page.getAttribute("html", "lang");
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
