import { expect, pickOption, test } from "./fixtures";
import { VARICODE } from "./dominoex_varicode_data";
import { MFSK_VARICODE } from "./mfsk_varicode_data";

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

/**
 * FEC（MultiPSK 兼容）的独立 TS 编码器：K=7 卷积码（0x6D / 0x4F）→ 4 个码位一个 nibble →
 * 4×4×4 前向交织（fldigi `interleave`）。与 Rust 核心、Rust 参考实现都不共享代码；
 * 码表是同源生成的 `mfsk_varicode_data.ts`。
 */
function fecNibbles(text: string): number[] {
  const parity = (x: number) => {
    let p = 0;
    for (let v = x; v; v >>= 1) p ^= v & 1;
    return p;
  };
  let shreg = 0;
  const encode = (bit: number) => {
    shreg = ((shreg << 1) | bit) & 0x7f;
    return parity(0x6d & shreg) | (parity(0x4f & shreg) << 1);
  };
  const table = new Uint8Array(4 * 4 * 4);
  const at = (k: number, i: number, j: number) => 16 * k + 4 * i + j;
  const interleave = (v: number) => {
    const syms = [3, 2, 1, 0].map((sh) => (v >> sh) & 1);
    for (let k = 0; k < 4; k += 1) {
      for (let i = 0; i < 4; i += 1) for (let j = 0; j < 3; j += 1) table[at(k, i, j)] = table[at(k, i, j + 1)];
      for (let i = 0; i < 4; i += 1) table[at(k, i, 3)] = syms[i];
      for (let i = 0; i < 4; i += 1) syms[i] = table[at(k, i, 3 - i)];
    }
    return syms.reduce((acc, b) => (acc << 1) | b, 0);
  };
  const out: number[] = [];
  let acc = 0;
  let n = 0;
  const pushBits = (data: number, send: boolean) => {
    for (let i = 0; i < 2; i += 1) {
      acc = (acc << 1) | ((data >> i) & 1);
      n += 1;
      if (n === 4) {
        const v = interleave(acc);
        if (send) out.push(v);
        acc = 0;
        n = 0;
      }
    }
  };
  // 发送前清交织器（100 对码位，不发符号）。
  const zero = encode(0);
  for (let i = 0; i < 100; i += 1) pushBits(zero, false);
  const sendCode = (code: string) => {
    for (const b of code) pushBits(encode(b === "1" ? 1 : 0), true);
  };
  // 二次信道空格借用主码 1：前后各补若干，让接收端 Viterbi 收敛、尾部排空。
  const pad = MFSK_VARICODE[1];
  for (let i = 0; i < 4; i += 1) sendCode(pad);
  for (const ch of text) sendCode(MFSK_VARICODE[ch.charCodeAt(0)]);
  for (let i = 0; i < 10; i += 1) sendCode(pad);
  return out;
}

/** 文本 → 16 位 PCM WAV（DominoEX，8 kHz，IFK+）；`fec` 时 nibble 来自 FEC 编码器。 */
function dominoexWav(text: string, mode = 8, fec = false): Buffer {
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
  if (fec) {
    nibbles.push(...fecNibbles(text));
  } else {
    const idle = Math.max(2, Math.round((0.5 * baud) / 3));
    for (let i = 0; i < idle; i += 1) nibbles.push(...nibblesOf(0));
    for (const ch of text) nibbles.push(...nibblesOf(ch.charCodeAt(0)));
    for (let i = 0; i < Math.max(4, idle); i += 1) nibbles.push(...nibblesOf(0));
  }

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

test("DominoEX 解码：FEC 模式录音须勾选 FEC 才能还原", async ({ page }) => {
  await page.goto("/dominoex-decode");
  const wav = dominoexWav("CQ CQ DE BG1AAA PSE K", 8, true);
  const upload = () =>
    page.locator('input[type="file"]').setInputFiles({ name: "dex8-fec.wav", mimeType: "audio/wav", buffer: wav });

  // 不勾 FEC：调制一样、能锁定，但 nibble 含义不同 —— 要么解出乱码、要么判为无信号，
  // 总之不能是原文。
  await upload();
  const outcome = page.locator("pre").or(page.getByRole("alert"));
  await expect(outcome.first()).toBeVisible({ timeout: 20000 });
  await expect(outcome.first()).not.toContainText("CQ CQ DE BG1AAA");

  // 勾上 FEC 再解同一份录音：逐字还原。
  await page.reload();
  await page.getByRole("checkbox", { name: /FEC 模式/ }).click();
  await expect(page.getByRole("checkbox", { name: /FEC 模式/ })).toHaveAttribute("aria-checked", "true");
  await upload();
  await expect(page.locator("pre")).toContainText("CQ CQ DE BG1AAA PSE K", { timeout: 20000 });
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
