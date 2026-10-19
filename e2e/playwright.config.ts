import { defineConfig, devices } from "@playwright/test";

// 默认用 release 服务器托管 ../dist（需先 `cargo make build-web`）；
// 设置 E2E_BASE_URL 可直接测已运行的站点，例如 trunk dev：E2E_BASE_URL=http://127.0.0.1:3000
const external = process.env.E2E_BASE_URL;
const port = Number(process.env.E2E_PORT ?? 4173);
const baseURL = external ?? `http://127.0.0.1:${port}`;

export default defineConfig({
  testDir: "./tests",
  timeout: 60_000,
  expect: { timeout: 15_000 },
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["github"], ["html", { open: "never" }]] : "list",
  use: {
    baseURL,
    locale: "zh-CN",
    timezoneId: "Asia/Shanghai",
    serviceWorkers: "block",
    trace: "retain-on-failure",
  },
  // E2E_CHANNEL=chrome 可改用本机已安装的 Chrome，免去下载 Playwright 浏览器
  //
  // 跨内核冒烟默认关闭：需要先 `npx playwright install firefox webkit`，
  // 没装时打开这两个 project 会让每次 `npx playwright test` 都红。
  // 用 `E2E_CROSS=1 npx playwright test --project=firefox --project=webkit` 单独跑。
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"], channel: process.env.E2E_CHANNEL } },
    ...(process.env.E2E_CROSS
      ? [
          // 全量用例依赖 Web Serial、假音频设备与 WebGL2，这两个内核跑不了；
          // 只跑 `cross_browser.spec.ts`：核心页面能开、无未捕获异常、三维图有 SVG 回退。
          {
            name: "firefox",
            use: { ...devices["Desktop Firefox"] },
            testMatch: /cross_browser\.spec\.ts/,
          },
          {
            name: "webkit",
            use: { ...devices["Desktop Safari"] },
            testMatch: /cross_browser\.spec\.ts/,
          },
        ]
      : []),
  ],
  webServer: external
    ? undefined
    : {
        command: "cargo run -q -p ham-web-server --release",
        cwd: "..",
        url: `${baseURL}/healthz`,
        reuseExistingServer: !process.env.CI,
        timeout: 600_000,
        env: { DIST_DIR: "dist", HOST: "127.0.0.1", PORT: String(port) },
      },
});
