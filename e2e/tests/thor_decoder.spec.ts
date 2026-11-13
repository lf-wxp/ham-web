import { expect, pickOption, test } from "./fixtures";
import { MFSK_VARICODE } from "./mfsk_varicode_data";
import { THOR_SECONDARY_VARICODE } from "./thor_varicode_data";

/**
 * 独立 JS/TS 实现：THOR 编码器（第二套实现，与 Rust 核心不共享代码）。
 * IFK+、卷积码、交织器与音调合成的算法逻辑手写（转写自 fldigi `thor.cxx` / `interleave.cxx` /
 * `viterbi.cxx`，GPL-3.0）；两张 Varicode 数据表与 Rust 同源生成。
 * 页面 wasm 解码必须还原这里编码的文本 —— 编解码分属两套实现，能对上才是真解码。
 * Rust 一侧另有与 fldigi 上游代码逐位对拍、fldigi 自己的收发端双向验证（见 `crates/mfsk/reference/`）。
 */

interface ModeSpec {
  rate: number;
  symlen: number;
  spacingFactor: number;
  depth: number;
  flush: number;
  k15: boolean;
}

const MODES: Record<string, ModeSpec> = {
  "16": { rate: 8000, symlen: 512, spacingFactor: 1, depth: 10, flush: 4, k15: false },
  "8": { rate: 8000, symlen: 1024, spacingFactor: 2, depth: 10, flush: 4, k15: false },
  "100": { rate: 8000, symlen: 80, spacingFactor: 1, depth: 50, flush: 40, k15: true },
  // fldigi 4.2.x 新增：THOR25 用 K = 15（不是名字看起来的慢速 K = 7）；THOR56 是 16 kHz 采样、符号长 290。
  "25": { rate: 8000, symlen: 320, spacingFactor: 1, depth: 25, flush: 20, k15: true },
  "56": { rate: 16000, symlen: 290, spacingFactor: 1, depth: 10, flush: 20, k15: false },
};

const parity = (x: number) => {
  let p = 0;
  for (let v = x; v; v >>>= 1) p ^= v & 1;
  return p;
};

/** 文本 → 一帧完整的音调号序列（0–17，IFK+ 之后）；`\u0001` 之后的字符走二次信道。 */
function thorTones(text: string, m: ModeSpec, eot = true): number[] {
  const [k, p1, p2] = m.k15 ? [15, 0o44735, 0o63057] : [7, 0x6d, 0x4f];
  let shreg = 0;
  const encode = (bit: number) => {
    shreg = ((shreg << 1) | bit) & ((1 << k) - 1);
    return parity(p1 & shreg) | (parity(p2 & shreg) << 1);
  };
  const size = 4;
  const table = new Uint8Array(size * size * m.depth);
  const at = (kk: number, i: number, j: number) => size * size * kk + size * i + j;
  const interleave = (v: number) => {
    const syms = Array.from({ length: size }, (_, i) => (v >> (size - 1 - i)) & 1);
    for (let kk = 0; kk < m.depth; kk += 1) {
      for (let i = 0; i < size; i += 1) for (let j = 0; j < size - 1; j += 1) table[at(kk, i, j)] = table[at(kk, i, j + 1)];
      for (let i = 0; i < size; i += 1) table[at(kk, i, size - 1)] = syms[i];
      for (let i = 0; i < size; i += 1) syms[i] = table[at(kk, i, size - i - 1)];
    }
    return syms.reduce((acc, b) => (acc << 1) | b, 0);
  };
  const tones: number[] = [];
  let prev = 0;
  const sendSymbol = (nibble: number) => {
    prev = (prev + 2 + nibble) % 18;
    tones.push(prev);
  };
  let acc = 0;
  let n = 0;
  const push = (data: number, send: boolean) => {
    for (let i = 0; i < 2; i += 1) {
      acc = (acc << 1) | ((data >> i) & 1);
      n += 1;
      if (n === size) {
        const v = interleave(acc);
        if (send) sendSymbol(v);
        acc = 0;
        n = 0;
      }
    }
  };
  const sendCode = (code: string) => {
    for (const b of code) push(encode(b === "1" ? 1 : 0), true);
  };
  const sendChar = (c: number, secondary = false) => {
    if (!secondary) sendCode(MFSK_VARICODE[c]);
    else sendCode(c >= 32 && c <= 122 ? THOR_SECONDARY_VARICODE[c - 32] : MFSK_VARICODE[0]);
  };

  // Clearbits：1400 对零码位只进交织器不发符号；再发 16 个 sendsymbol(0) 作前导。
  const zero = encode(0);
  for (let i = 0; i < 1400; i += 1) push(zero, false);
  for (let i = 0; i < 16; i += 1) sendSymbol(0);
  sendChar(0);
  for (const c of [13, 2, 13]) sendChar(c);
  const bytes = new TextEncoder().encode(text);
  for (let i = 0; i < bytes.length; i += 1) {
    if (bytes[i] === 1 && i + 1 < bytes.length) {
      sendChar(bytes[i + 1], true);
      i += 1;
    } else sendChar(bytes[i]);
  }
  // 图片发射在报头后直接冲刷，没有 EOT（EOT 要等图像发完才发）。
  if (eot) for (const c of [13, 4, 13]) sendChar(c);
  for (let i = 0; i < m.flush; i += 1) sendChar(0);
  return tones;
}

