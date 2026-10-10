// 诊断脚本：列出指定页面在亮 / 暗主题下的 axe 对比度违规，附前景 / 背景色与实测比值。
// 用法：E2E_BASE_URL=http://127.0.0.1:3030 node scripts/contrast_report.mjs / /browse /print
import AxeBuilder from "@axe-core/playwright";
import { chromium } from "@playwright/test";

const base = process.env.E2E_BASE_URL ?? "http://127.0.0.1:3030";
const routes = process.argv.slice(2);
const browser = await chromium.launch({ channel: process.env.E2E_CHANNEL });

for (const scheme of ["light", "dark"]) {
  const ctx = await browser.newContext({ colorScheme: scheme, locale: "zh-CN", serviceWorkers: "block" });
  for (const route of routes) {
    const page = await ctx.newPage();
    await page.goto(base + route);
    await page.waitForSelector("main h1", { state: "attached" });
    await page.waitForTimeout(1200);
    const res = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "best-practice"])
      .exclude(".leaflet-container")
      .analyze();
    for (const v of res.violations.filter((x) => x.id === "color-contrast")) {
      for (const n of v.nodes) {
        const d = n.any[0]?.data ?? {};
        console.log(
          `[${scheme}] ${route}  ${d.contrastRatio} (需 ${d.expectedContrastRatio})  fg=${d.fgColor} bg=${d.bgColor}  ${d.fontSize}  ${n.target.join(" ").slice(0, 110)}`,
        );
      }
    }
    await page.close();
  }
  await ctx.close();
}
await browser.close();
