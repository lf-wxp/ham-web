import { expect, test } from "./fixtures";
import { readFileSync } from "node:fs";
import { join } from "node:path";

/**
 * LRPT 解码器端到端：上传 lrpt-encoder 生成的**合成**录音夹具（色带图，约 2 秒；首尾残缺行被裁剪后解出 1568 × 48）
 * → 页面 wasm 完整链路（解调 → Viterbi → RS → VCDU/MPDU → JPEG）→ 图像。
 *
 * 夹具 `crates/lrpt/samples/bands.wav` 由独立 Rust 实现 lrpt-encoder（MIT）
 * 对 `truth_bands.png` 编码 —— 与页面里的解码器互不共享代码。
 */

test("LRPT 解码：上传合成录音夹具，解码出图像", async ({ page }) => {
  await page.goto("/lrpt-decode");
  await expect(page.getByRole("heading", { name: "LRPT 解码器" })).toBeVisible();

  const wav = readFileSync(
    join(__dirname, "../../crates/lrpt/samples/bands.wav"),
  );
  await page.locator('input[type="file"]').setInputFiles({
    name: "bands.wav",
    mimeType: "audio/wav",
    buffer: wav,
  });
  // 尺寸文本 = "1568 × 48"（8 MCU 行的夹具被首尾残缺行裁剪后剩 6 行）
  await expect(page.getByText("1568 × 48")).toBeVisible({ timeout: 60000 });
  const img = page.locator("img[alt='LRPT 解码器']");
  await expect(img).toBeVisible();

  // 像素级抽查：色带 0 = (13, 71, 127)，色带 196 边界前最后一列同理。
  // 注意：`<img>` 可见 ≠ 图片已解码完（data URL 的 decode 是异步的）——
  // 先 `await img.decode()` 再 drawImage，否则 getImageData 会拿到全透明。
  const pixel = await page.evaluate(async () => {
    const img = document.querySelector("img[alt='LRPT 解码器']") as HTMLImageElement;
    await img.decode();
    if (img.naturalWidth !== 1568) {
      throw new Error(`图像未解码完成：naturalWidth=${img.naturalWidth}`);
    }
    const canvas = document.createElement("canvas");
    canvas.width = 1568;
    canvas.height = 16;
    const ctx = canvas.getContext("2d")!;
    ctx.drawImage(img, 0, 0);
    return {
      first: Array.from(ctx.getImageData(4, 8, 1, 1).data),
      mid: Array.from(ctx.getImageData(1564, 8, 1, 1).data),
    };
  });
  expect(pixel.first.slice(0, 3)).toEqual([13, 71, 127]); // 带 0
  expect(pixel.mid.slice(0, 3)).toEqual([
    (195 * 7 + 13) % 256,
    (195 * 29 + 71) % 256,
    (195 * 53 + 127) % 256,
  ]); // 带 195
});

test("LRPT 解码：坏样本给出错误提示而非空白", async ({ page }) => {
  await page.goto("/lrpt-decode");
  const empty = Buffer.alloc(44);
  empty.write("RIFF", 0);
  empty.writeUInt32LE(36, 4);
  empty.write("WAVEfmt ", 8);
  empty.writeUInt32LE(16, 16);
  empty.writeUInt16LE(1, 20);
  empty.writeUInt16LE(2, 22); // 立体声
  empty.writeUInt32LE(288000, 24);
  empty.writeUInt32LE(288000 * 4, 28);
  empty.writeUInt16LE(4, 32);
  empty.writeUInt16LE(16, 34);
  empty.write("data", 36);
  empty.writeUInt32LE(0, 40);
  await page.locator('input[type="file"]').setInputFiles({
    name: "empty.wav",
    mimeType: "audio/wav",
    buffer: empty,
  });
  await expect(page.getByText(/文件中没有可解码的音频数据|解码失败/).first()).toBeVisible();
});
