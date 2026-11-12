import { expect, pickOption, test } from "./fixtures";
import { MFSK_VARICODE } from "./mfsk_varicode_data";

/**
 * 独立 JS/TS 实现：MFSK 编码器（第二套实现，与 Rust 核心不共享代码）。
 * 卷积码、交织器、格雷码与音调合成的算法逻辑手写（转写自 fldigi `mfsk.cxx` / `interleave.cxx` /
 * `viterbi.cxx`，GPL-3.0）；Varicode 数据表与 Rust 同源生成（`mfsk_varicode_data.ts`）。
 * 页面 wasm 解码必须还原这里编码的文本 —— 编解码分属两套实现，能对上才是真解码。
 * Rust 一侧另有与 fldigi 上游代码逐位对拍和真实录音的验证（见 `crates/mfsk/reference/README.md`）。
 */

interface ModeSpec {
  rate: number;
  symlen: number;
  symbits: number;
  depth: number;
  preamble: number;
}

const MODES: Record<string, ModeSpec> = {
  "16": { rate: 8000, symlen: 512, symbits: 4, depth: 10, preamble: 107 },
  "32": { rate: 8000, symlen: 256, symbits: 4, depth: 10, preamble: 107 },
};

const parity = (x: number) => {
  let p = 0;
  for (let v = x; v; v >>= 1) p ^= v & 1;
  return p;
};

/** 文本 → 一帧完整的音调号序列（已过格雷码映射）。 */
function mfskTones(text: string, m: ModeSpec): number[] {
  let shreg = 0;
  const encode = (bit: number) => {
    shreg = ((shreg << 1) | bit) & 0x7f;
    return parity(0x6d & shreg) | (parity(0x4f & shreg) << 1);
  };
  const size = m.symbits;
  const table = new Uint8Array(size * size * m.depth);
  const at = (k: number, i: number, j: number) => size * size * k + size * i + j;
  const interleave = (v: number) => {
    const syms = Array.from({ length: size }, (_, i) => (v >> (size - 1 - i)) & 1);
    for (let k = 0; k < m.depth; k += 1) {
      for (let i = 0; i < size; i += 1) for (let j = 0; j < size - 1; j += 1) table[at(k, i, j)] = table[at(k, i, j + 1)];
      for (let i = 0; i < size; i += 1) table[at(k, i, size - 1)] = syms[i];
      for (let i = 0; i < size; i += 1) syms[i] = table[at(k, i, size - i - 1)];
    }
    return syms.reduce((acc, b) => (acc << 1) | b, 0);
  };
  // fldigi 的 `grayencode`（前缀异或；名字与传统叫法相反）。
  const gray = (d: number) => {
    let bits = d;
    for (let s = 1; s < 8; s += 1) bits ^= d >> s;
    return bits;
  };
  const tones: number[] = [];
  let acc = 0;
  let n = 0;
  const push = (data: number, send: boolean) => {
    for (let i = 0; i < 2; i += 1) {
      acc = (acc << 1) | ((data >> i) & 1);
      n += 1;
      if (n === size) {
        const v = interleave(acc);
        if (send) tones.push(gray(v & ((1 << size) - 1)));
        acc = 0;
        n = 0;
      }
    }
  };
  const sendBit = (b: number) => push(encode(b), true);
  const sendCode = (code: string) => {
    for (const b of code) sendBit(b === "1" ? 1 : 0);
  };
  // 前导：`preamble` 对零码位只进交织器不发符号，再发 `preamble / 3` 个零位。
  const zero = encode(0);
  for (let i = 0; i < m.preamble; i += 1) push(zero, false);
  for (let i = 0; i < Math.floor(m.preamble / 3); i += 1) sendBit(0);
  for (const c of [13, 2, 13]) sendCode(MFSK_VARICODE[c]);
  for (const ch of new TextEncoder().encode(text)) sendCode(MFSK_VARICODE[ch]);
  for (const c of [13, 4, 13]) sendCode(MFSK_VARICODE[c]);
  // 收尾：先一个 1 冲掉对端 Varicode，再 `preamble` 个零冲卷积码与交织器。
  sendBit(1);
  for (let i = 0; i < m.preamble; i += 1) sendBit(0);
  return tones;
}

/** 文本 → 16 位 PCM WAV（8 kHz）。`reverse` 把音调序倒过来，模拟下边带录音。 */
function mfskWav(text: string, opts: { mode?: string; center?: number; reverse?: boolean } = {}): Buffer {
  const m = MODES[opts.mode ?? "16"];
  const center = opts.center ?? 1200;
  const ntones = 1 << m.symbits;
  const baud = m.rate / m.symlen;
  const low = center - ((ntones - 1) * baud) / 2;
  const samples: number[] = [];
  let phase = 0;
  for (const t of mfskTones(text, m)) {
    const tone = opts.reverse ? ntones - 1 - t : t;
    const step = (2 * Math.PI * (low + tone * baud)) / m.rate;
    for (let i = 0; i < m.symlen; i += 1) {
      samples.push(Math.cos(phase));
      phase += step;
    }
  }
  return pcmWav(samples, m.rate);
}