/** 文本 → 16 位 PCM WAV（8 kHz）。`reverse` 把音调序倒过来，模拟下边带录音。 */
function thorWav(
  text: string,
  opts: { mode?: string; center?: number; reverse?: boolean; carrier?: number; noise?: number } = {},
): Buffer {
  const m = MODES[opts.mode ?? "16"];
  const center = opts.center ?? 1500;
  const baud = m.rate / m.symlen;
  const spacing = baud * m.spacingFactor;
  const samples: number[] = [];
  let phase = 0;
  for (const t of thorTones(text, m)) {
    const tone = opts.reverse ? 17 - t : t;
    const step = (2 * Math.PI * (center + (tone - 8.5) * spacing)) / m.rate;
    for (let i = 0; i < m.symlen; i += 1) {
      samples.push(Math.cos(phase));
      phase += step;
    }
  }
  return pcmWav(
    opts.carrier || opts.noise ? impair(samples, m.rate, center, opts.carrier ?? 0, opts.noise ?? 0) : samples,
    m.rate,
  );
}

/**
 * 图片发射（THOR 16，59 × 74 彩色，报头 `pic%T`）：文本 + 报头 → 冲刷 → 20 个符号长的静音 →
 * 20 个符号长的同步音（中心 − 0.6 带宽，带宽 = 18 个音调间隔）→ 每像素 10 个样本的调频图像
 * （彩色每行 R、G、B 三遍，频率 = 中心 + 带宽 × (字节 − 128) / 256）。结构转写自 fldigi `thor.cxx` 的 `send_image`。
 */
function thorPictureWav(text: string, rgb: Uint8Array, center = 1500): Buffer {
  const [width, height] = [59, 74];
  const m = MODES["16"];
  const spacing = m.rate / m.symlen;
  const bandwidth = 18 * spacing;
  const samples: number[] = [];
  let phase = 0;
  const tone = (freq: number, count: number) => {
    const step = (2 * Math.PI * freq) / m.rate;
    for (let i = 0; i < count; i += 1) {
      samples.push(Math.cos(phase));
      phase += step;
    }
  };
  for (const t of thorTones(`${text}pic%T`, m, false)) tone(center + (t - 8.5) * spacing, m.symlen);
  samples.push(...new Array<number>(20 * m.symlen).fill(0));
  tone(center - 0.6 * bandwidth, 20 * m.symlen);
  for (let y = 0; y < height; y += 1) {
    for (let plane = 0; plane < 3; plane += 1) {
      for (let x = 0; x < width; x += 1) {
        tone(center + (bandwidth * (rgb[3 * (y * width + x) + plane] - 128)) / 256, 10);
      }
    }
  }
  samples.push(...new Array<number>(800).fill(0));
  return pcmWav(samples, m.rate);
}

