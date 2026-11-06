import { readFileSync } from "node:fs";
import { join } from "node:path";
import AxeBuilder from "@axe-core/playwright";
import {
  AXE_DISABLED_RULES,
  expect,
  ignoredLoadError,
  settleFrames,
  test,
  waitSettled,
} from "./fixtures";

// 路由表以 main_content.rs 为准，新增页面自动纳入冒烟测试
const source = readFileSync(join(__dirname, "../../crates/app/src/app/main_content.rs"), "utf8");
const routes = [...source.matchAll(/path!\("([^"]+)"\)/g)]
  .map((m) => m[1])
  .filter((p) => !p.includes(":") && !p.includes("*"));

// 实时数据依赖 /api 代理与第三方服务，测试环境下可能不可用，属于预期降级
const ignoredRequest = (url: string, base: string) =>
  !url.startsWith(base) || new URL(url).pathname.startsWith("/api/");

// 第三方脚本（如 AdSense）在无外网的测试环境里会抛自己的 unhandled rejection，
// 堆栈里全是外域地址，与本站代码无关，按与 console/网络一致的规则忽略。
const fromThirdParty = (stack: string, base: string) => {
  const urls = stack.match(/https?:\/\/[^\s)]+/g) ?? [];
  return urls.length > 0 && urls.every((u) => !u.startsWith(base));
};

// Leptos 对「在跟踪上下文之外读信号」发的告警：组件体 / 被立即调用的闭包读了订阅版信号，
// 那块内容不会随界面风格重渲染（规则见 docs/ui/decisions.md §7）。
// 它只在 dev 构建（`debug_assertions`）里发声：`cargo make e2e` 的 release 产物下这条检查空转，
// 守的是「trunk serve 起 dev 再跑冒烟」这个场景，见 docs/engineering/testing.md。
const NON_REACTIVE_READ = "outside a reactive tracking context";

for (const colorScheme of ["light", "dark"] as const) {
test.describe(`全站冒烟（${colorScheme}）`, () => {
  test.use({ colorScheme });

  for (const route of routes) {
    test(`${route} 可正常渲染且无障碍检查通过`, async ({ page, baseURL }) => {
      const base = baseURL ?? "";
      const problems: string[] = [];
      page.on("pageerror", (e) => {
        const stack = e.stack ?? "";
        if (fromThirdParty(stack, base)) return;
        problems.push(`pageerror: ${e.message}`);
      });
      page.on("console", (msg) => {
        const text = msg.text();
        if (msg.type() === "warning" && text.includes(NON_REACTIVE_READ)) {
          // 只留第一行：Leptos 后面跟着一大段修复建议，进断言输出会淹没其它问题。
          problems.push(`console warning: ${text.split("\n")[0]}`);
          return;
        }
        if (msg.type() !== "error") return;
        const url = msg.location().url;
        if (ignoredLoadError(text, url, base)) return;
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
      await waitSettled(page);
      await settleFrames(page);

      const axe = await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "best-practice"])
        .exclude(".leaflet-container")
        .disableRules(AXE_DISABLED_RULES)
        .analyze();
      const violations = axe.violations.map(
        (v) => `${v.id} (${v.impact}): ${v.nodes.slice(0, 3).map((n) => n.target.join(" ")).join(" | ")}`,
      );

      expect(problems, "运行时错误 / 告警").toEqual([]);
      expect(violations, "axe 违规").toEqual([]);
    });
  }

});
}

test("未知路径显示 404 页", async ({ page }) => {
  await page.goto("/definitely-not-a-page");
  await expect(page.getByRole("heading", { level: 1, name: "未找到页面" })).toBeVisible();
});
