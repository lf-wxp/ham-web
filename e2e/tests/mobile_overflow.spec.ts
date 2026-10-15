import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { expect, test, waitSettled } from "./fixtures";

// 路由表以 main_content.rs 为准，与 smoke.spec.ts / i18n_layout.spec.ts 保持同一来源
const source = readFileSync(join(__dirname, "../../crates/app/src/app/main_content.rs"), "utf8");
const allRoutes = [...source.matchAll(/path!\("([^"]+)"\)/g)]
  .map((m) => m[1])
  .filter((p) => !p.includes(":") && !p.includes("*"));

// 逐页修复时用：MOBILE_ONLY=/tools,/log 只扫子串命中的路由（默认全站）
const only = (process.env.MOBILE_ONLY ?? "")
  .split(",")
  .map((s) => s.trim())
  .filter(Boolean);
const routes = only.length ? allRoutes.filter((r) => only.some((o) => r.includes(o))) : allRoutes;

const LANGS = [
  { code: "zh", htmlLang: "zh-CN", name: "中文" },
  { code: "en", htmlLang: "en", name: "English" },
  { code: "es", htmlLang: "es", name: "Español" },
] as const;

/** 320 = 最窄的主流机型（iPhone SE 一代 / 老安卓），390 = 现代主流（iPhone 14/15）。 */
const WIDTHS = [320, 390] as const;

const OUT_DIR = join(__dirname, "../test-results/mobile-overflow");
const limit = Number(process.env.MOBILE_MAX_FINDINGS ?? 60);

/**
 * 手机宽度横向溢出审计。
 *
 * 判定只看文档级 `scrollWidth - innerWidth`：手机是覆盖式滚动条（`clientWidth == innerWidth`），
 * 这就是「整页能横向滚」在真机上的表现，也正是此前 `/log`、`/tools` 那 4 处问题的症状。
 *
 * 元素越界清单只用于定位根因，因此反过来要**剔除**落在 `overflow-x: auto/scroll` 容器内的元素：
 * 一张 `min-w-[680px]` 的表格正常就该在自己的容器里横向滚，算进去只会刷满假阳性。
 */
const AUDIT = () => {
  const doc = document.documentElement;
  const vw = window.innerWidth;
  const overflow = doc.scrollWidth - vw;
  if (overflow <= 1) return { overflow, offenders: [] as string[] };

  const scrollableX = (el: Element) => {
    const cs = getComputedStyle(el);
    return cs.overflowX === "auto" || cs.overflowX === "scroll";
  };
  const inScrollableX = (el: Element | null) => {
    for (let p = el; p; p = p.parentElement) {
      if (scrollableX(p)) return true;
    }
    return false;
  };
  const describe = (el: Element) => {
    const id = el.id ? `#${el.id}` : "";
    const cls = (el.getAttribute("class") || "").split(/\s+/).filter(Boolean).slice(0, 4).join(".");
    const text = (el.textContent || "").trim().replace(/\s+/g, " ").slice(0, 60);
    return `${el.tagName.toLowerCase()}${id}${cls ? "." + cls : ""} :: ${text}`;
  };

  const hit: { depth: number; text: string }[] = [];
  for (const el of Array.from(document.querySelectorAll("body *"))) {
    if ((el as SVGElement).ownerSVGElement) continue; // SVG 内部坐标由 viewBox 决定
    const cs = getComputedStyle(el);
    if (cs.display === "none" || cs.visibility === "hidden") continue;
    // 抽屉 / 吸顶导航这类脱离文档流的元素，越界不代表页面能横向滚
    if (cs.position === "fixed" || cs.position === "sticky") continue;
    const rect = el.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) continue;
    if (rect.right <= vw + 1) continue;
    if (inScrollableX(el.parentElement)) continue;
    let depth = 0;
    for (let p = el.parentElement; p; p = p.parentElement) depth += 1;
    hit.push({ depth, text: `${describe(el)} right=${Math.round(rect.right)}` });
  }
  // 越深的元素离「不换行的内容」越近，排前面最省事
  hit.sort((a, b) => b.depth - a.depth);
  return { overflow, offenders: hit.slice(0, 12).map((h) => h.text) };
};

for (const lang of LANGS) {
  test.describe(`手机宽度横向溢出（${lang.name}）`, () => {
    test.setTimeout(900_000);

    test(`全部 ${routes.length} 个路由：320 / 390px 无整页横向滚动`, async ({ page }) => {
      await page.addInitScript((code) => localStorage.setItem("locale", code), lang.code);

      const findings: { sel: string; detail: string }[] = [];
      // 每扫完一条路由就落盘：整批要跑十几分钟，中途超时也不能把清单丢了
      const save = () => {
        mkdirSync(OUT_DIR, { recursive: true });
        writeFileSync(
          join(OUT_DIR, `${lang.code}.json`),
          JSON.stringify({ lang: lang.code, widths: WIDTHS, findings }, null, 2),
        );
      };

      for (const route of routes) {
        await page.setViewportSize({ width: WIDTHS[0], height: 900 });
        await page.goto(route, { waitUntil: "domcontentloaded" });
        try {
          // Leptos 客户端渲染：等主内容挂载，再等异步内容到位（骨架 / spinner 消失）
          await page.waitForSelector("main", { timeout: 15_000 });
          await waitSettled(page);
        } catch {
          findings.push({ sel: `${route} :: main 未挂载`, detail: "页面渲染超时" });
          save();
          continue;
        }

        // 语言没生效的话，这三条用例量的是同一份中文排版 —— 扫描本身就失去意义，所以也当问题报
        const attr = await page.getAttribute("html", "lang");
        if (attr !== lang.htmlLang) {
          findings.push({
            sel: `${route} :: <html lang>`,
            detail: `实为 ${attr}，期望 ${lang.htmlLang}`,
          });
        }

        for (const width of WIDTHS) {
          await page.setViewportSize({ width, height: 900 });
          // 让重排落地（容器宽度变了，画布 / 响应式组件要跟一轮）再量
          await page.evaluate(
            () =>
              new Promise<void>((r) =>
                requestAnimationFrame(() => requestAnimationFrame(() => r())),
              ),
          );
          const r = await page.evaluate(AUDIT);
          if (r.overflow > 1) {
            findings.push({
              sel: `${route} :: ${width}px 横向溢出 ${r.overflow}px`,
              detail: r.offenders.length ? r.offenders.join("\n      ") : "（未定位到越界元素）",
            });
          }
        }
        save();
      }

      const shown = findings.slice(0, limit);
      const report =
        shown.map((f) => `  - ${f.sel}\n      ${f.detail}`).join("\n") +
        (findings.length > shown.length ? `\n  … 另有 ${findings.length - shown.length} 条` : "");

      expect(
        findings,
        `共 ${findings.length} 处横向溢出（明细见 test-results/mobile-overflow/${lang.code}.json）：\n${report}`,
      ).toEqual([]);
    });
  });
}