/**
 * 给采样加一个恒定载波（`carrier`：相对信号幅度的倍数，频率 = 中心 + 50 Hz，落在两个音调之间）与固定种子的均匀噪声
 * （`noise`：幅度，信号幅度 = 1），再整体缩小到不削波。信噪比不受缩放影响。
 */
function impair(samples: number[], rate: number, center: number, carrier: number, noise: number): number[] {
  const w = (2 * Math.PI * (center + 50)) / rate;
  let seed = 0x2468ace1;
  const scale = 1 / (1 + carrier + noise);
  return samples.map((s, i) => {
    seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
    const n = ((seed >>> 8) / 8388608 - 1) * noise;
    return (s + carrier * Math.sin(w * i) + n) * scale;
  });
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

test("THOR 解码：上传合成录音，自动检测中心频率并还原原文（默认 THOR 16）", async ({ page }) => {
  await page.goto("/thor-decode");
  await expect(page.getByRole("heading", { name: "THOR 解码器" })).toBeVisible();

  // 正文后补空格：THOR 发端的冲刷比管线延迟短，尾巴要靠接收端的 flush 推出来 —— 补几个空格后全文可见。
  await upload(page, "thor16.wav", thorWav("THE QUICK BROWN FOX JUMPS OVER 1234567890   ", { center: 1337.5 }));
  await expect(page.locator("pre")).toContainText("THE QUICK BROWN FOX JUMPS OVER 1234567890", { timeout: 30000 });
  await expect(page.getByText(/中心频率 1337\.\d Hz/)).toBeVisible();
});

test("THOR 解码：二次信道（ID 信标）单独显示，不混进正文", async ({ page }) => {
  await page.goto("/thor-decode");
  await upload(page, "thor16-id.wav", thorWav("CQ \u0001f\u0001l\u0001d\u0001i\u0001g\u0001i DE BG1AAA   "));
  await expect(page.locator("pre")).toContainText("DE BG1AAA", { timeout: 30000 });
  await expect(page.locator("pre")).not.toContainText("fldigi");
  await expect(page.getByText(/二次信道（ID 信标）：.*fldigi/)).toBeVisible();
});

test("THOR 解码：THOR 8 与高速的 THOR 100（K = 15 长约束卷积码）", async ({ page }) => {
  await page.goto("/thor-decode");
  await pickOption(page, "THOR 子模式", "THOR 8");
  await upload(page, "thor8.wav", thorWav("CQ CQ DE BG1AAA 599   ", { mode: "8", center: 1200 }));
  await expect(page.locator("pre")).toContainText("CQ CQ DE BG1AAA 599", { timeout: 30000 });

  await page.reload();
  await pickOption(page, "THOR 子模式", "THOR 100");
  await page.getByLabel(/中心频率/).fill("2000");
  await upload(page, "thor100.wav", thorWav("CQ CQ DE BG1AAA 599   ", { mode: "100", center: 2000 }));
  await expect(page.locator("pre")).toContainText("CQ CQ DE BG1AAA 599", { timeout: 30000 });
});

test("THOR 解码：fldigi 4.2.x 新增的 THOR 25（K = 15）与 THOR 56（16 kHz）", async ({ page }) => {
  await page.goto("/thor-decode");
  await pickOption(page, "THOR 子模式", "THOR 25");
  await upload(page, "thor25.wav", thorWav("CQ CQ DE BG1AAA 599   ", { mode: "25", center: 1337.5 }));
  await expect(page.locator("pre")).toContainText("CQ CQ DE BG1AAA 599", { timeout: 60000 });

  await page.reload();
  await pickOption(page, "THOR 子模式", "THOR 56");
  await upload(page, "thor56.wav", thorWav("CQ CQ DE BG1AAA 599   ", { mode: "56", center: 2000 }));
  await expect(page.locator("pre")).toContainText("CQ CQ DE BG1AAA 599", { timeout: 60000 });
});

test("THOR 解码：图片模式（pic%T 报头 + 同步音 + 模拟调频图像）显示出图片，正文里没有图像段的乱码", async ({ page }) => {
  const [w, h] = [59, 74];
  const rgb = new Uint8Array(w * h * 3);
  for (let y = 0; y < h; y += 1) {
    for (let x = 0; x < w; x += 1) {
      rgb.set([x < w / 2 ? 220 : 40, y < h / 2 ? 180 : 70, x < w / 2 ? 30 : 210], 3 * (y * w + x));
    }
  }
  await page.goto("/thor-decode");
  // 不给中心频率：自动检测；中心频率靠同步音定（比文本里的音调梳准得多）。
  await upload(page, "picture.wav", thorPictureWav("CQ ", rgb));
  const figure = page.getByTestId("decode-picture");
  await expect(figure).toBeVisible({ timeout: 60000 });
  await expect(figure.locator("figcaption")).toContainText("59 × 74");
  await expect(page.locator("pre")).toContainText("CQ pic%T");
  // 画出来的像素：四个象限中心的颜色与发出去的一致（FM 解调加低通的容差 ±4）。
  const got = await figure.locator("img").evaluate(async (img: HTMLImageElement) => {
    await img.decode();
    const canvas = document.createElement("canvas");
    canvas.width = img.naturalWidth;
    canvas.height = img.naturalHeight;
    const ctx = canvas.getContext("2d")!;
    ctx.drawImage(img, 0, 0);
    return [[14, 18], [44, 18], [14, 55], [44, 55]].map(([x, y]) => Array.from(ctx.getImageData(x, y, 1, 1).data.slice(0, 3)));
  });
  const want = [[220, 180, 30], [40, 180, 210], [220, 70, 30], [40, 70, 210]];
  got.forEach((px, i) => px.forEach((v, c) => expect(Math.abs(v - want[i][c])).toBeLessThanOrEqual(4)));
  await expect(figure.getByRole("link", { name: /下载|PNG/ })).toHaveAttribute("download", "thor-picture-1.png");
});

test("THOR 解码：与信号等幅 / 更强的恒定载波加噪声下仍能还原（fldigi 自己的接收机在这里解不出）", async ({ page }) => {
  const text = Array.from({ length: 6 }, () => "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG 0123456789").join(" ") + "   ";
  await page.goto("/thor-decode");
  for (const carrier of [1, 2]) {
    await page.getByLabel(/中心频率/).fill("1500");
    await upload(page, `carrier${carrier}.wav`, thorWav(text, { center: 1500, carrier, noise: 3 }));
    await expect(page.locator("pre")).toContainText("THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG 0123456789", { timeout: 60000 });
    expect((await page.locator("pre").textContent())?.split("THE QUICK BROWN FOX").length ?? 0).toBeGreaterThan(5);
    await page.getByRole("button", { name: "解码其他文件" }).click();
  }
});

test("THOR 解码：下边带录音须勾选「边带反转」才能还原", async ({ page }) => {
  await page.goto("/thor-decode");
  const wav = thorWav("CQ CQ DE BG1AAA PSE K   ", { reverse: true });

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

test("THOR 解码：中心频率写成非数字或越过 Nyquist 时如实报错", async ({ page }) => {
  await page.goto("/thor-decode");
  await page.getByLabel(/中心频率/).fill("1.2.3");
  await upload(page, "thor16.wav", thorWav("CQ"));
  await expect(page.getByRole("alert")).toContainText("中心频率越界");

  await page.reload();
  await page.getByLabel(/中心频率/).fill("3990");
  await upload(page, "thor16.wav", thorWav("CQ CQ DE BG1AAA 599"));
  await expect(page.getByRole("alert")).toContainText("中心频率越界", { timeout: 20000 });
});

test("THOR 解码：纯噪声与空文件如实报错而非乱码", async ({ page }) => {
  await page.goto("/thor-decode");
  let seed = 0x12345678;
  const noise = Array.from({ length: 8000 * 20 }, () => {
    seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
    return (seed >>> 8) / 8388608 - 1;
  });
  await upload(page, "noise.wav", pcmWav(noise, 8000));
  await expect(page.getByRole("alert")).toContainText("未识别出有效的 THOR 信号", { timeout: 30000 });

  await page.reload();
  await upload(page, "empty.wav", pcmWav([], 8000));
  await expect(page.getByRole("alert")).toBeVisible({ timeout: 20000 });
  await expect(page.locator("pre")).toHaveCount(0);
});
