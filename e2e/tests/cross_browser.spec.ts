import { expect, test } from "./fixtures";

/**
 * 跨内核冒烟（Firefox / WebKit）。
 *
 * 全量 e2e 依赖 Web Serial（CAT / 旋转器 / VNA / KISS）、假音频设备与 WebGL2，
 * 那两个内核上跑不了。这里只守住三件事：
 * 1. 核心页面能打开、不白屏；
 * 2. 没有未捕获的 JS 异常（内核差异最常见的表现就是这里）；
 * 3. 三维方向图在 WebGL2 不可用时走 SVG 回退。
 *
 * **状态：只在 chromium 上通过过（基线）。** 目标内核还没跑——本机缓存里的
 * firefox / webkit 与当前 Playwright 版本不匹配，需要先 `npx playwright install firefox webkit`。
 * 所以这两个 project 默认关闭，用 `E2E_CROSS=1 npx playwright test --project=firefox --project=webkit` 启用。
 * 不要把「用例存在」当成「已经验证」。
 */

test("跨内核：首页与主要页面可打开，且没有未捕获异常", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));

  for (const path of ["/", "/tools", "/signal-id", "/log"]) {
    await page.goto(path);
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  }

  expect(errors).toEqual([]);
});

test("跨内核：NEC 求解出结果，三维方向图有可用渲染器", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));

  await page.goto("/nec");
  await expect(page.getByText(/馈电点阻抗 .+ Ω/)).toBeVisible();

  const sphere = page.getByRole("img", { name: "三维方向图" });
  await expect(sphere).toBeVisible();
  // WebKit 常常拿不到 WebGL2，此时必须落到 SVG 回退，而不是一片空白。
  expect(["webgl2", "svg"]).toContain(await sphere.getAttribute("data-renderer"));

  expect(errors).toEqual([]);
});
