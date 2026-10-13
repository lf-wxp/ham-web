import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, test } from "./fixtures";

/**
 * SSTV / WSPR 解码页：这两页此前没有专属 e2e，改用共享的 `DecodeSession` 后至少要钉住页面
 * 层面的行为：坏样本给出错误提示（不是空白、不是一直转圈）、文件选择可键盘聚焦（FileInput）、
 * WSPR 基准频率输入保留原文。解码正确性由各 crate 的单测与协议冒烟用例负责。
 */

/** 只有 44 字节 WAV 头、没有音频数据的坏样本。 */
function emptyWav(rate = 12000): Buffer {
  const b = Buffer.alloc(44);
  b.write("RIFF", 0);
  b.writeUInt32LE(36, 4);
  b.write("WAVEfmt ", 8);
  b.writeUInt32LE(16, 16);
  b.writeUInt16LE(1, 20);
  b.writeUInt16LE(1, 22);
  b.writeUInt32LE(rate, 24);
  b.writeUInt32LE(rate * 2, 28);
  b.writeUInt16LE(2, 32);
  b.writeUInt16LE(16, 34);
  b.write("data", 36);
  return b;
}

for (const [route, heading] of [
  ["/sstv-decode", "SSTV 解码器"],
  ["/wspr-decode", "WSPR 解码器"],
] as const) {
  test(`${heading}：坏样本给出错误提示而非空白`, async ({ page }) => {
    await page.goto(route);
    await expect(page.getByRole("heading", { name: heading })).toBeVisible();
    await page.setInputFiles('input[type="file"]', {
      name: "bad.wav",
      mimeType: "audio/wav",
      buffer: emptyWav(),
    });
    await expect(page.getByText("解码失败").first()).toBeVisible();
  });
}

test("WSPR 解码：基准频率保留输入原文，非法值在解码时报错", async ({ page }) => {
  await page.goto("/wspr-decode");
  const base = page.getByLabel("基准频率", { exact: false }).first();
  await base.fill("1500.");
  // 旧实现把值做 f64 往返，`1500.` 会被立刻改写成 `1500`，清空时还会静默沿用旧值。
  await expect(base).toHaveValue("1500.");
  await base.fill("");
  await expect(base).toHaveValue("");
  await page.setInputFiles('input[type="file"]', {
    name: "bad.wav",
    mimeType: "audio/wav",
    buffer: emptyWav(),
  });
  await expect(page.getByText(/基带频率越界/)).toBeVisible();
});

/**
 * 真实录音端到端：上传 `crates/sstv/samples/` 里的两份**开源真实 SSTV 录音**（来源与许可见该目录
 * README），走完整浏览器链路（统一 Worker → wasm 解码 → canvas → PNG）。
 *
 * 为什么必须有这一条：此前所有 SSTV 测试都用自己的编码器合成信号，编码器与解码器共用同一份
 * 错误假设（VIS 起始位 300 ms、位极性写反、Robot / Scottie DX 时序错误），自洽测试一直是绿的，
 * 而对真实录音一份也解不出（5 份开源录音全部「未检测到 VIS 头」）。
 */
const SAMPLES = join(__dirname, "../../crates/sstv/samples");

/** 取 `<img>` 的 canvas 像素：返回若干点的 RGB。 */
async function samplePixels(
  page: import("@playwright/test").Page,
  points: Array<[number, number]>,
): Promise<number[][]> {
  return page.evaluate(async (pts) => {
    const img = document.querySelector("img[alt='SSTV 解码结果']") as HTMLImageElement;
    await img.decode();
    const c = document.createElement("canvas");
    c.width = img.naturalWidth;
    c.height = img.naturalHeight;
    const ctx = c.getContext("2d")!;
    ctx.drawImage(img, 0, 0);
    return pts.map(([x, y]) => Array.from(ctx.getImageData(x, y, 1, 1).data.slice(0, 3)));
  }, points);
}

test("SSTV 解码：真实 Robot 36 录音（CC0）识别模式并还原图像", async ({ page }) => {
  await page.goto("/sstv-decode");
  await page.setInputFiles('input[type="file"]', {
    name: "robot36_wikipedia_fr.wav",
    mimeType: "audio/wav",
    buffer: readFileSync(join(SAMPLES, "robot36_wikipedia_fr.wav")),
  });
  await expect(page.getByText("Robot 36", { exact: true })).toBeVisible({ timeout: 60000 });
  await expect(page.getByText("320 × 240")).toBeVisible();
  const [white, rightEdge, bar0] = await samplePixels(page, [
    [160, 75], // 白底（避开 logo 与文字）
    [318, 75], // 最右边缘：旧实现这里是绿线
    [30, 12], //  顶部色条第一格（灰）
  ]);
  expect(Math.min(...white)).toBeGreaterThan(220);
  expect(Math.min(...rightEdge)).toBeGreaterThan(220);
  // 灰色条：三通道接近且不偏白不偏黑。
  expect(Math.abs(bar0[0] - bar0[1])).toBeLessThan(40);
  expect(bar0[0]).toBeGreaterThan(100);
  expect(bar0[0]).toBeLessThan(230);
});

test("SSTV 解码：真实 Martin M1 录音（CC BY）识别模式并还原图像", async ({ page }) => {
  await page.goto("/sstv-decode");
  await page.setInputFiles('input[type="file"]', {
    name: "martin_m1_sunset.wav",
    mimeType: "audio/wav",
    buffer: readFileSync(join(SAMPLES, "martin_m1_sunset.wav")),
  });
  await expect(page.getByText("Martin M1", { exact: true })).toBeVisible({ timeout: 90000 });
  await expect(page.getByText("320 × 256")).toBeVisible();
  // 日落：晚霞（约 x=230, y=110）偏暖，左下林木近乎全黑。颜色分量顺序读错（G→B→R）会让晚霞偏蓝。
  const [glow, dark] = await samplePixels(page, [
    [230, 112],
    [30, 215],
  ]);
  expect(glow[0]).toBeGreaterThan(glow[2] + 25);
  expect(Math.max(...dark)).toBeLessThan(60);
});