function pcmWav(samples: number[], rate: number): Buffer {
  const data = Buffer.alloc(samples.length * 2);
  samples.forEach((s, i) => {
    data.writeInt16LE(Math.max(-32767, Math.min(32767, Math.round(s * 20000))), i * 2);
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

const upload = (page: import("@playwright/test").Page, name: string, buffer: Buffer) =>
  page.locator('input[type="file"]').setInputFiles({ name, mimeType: "audio/wav", buffer });

test("MFSK 解码：上传合成录音，自动检测中心频率并还原原文（默认 MFSK16）", async ({ page }) => {
  await page.goto("/mfsk-decode");
  await expect(page.getByRole("heading", { name: "MFSK 解码器" })).toBeVisible();

  // 1337.5 Hz 不在任何整数音调边界上：自动检测要把它找回来。
  await upload(page, "mfsk16.wav", mfskWav("THE QUICK BROWN FOX JUMPS OVER 1234567890", { center: 1337.5 }));
  await expect(page.locator("pre")).toContainText("THE QUICK BROWN FOX JUMPS OVER 1234567890", { timeout: 30000 });
  await expect(page.getByText(/中心频率 1337\.\d Hz/)).toBeVisible();
});

test("MFSK 解码：手填中心频率与 MFSK32 子模式", async ({ page }) => {
  await page.goto("/mfsk-decode");
  await pickOption(page, "MFSK 子模式", "MFSK32");
  await page.getByLabel(/中心频率/).fill("900");
  await upload(page, "mfsk32.wav", mfskWav("CQ CQ DE BG1AAA 599", { mode: "32", center: 900 }));
  await expect(page.locator("pre")).toContainText("CQ CQ DE BG1AAA 599", { timeout: 30000 });
  await expect(page.getByText(/中心频率 900\.0 Hz/)).toBeVisible();
});

test("MFSK 解码：下边带录音须勾选「边带反转」才能还原", async ({ page }) => {
  await page.goto("/mfsk-decode");
  const wav = mfskWav("CQ CQ DE BG1AAA PSE K", { reverse: true });

  // 不勾：能锁定、音调序是反的 —— 要么乱码、要么判为无信号，总之不能是原文。
  await upload(page, "lsb.wav", wav);
  const outcome = page.locator("pre").or(page.getByRole("alert"));
  await expect(outcome.first()).toBeVisible({ timeout: 30000 });
  await expect(outcome.first()).not.toContainText("CQ CQ DE BG1AAA");

  await page.reload();
  const reverse = page.getByRole("checkbox", { name: /边带反转/ });
  await reverse.click();
  await expect(reverse).toHaveAttribute("aria-checked", "true");
  await upload(page, "lsb.wav", wav);
  await expect(page.locator("pre")).toContainText("CQ CQ DE BG1AAA PSE K", { timeout: 30000 });
});

test("MFSK 解码：中心频率写成非数字时如实报错、不发 Worker", async ({ page }) => {
  await page.goto("/mfsk-decode");
  await page.getByLabel(/中心频率/).fill("1.2.3");
  await upload(page, "mfsk16.wav", mfskWav("CQ"));
  await expect(page.getByRole("alert")).toContainText("中心频率越界");
});

test("MFSK 解码：中心频率让信号带越过 Nyquist 时给出明确错误", async ({ page }) => {
  await page.goto("/mfsk-decode");
  await page.getByLabel(/中心频率/).fill("3990");
  await upload(page, "mfsk16.wav", mfskWav("CQ CQ DE BG1AAA 599"));
  await expect(page.getByRole("alert")).toContainText("中心频率越界", { timeout: 20000 });
});

test("MFSK 解码：纯噪声与空文件如实报错而非乱码", async ({ page }) => {
  await page.goto("/mfsk-decode");
  // 固定种子的均匀噪声，20 秒。
  let seed = 0x12345678;
  const noise = Array.from({ length: 8000 * 20 }, () => {
    seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
    return (seed >>> 8) / 8388608 - 1;
  });
  await upload(page, "noise.wav", pcmWav(noise, 8000));
  await expect(page.getByRole("alert")).toContainText("未识别出有效的 MFSK 信号", { timeout: 30000 });

  await page.reload();
  await upload(page, "empty.wav", pcmWav([], 8000));
  await expect(page.getByRole("alert")).toBeVisible({ timeout: 20000 });
  await expect(page.locator("pre")).toHaveCount(0);
});
