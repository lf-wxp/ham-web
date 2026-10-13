import { expect, pickOption, test } from "./fixtures";

/**
 * 独立 JS/TS 实现：Olivia MFSK 编码器（第三套实现，与 Rust 核心、纯 Python 参考
 * 实现互不共享代码）。算法转写自 sntfrc/olivia-python（MIT）。
 * 页面 wasm 解码必须还原这里编码的文本 —— 编解码分属两套实现，能对上才是真解码。
 */

const KEY = 0xe257e6d0291574ecn;

function gray(n: number): number {
  return n ^ (n >> 1);
}

function ifwht(data: number[]): number[] {
  let step = Math.floor(data.length / 2);
  while (step >= 1) {
    for (let ptr = 0; ptr < data.length; ptr += 2 * step) {
      for (let ptr2 = ptr; ptr2 < ptr + step; ptr2 += 1) {
        const b1 = data[ptr2];
        const b2 = data[ptr2 + step];
        data[ptr2] = b1 - b2;
        data[ptr2 + step] = b1 + b2;
      }
    }
    step = Math.floor(step / 2);
  }
  return data;
}

function keyBit(i: number): number {
  return Number((KEY >> BigInt(i & 63)) & 1n);
}

function encodeBlock(chars: number[], spb: number): number[] {
  const w: number[][] = [];
  for (let i = 0; i < spb; i += 1) {
    const row = new Array<number>(64).fill(0);
    const q = chars[i] ?? 0;
    row[q < 64 ? q : q - 64] = q < 64 ? 1 : -1;
    ifwht(row);
    for (let j = 0; j < 64; j += 1) {
      if (keyBit(j + 13 * i) === 1) row[j] = -row[j];
    }
    w.push(row);
  }
  const syms: number[] = [];
  for (let sym = 0; sym < 64; sym += 1) {
    let val = 0;
    for (let bis = 0; bis < spb; bis += 1) {
      const q = (bis - sym + 100 * spb) % spb;
      if (w[q][sym] < 0) val |= 1 << bis;
    }
    syms.push(val);
  }
  return syms;
}

/** 文本 → 16 位 PCM WAV（Uint8Array）。 */
function oliviaWav(
  text: string,
  symbols = 32,
  bandwidth = 1000,
  center = 1500,
): Buffer {
  const spb = Math.log2(symbols);
  const fsep = bandwidth / symbols;
  const wlen = Math.ceil(8000 / fsep);
  const rate = 8000;
  const samples: number[] = [];
  const firstFreq = center - bandwidth / 2 + fsep / 2;
  const lastFreq = center + bandwidth / 2 - fsep / 2;
  // preamble：静音 + 首尾音调交替尾（与参考实现一致）
  for (let i = 0; i < 64 * wlen - rate; i += 1) samples.push(0);
  for (const f of [firstFreq, lastFreq, firstFreq, lastFreq]) {
    for (let i = 0; i < rate / 4; i += 1) {
      samples.push(Math.sin((2 * Math.PI * f * samples.length) / rate));
    }
  }
  const chars = [...text].map((c) => c.charCodeAt(0));
  for (let i = 0; i < chars.length; i += spb) {
    const block = chars.slice(i, i + spb);
    while (block.length < spb) block.push(0);
    for (const sym of encodeBlock(block, spb)) {
      const f = firstFreq + fsep * gray(sym);
      for (let k = 0; k < wlen; k += 1) {
        samples.push(Math.sin((2 * Math.PI * f * samples.length) / rate));
      }
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

test("Olivia 解码：上传合成录音，解码出原文（32/1000）", async ({ page }) => {
  await page.goto("/olivia-decode");
  await expect(page.getByRole("heading", { name: "Olivia 解码器" })).toBeVisible();

  const wav = oliviaWav("THE QUICK BROWN FOX JUMPS");
  await page.locator('input[type="file"]').setInputFiles({
    name: "olivia-32.wav",
    mimeType: "audio/wav",
    buffer: wav,
  });
  await expect(page.locator("pre")).toContainText("THE QUICK BROWN FOX JUMPS", {
    timeout: 20000,
  });
});

test("Olivia 解码：8/250 子模式与数字换挡", async ({ page }) => {
  await page.goto("/olivia-decode");
  await pickOption(page, "子模式（音调数/带宽）", "8/250");
  const wav = oliviaWav("CQ CQ DE BG1AAA 599", 8, 250, 750);
  await page.locator('input[type="file"]').setInputFiles({
    name: "olivia-8.wav",
    mimeType: "audio/wav",
    buffer: wav,
  });
  await expect(page.locator("pre")).toContainText("CQ CQ DE BG1AAA 599", {
    timeout: 20000,
  });
});

test("Olivia 解码：坏样本给出错误提示而非空白", async ({ page }) => {
  await page.goto("/olivia-decode");
  // 纯 preamble（尾音调交替）而没有数据块 → 解不出任何有效块 → 如实报错。
  await page.locator('input[type="file"]').setInputFiles({
    name: "no-data.wav",
    mimeType: "audio/wav",
    buffer: oliviaWav(""),
  });
  await expect(page.getByText("解码失败").first()).toBeVisible();
});
