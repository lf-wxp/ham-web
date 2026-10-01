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
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"], channel: process.env.E2E_CHANNEL } },
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
