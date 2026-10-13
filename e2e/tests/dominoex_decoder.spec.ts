import { expect, pickOption, test } from "./fixtures";
import { VARICODE } from "./dominoex_varicode_data";

/**
 * 独立 JS/TS 实现：DominoEX 编码器（第三套实现，与 Rust 核心、纯 Python 参考
 * 实现互不共享代码）。IFK+ 与音调合成的算法逻辑手写（转写自 fldigi，GPL-3.0）；
 * Varicode 数据表与 Rust/Python 同源生成（`dominoex_varicode_data.ts`）。
 * 页面 wasm 解码必须还原这里编码的文本 —— 编解码分属两套实现，能对上才是真解码。
 */

function nibblesOf(c: number): number[] {
  const code = VARICODE[c] ?? [0, 0, 0];
  const out = [code[0]];
  for (const n of code.slice(1)) {
    if (n & 0x8) out.push(n);
    else break;
  }
  return out;
}

/** 文本 → 16 位 PCM WAV（DominoEX，8 kHz，IFK+）。 */
function dominoexWav(text: string, mode = 8): Buffer {
  const baudByMode: Record<number, number> = {
    4: 3.90625,
    5: 5.3833,
    8: 7.8125,
    11: 10.766,
    16: 15.625,
    22: 21.533,
  };
  const baud = baudByMode[mode] ?? 7.8125;
  const spacing = mode <= 8 ? baud * 2 : baud;
  const rate = 8000;
  const symlen = Math.round(rate / baud);
  const nibbles: number[] = [];
  const idle = Math.max(2, Math.round((0.5 * baud) / 3));
  for (let i = 0; i < idle; i += 1) nibbles.push(...nibblesOf(0));
  for (const ch of text) nibbles.push(...nibblesOf(ch.charCodeAt(0)));
  for (let i = 0; i < Math.max(4, idle); i += 1) nibbles.push(...nibblesOf(0));

  const samples: number[] = [];
  let prev = 0;
  for (const n of nibbles) {
    prev = (prev + 2 + n) % 18;
    const f = 1000 - (18 * spacing) / 2 + (prev + 0.5) * spacing;
    for (let i = 0; i < symlen; i += 1) {
      samples.push(Math.sin((2 * Math.PI * f * samples.length) / rate));
    }
  }
  const data = Buffer.alloc(samples.length * 2);
  samples.forEach((s, i) => {
    data.writeInt16LE(Math.max(-32767, Math.min(32767, Math.round(s * 32767))), i * 2);
  });
  const head = Buffer.alloc(44);
  head.write("RIFF", 0);
  head.writeUInt32LE(36 + data.length, 4);
  head.write("WAVEfmt ", 8);
  head.writeUInt32LE(16, 16);
  head.writeUInt16LE(1, 20);
  head.writeUInt16LE(1, 22);
  head.writeUInt32LE(rate, 24);
  head.writeUInt32LE(rate * 2, 28);
  head.writeUInt16LE(2, 32);
  head.writeUInt16LE(16, 34);
  head.write("data", 36);
  head.writeUInt32LE(data.length, 40);
  return Buffer.concat([head, data]);
}

test("DominoEX 解码：上传合成录音，解码出原文（默认 DominoEX 8）", async ({ page }) => {
  await page.goto("/dominoex-decode");
  await expect(page.getByRole("heading", { name: "DominoEX 解码器" })).toBeVisible();

  const wav = dominoexWav("THE QUICK BROWN FOX JUMPS");
  await page.locator('input[type="file"]').setInputFiles({
    name: "dex8.wav",
    mimeType: "audio/wav",
    buffer: wav,
  });
  await expect(page.locator("pre")).toContainText("THE QUICK BROWN FOX JUMPS", {
    timeout: 20000,
  });
});

test("DominoEX 解码：DominoEX 16 子模式", async ({ page }) => {
  await page.goto("/dominoex-decode");
  await pickOption(page, "子模式", "DominoEX 16");
  const wav = dominoexWav("CQ CQ DE BG1AAA 599", 16);
  await page.locator('input[type="file"]').setInputFiles({
    name: "dex16.wav",
    mimeType: "audio/wav",
    buffer: wav,
  });
  await expect(page.locator("pre")).toContainText("CQ CQ DE BG1AAA 599", {
    timeout: 20000,
  });
});

test("DominoEX 解码：坏样本给出错误提示而非空白", async ({ page }) => {
  await page.goto("/dominoex-decode");
  // 数据长度为 0 的合法 WAV：没有任何音频 → 如实报错。
  // （空闲符序列会解出自同步锁定的瞬态字符，不算「无信号」—— 用真空 WAV 才确定。）
  const empty = Buffer.alloc(44);
  empty.write("RIFF", 0);
  empty.writeUInt32LE(36, 4);
  empty.write("WAVEfmt ", 8);
  empty.writeUInt32LE(16, 16);
  empty.writeUInt16LE(1, 20);
  empty.writeUInt16LE(1, 22);
  empty.writeUInt32LE(8000, 24);
  empty.writeUInt32LE(16000, 28);
  empty.writeUInt16LE(2, 32);
  empty.writeUInt16LE(16, 34);
  empty.write("data", 36);
  empty.writeUInt32LE(0, 40);
  await page.locator('input[type="file"]').setInputFiles({
    name: "empty.wav",
    mimeType: "audio/wav",
    buffer: empty,
  });
  // 零数据 WAV 在解析层就被拒（不是解码失败，是输入无效 —— 两种错误都要如实显示）。
  await expect(page.getByText(/文件中没有可解码的音频数据|解码失败/).first()).toBeVisible();
});
