import { expect, test } from "./fixtures";

/**
 * 在测试里合成一段 WEFAX 音频（WAV）—— 不依赖仓库的 Rust 样本生成器，
 * 编码与解码分属两套实现，能过才是真解码。
 *
 * 参数与 crates/wefax/src/lib.rs 一致：AM 视频（幅度 = 亮度）、8000 Hz、
 * 2 行/秒（120 LPM）、每行同步脉冲 200 样本 + 380 像素 × 10 样本。
 */
function wefaxWav(height = 16): Buffer {
  const RATE = 8000;
  const LINE = 4000; // 0.5 s
  const SYNC = 200; // 5%
  const PIXELS = 380;
  const PP = 10;
  const samples: number[] = [];
  for (let row = 0; row < height; row += 1) {
    for (let i = 0; i < SYNC; i += 1) samples.push(1.0);
    for (let col = 0; col < PIXELS; col += 1) {
      const amp = col / PIXELS; // 0..1 渐变
      for (let i = 0; i < PP; i += 1) samples.push(amp);
    }
  }
  const dataLen = samples.length * 2;
  const out = Buffer.alloc(44 + dataLen);
  out.write("RIFF", 0);
  out.writeUInt32LE(36 + dataLen, 4);
  out.write("WAVEfmt ", 8);
  out.writeUInt32LE(16, 16);
  out.writeUInt16LE(1, 20);
  out.writeUInt16LE(1, 22);
  out.writeUInt32LE(RATE, 24);
  out.writeUInt32LE(RATE * 2, 28);
  out.writeUInt16LE(2, 32);
  out.writeUInt16LE(16, 34);
  out.write("data", 36);
  out.writeUInt32LE(dataLen, 40);
  samples.forEach((s, i) => out.writeInt16LE(Math.round(s * 32767), 44 + i * 2));
  return out;
}

test("WEFAX 解码：上传合成录音，解码出灰度图", async ({ page }) => {
  await page.goto("/wefax-decode");
  await expect(page.getByRole("heading", { name: "WEFAX 解码器" })).toBeVisible();

  await page.setInputFiles('input[type="file"]', {
    name: "wefax.wav",
    mimeType: "audio/wav",
    buffer: wefaxWav(16),
  });

  // Worker 后台解码后，结果区显示 380 × 16 的尺寸与图像。wasm 首次加载较慢，放宽到 30s。
  await expect(page.getByText(/380 × 16/)).toBeVisible({ timeout: 30000 });
  await expect(page.getByRole("img", { name: "WEFAX 解码图像" })).toBeVisible();
});

test("WEFAX 解码：坏样本给出错误提示而非空白", async ({ page }) => {
  await page.goto("/wefax-decode");
  await page.setInputFiles('input[type="file"]', {
    name: "empty.wav",
    mimeType: "audio/wav",
    buffer: wefaxWav(0).subarray(0, 44), // 只有 WAV 头
  });
  await expect(page.getByText("解码失败").first()).toBeVisible();
});
