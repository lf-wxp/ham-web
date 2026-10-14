import { expect, test } from "./fixtures";

/**
 * SDR 瀑布图：频谱画面是逐帧像素，断言落在「状态机」与读数上 ——
 * 开始 → 请求麦克风 → 停止的按钮流转、采样率推出的频率范围、增益滑块、权限被拒的提示。
 *
 * 整个文件用 Chromium 的假音频设备（`launchOptions` 只能在文件顶层设）；
 * 「权限被拒」那条在页面里把 `getUserMedia` 换成直接拒绝，不受假设备影响。
 */
test.use({
  launchOptions: { args: ["--use-fake-ui-for-media-stream", "--use-fake-device-for-media-stream"] },
  permissions: ["microphone"],
});

test("SDR 瀑布图：增益滑块调整读数", async ({ page }) => {
  await page.goto("/sdr-waterfall");
  await expect(page.getByText("频率范围 —")).toBeVisible();
  const gain = page.getByRole("slider", { name: "显示增益（dB）" });
  await expect(gain).toHaveAttribute("aria-valuetext", "+0 dB");
  await gain.focus();
  for (let i = 0; i < 10; i++) {
    await page.keyboard.press("ArrowRight");
  }
  await expect(gain).toHaveAttribute("aria-valuetext", "+10 dB");
  await expect(page.getByText("+10 dB", { exact: true })).toBeVisible();
});

test("SDR 瀑布图：麦克风权限被拒时给出提示并允许重试", async ({ page }) => {
  await page.addInitScript(() => {
    navigator.mediaDevices.getUserMedia = () =>
      Promise.reject(new DOMException("denied", "NotAllowedError"));
  });
  await page.goto("/sdr-waterfall");
  await page.getByRole("button", { name: "开始" }).click();
  await expect(page.getByRole("alert")).toHaveText("没有麦克风权限，请在地址栏允许后重试。");
  await expect(page.getByRole("button", { name: "开始" })).toBeEnabled();
});

test("SDR 瀑布图：开始后按采样率给出频率范围，停止后复原", async ({ page }) => {
  await page.goto("/sdr-waterfall");
  await page.getByRole("button", { name: "开始" }).click();
  await expect(page.getByRole("button", { name: "停止" })).toBeVisible();
  // 频率上限 = 采样率 / 2（假设备通常 48 kHz，但不写死）。
  const range = page.getByText(/^0 – \d+ Hz$/);
  await expect(range).toBeVisible();
  const top = Number((await range.innerText()).match(/– (\d+) Hz/)![1]);
  expect(top).toBeGreaterThanOrEqual(8000);
  await expect(page.locator('canvas[aria-label="频谱瀑布图"]')).toBeVisible();

  await page.getByRole("button", { name: "停止" }).click();
  await expect(page.getByRole("button", { name: "开始" })).toBeVisible();
});
