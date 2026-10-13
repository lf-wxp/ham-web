import { expect, test } from "./fixtures";

/**
 * 在测试里合成一段 RTTY 音频（WAV）—— 不依赖仓库的 Rust 样本生成器，
 * 编码与解码分属两套实现，能过才是真解码。
 */
function rttyWav(text: string): Buffer {
  // ITA2（bit0 = 先发的位），与 crates/rtty/src/lib.rs 的表逐项一致。
  const LTRS = [
    "\x00", "E", "\n", "A", " ", "S", "I", "U", "\r", "D", "R", "J", "N", "F", "C", "K",
    "T", "Z", "L", "W", "H", "Y", "P", "Q", "O", "B", "G", "\x1F", "M", "X", "V", "\x1E",
  ];
  const FIGS = [
    "\x00", "3", "\n", "-", " ", "'", "8", "7", "\r", "$", "4", "'", ",", "!", ":", "(",
    "5", '"', ")", "2", "#", "6", "0", "1", "9", "?", "&", "\x1F", ".", "/", "=", "\x1E",
  ];
  const ltrsIndex: Record<string, number> = {};
  const figsIndex: Record<string, number> = {};
  LTRS.forEach((ch, i) => {
    ltrsIndex[ch] = i;
  });
  FIGS.forEach((ch, i) => {
    figsIndex[ch] = i;
  });

  const bits: number[] = [];
  let figs = false;
  const pushCode = (code: number) => {
    bits.push(0); // Space 起始位
    for (let b = 0; b < 5; b += 1) bits.push((code >> b) & 1);
    bits.push(1); // 停止位（1.5 中的 1）
  };
  for (const ch of text.toUpperCase()) {
    const l = ltrsIndex[ch];
    const f = figsIndex[ch];
    if (l === undefined && f === undefined) continue;
    const wantFigs = l === undefined;
    const code = wantFigs ? f : l;
    if (wantFigs !== figs) {
      pushCode(wantFigs ? 27 : 31);
      figs = wantFigs;
    }
    pushCode(code);
  }

  const RATE = 12000;
  const BAUD = 45.45;
  const spb = Math.round(RATE / BAUD);
  const mark = 2125;
  const space = mark + 170;
  const samples: number[] = [];
  const push = (freq: number, count: number) => {
    for (let i = 0; i < count; i += 1) {
      const t = samples.length / RATE;
      samples.push(Math.sin(2 * Math.PI * freq * t) * 0.5);
    }
  };
  let idx = 0;
  while (idx + 7 <= bits.length) {
    for (const b of bits.slice(idx, idx + 7)) push(b ? mark : space, spb);
    push(mark, Math.round(spb / 2)); // 1.5 停止位的余下 0.5
    idx += 7;
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

test("RTTY 解码：上传合成录音，解码出原文（含 FIGS 数字）", async ({ page }) => {
  await page.goto("/rtty-decode");
  await expect(page.getByRole("heading", { name: "RTTY 解码器" })).toBeVisible();

  await page.setInputFiles('input[type="file"]', {
    name: "cq.wav",
    mimeType: "audio/wav",
    buffer: rttyWav("CQ CQ DE BG1AAA 599 K"),
  });

  // Worker 后台解码后，原文与数字（FIGS 换挡）都要还原。
  // wasm 首次加载较慢，显式放宽到 30s（默认 15s 在 CI 冷缓存下偏紧）。
  await expect(page.getByText("CQ CQ DE BG1AAA 599 K")).toBeVisible({ timeout: 30000 });
  await expect(page.getByRole("button", { name: "解码其他文件" })).toBeVisible();
});

test("解码页卸载会清除超时，读取中的异步结果不访问已释放状态", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.clock.install();
  await page.addInitScript(() => {
    const original = Blob.prototype.arrayBuffer;
    let reads = 0;
    Blob.prototype.arrayBuffer = async function () {
      const bytes = await original.call(this);
      if (++reads > 1) await new Promise((resolve) => setTimeout(resolve, 2000));
      return bytes;
    };
  });
  await page.goto("/rtty-decode");
  const sample = { name: "standard.wav", mimeType: "audio/wav", buffer: rttyWav("RTTY TEST 599") };
  await page.setInputFiles('input[type="file"]', sample);
  await expect(page.getByText("RTTY TEST 599")).toBeVisible({ timeout: 30000 });
  await page.getByRole("button", { name: "解码其他文件" }).click();
  await page.setInputFiles('input[type="file"]', sample);
  await page.getByRole("link", { name: "RTTY 知识", exact: true }).click();
  await expect(page).toHaveURL(/\/rtty$/);
  await page.clock.runFor(301000);
  expect(errors).toEqual([]);
});

for (const route of ["/olivia-decode", "/dominoex-decode", "/navtex-decode", "/lrpt-decode"]) {
  test(`数字模式参考链接：${route}`, async ({ page }) => {
    await page.goto(route);
    await page.getByRole("link", { name: "数字模式知识", exact: true }).click();
    await expect(page).toHaveURL(/\/modes$/);
    await expect(page.getByRole("heading", { level: 1, name: "数字模式" })).toBeVisible();
  });
}

test("RTTY 解码：坏样本给出错误提示而非空白", async ({ page }) => {
  await page.goto("/rtty-decode");
  await page.setInputFiles('input[type="file"]', {
    name: "noise.wav",
    mimeType: "audio/wav",
    buffer: rttyWav("").subarray(0, 44), // 只有 WAV 头，没有音频数据
  });
  await expect(page.getByText("解码失败").first()).toBeVisible();
});
