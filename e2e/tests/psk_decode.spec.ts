import { expect, test } from "./fixtures";
import { synthesizeWav } from "./psk31";

test("PSK31 解码：上传合成 WAV 后解出文本", async ({ page }) => {
  await page.goto("/psk-decode");
  await expect(page.getByRole("heading", { level: 1, name: "PSK31 解码器" })).toBeVisible();
  await expect(page.getByText("选择 PSK31 录音（WAV）")).toBeVisible();

  const text = "CQ TEST";
  const wav = synthesizeWav(text, 8000, 1000, 0);
  await page.locator("#psk-file").setInputFiles({
    name: "psk31.wav",
    mimeType: "audio/wav",
    buffer: wav,
  });

  await expect(page.getByText("已解码 · psk31.wav")).toBeVisible();
  await expect(page.locator("pre")).toContainText(text);
});

test("PSK31 解码：载波频率输入框存在且可修改", async ({ page }) => {
  await page.goto("/psk-decode");
  const input = page.getByLabel("载波频率（Hz）");
  await expect(input).toBeVisible();
  await expect(input).toHaveValue("1000");
  await input.fill("1500");
  await expect(input).toHaveValue("1500");
});
