import { readFileSync } from "node:fs";
import { join } from "node:path";
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "./fixtures";

// 路由表以 main_content.rs 为准，新增页面自动纳入冒烟测试
const source = readFileSync(join(__dirname, "../../crates/app/src/app/main_content.rs"), "utf8");
const routes = [...source.matchAll(/path!\("([^"]+)"\)/g)]
  .map((m) => m[1])
  .filter((p) => !p.includes(":") && !p.includes("*"));

// 实时数据依赖 /api 代理与第三方服务，测试环境下可能不可用，属于预期降级
const ignoredRequest = (url: string, base: string) =>
  !url.startsWith(base) || new URL(url).pathname.startsWith("/api/");

for (const colorScheme of ["light", "dark"] as const) {
test.describe(`全站冒烟（${colorScheme}）`, () => {
  test.use({ colorScheme });

  for (const route of routes) {
    test(`${route} 可正常渲染且无障碍检查通过`, async ({ page, baseURL }) => {
      const base = baseURL ?? "";
      const problems: string[] = [];
      page.on("pageerror", (e) => problems.push(`pageerror: ${e.message}`));
      page.on("console", (msg) => {
        if (msg.type() !== "error") return;
        const url = msg.location().url;
        if (msg.text().startsWith("Failed to load resource") && ignoredRequest(url, base)) return;
        // 浏览器对外部站点的 report-only CSP 报告（如 Google frame-ancestors），非本页错误，忽略
        if (msg.text().includes("report-only Content Security Policy")) return;
        problems.push(`console: ${msg.text()}`);
      });
      page.on("response", (res) => {
        if (res.status() >= 400 && !ignoredRequest(res.url(), base)) {
          problems.push(`HTTP ${res.status()}: ${res.url()}`);
        }
      });

      await page.goto(route);
      await expect(page.locator("main h1").first()).toBeAttached();
      await expect(page.getByText("未找到页面")).toHaveCount(0);
      await expect(page.locator("#fatal")).toHaveCount(0);
      await page.waitForTimeout(800);

      const axe = await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "best-practice"])
        .exclude(".leaflet-container")
        .analyze();
      const violations = axe.violations.map(
        (v) => `${v.id} (${v.impact}): ${v.nodes.slice(0, 3).map((n) => n.target.join(" ")).join(" | ")}`,
      );

      expect(problems, "运行时错误").toEqual([]);
      expect(violations, "axe 违规").toEqual([]);
    });
  }

});
}

test("未知路径显示 404 页", async ({ page }) => {
  await page.goto("/definitely-not-a-page");
  await expect(page.getByRole("heading", { level: 1, name: "未找到页面" })).toBeVisible();
});
