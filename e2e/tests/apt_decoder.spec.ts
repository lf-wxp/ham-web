import { closeSync, ftruncateSync, openSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, test } from "./fixtures";

/** 体积上限 96 MB，与 apt_decoder_page.rs 的 MAX_BYTES 保持一致。 */
const MAX_BYTES = 96 * 1024 * 1024;

/**
 * 创建一个稀疏大文件（不实际占用磁盘）。
 *
 * Playwright 的 `setInputFiles` 不接受超过 50 MB 的 buffer，必须传路径；
 * 体积校验读的是 `File.size`（来自文件系统元数据），不会真的把内容读进内存。
 */
function sparseFile(name: string, size: number): string {
  const path = join(tmpdir(), name);
  const fd = openSync(path, "w");
  ftruncateSync(fd, size);
  closeSync(fd);
  return path;
}

test("APT 解码：超大文件被拦截并给出可操作提示", async ({ page }) => {
  await page.goto("/apt-decoder");
  await expect(page.getByRole("heading", { level: 1, name: "NOAA APT 解码器" })).toBeVisible();

  const huge = sparseFile(`apt-huge-${Date.now()}.wav`, MAX_BYTES + 1024 * 1024);
  await page.setInputFiles('input[type="file"]', huge);

  await expect(page.getByText("解码失败")).toBeVisible();
  await expect(page.getByText(/文件过大/)).toBeVisible();
  // 提示要给出可操作的补救办法，而不只是报错
  await expect(page.getByText(/降采样|截取/)).toBeVisible();
  // 被拦截时不应进入解码中状态
  await expect(page.getByText(/正在后台解码|正在读取文件/)).toHaveCount(0);
});

test("APT 解码：正常大小文件进入处理流程", async ({ page }) => {
  await page.goto("/apt-decoder");
  await expect(page.getByRole("heading", { level: 1, name: "NOAA APT 解码器" })).toBeVisible();

  // 合法 WAV 头 + 1 秒数据（内容不是真 APT 信号，解码会失败，但不应被体积校验拦下）
  const header = Buffer.alloc(44);
  header.write("RIFF", 0);
  header.writeUInt32LE(36 + 8000, 4);
  header.write("WAVE", 8);
  header.write("fmt ", 12);
  header.writeUInt32LE(16, 16);
  header.writeUInt16LE(1, 20); // PCM
  header.writeUInt16LE(1, 22); // mono
  header.writeUInt32LE(8000, 24); // 8 kHz
  header.writeUInt32LE(16000, 28); // byte rate
  header.writeUInt16LE(2, 32); // block align
  header.writeUInt16LE(16, 34); // bits
  header.write("data", 36);
  header.writeUInt32LE(8000, 40);

  await page.setInputFiles('input[type="file"]', {
    name: "sample.wav",
    mimeType: "audio/wav",
    buffer: Buffer.concat([header, Buffer.alloc(8000, 128)]),
  });

  // 要么在处理（出现取消按钮），要么已出结果 / 报错 —— 总之不能停在「没反应」
  await expect(
    page
      .getByRole("button", { name: "取消" })
      .or(page.getByText("解码失败"))
      .or(page.getByText(/正在后台解码|正在读取文件/)),
  ).toBeVisible();
});
