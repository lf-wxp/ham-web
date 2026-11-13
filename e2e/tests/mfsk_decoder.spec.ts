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
  /** 长交织变体（64L / 128L）：发端不发 `preamble / 3` 个前导零位。 */
  long?: boolean;
}

const MODES: Record<string, ModeSpec> = {
  "16": { rate: 8000, symlen: 512, symbits: 4, depth: 10, preamble: 107 },
  "32": { rate: 8000, symlen: 256, symbits: 4, depth: 10, preamble: 107 },
  "64L": { rate: 8000, symlen: 128, symbits: 4, depth: 400, preamble: 2500, long: true },
};

const parity = (x: number) => {
  let p = 0;
  for (let v = x; v; v >>= 1) p ^= v & 1;
  return p;
};

/**
 * 文本 → 一帧完整的音调号序列（已过格雷码映射）。`eot = false` 不发 `\r EOT \r`：图片发射在报头之后直接冲刷、
 * 接序言与图像（fldigi `tx_process` 的 `TX_STATE_PICTURE_START` 路径）。
 */
function mfskTones(text: string, m: ModeSpec, eot = true): number[] {
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
  if (!m.long) for (let i = 0; i < Math.floor(m.preamble / 3); i += 1) sendBit(0);
  for (const c of [13, 2, 13]) sendCode(MFSK_VARICODE[c]);
  for (const ch of new TextEncoder().encode(text)) sendCode(MFSK_VARICODE[ch]);
  if (eot) for (const c of [13, 4, 13]) sendCode(MFSK_VARICODE[c]);
  // 收尾：先一个 1 冲掉对端 Varicode，再 `preamble` 个零冲卷积码与交织器。
  sendBit(1);
  for (let i = 0; i < m.preamble; i += 1) sendBit(0);
  return tones;
}

/**
 * 文本 → 16 位 PCM WAV（8 kHz）。`reverse` 把音调序倒过来，模拟下边带录音；
 * `clockPpm` 把整段采样线性拉伸（正 = 录音时钟偏慢、符号变长），模拟录音端时钟与发射端不一致。
 */
function mfskWav(
  text: string,
  opts: { mode?: string; center?: number; reverse?: boolean; clockPpm?: number; carrier?: number; noise?: number } = {},
): Buffer {
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
  const timed = opts.clockPpm ? stretch(samples, opts.clockPpm) : samples;
  return pcmWav(
    opts.carrier || opts.noise ? impair(timed, m.rate, center, opts.carrier ?? 0, opts.noise ?? 0) : timed,
    m.rate,
  );
}

/**
 * 一次 MFSK16 图片发射（fldigi 的结构）：文本 + `Pic:宽x高C;` 报头 → 冲刷 → 352 个样本的序言（最低音调）→
 * FM 图像（每像素 8 个样本，每行 R、G、B 三遍，频率 = 中心 + 带宽 × (字节 − 128) / 256）→ 一小段最低音调。
 * `rgb` 是宽 × 高 × 3 字节。
 */
function mfskPictureWav(text: string, width: number, height: number, rgb: Uint8Array, center = 1500): Buffer {
  const m = MODES["16"];
  const baud = m.rate / m.symlen;
  const bandwidth = ((1 << m.symbits) - 1) * baud;
  const low = center - bandwidth / 2;
  const samples: number[] = [];
  let phase = 0;
  const tone = (freq: number, count: number) => {
    const step = (2 * Math.PI * freq) / m.rate;
    for (let i = 0; i < count; i += 1) {
      samples.push(Math.cos(phase));
      phase += step;
    }
  };
  for (const t of mfskTones(`${text}Pic:${width}x${height}C;`, m, false)) tone(low + t * baud, m.symlen);
  tone(low, 352);
  for (let y = 0; y < height; y += 1) {
    for (let plane = 0; plane < 3; plane += 1) {
      for (let x = 0; x < width; x += 1) {
        tone(center + (bandwidth * (rgb[3 * (y * width + x) + plane] - 128)) / 256, 8);
      }
    }
  }
  tone(low, 2000);
  return pcmWav(samples, m.rate);
}

/** 线性插值拉伸：新长度 = 旧长度 × (1 + ppm·1e-6)，输出第 i 点取原序列的 i / (1 + ppm·1e-6) 处。 */
function stretch(samples: number[], ppm: number): number[] {
  const k = 1 + ppm * 1e-6;
  const out = new Array<number>(Math.floor((samples.length - 1) * k));
  for (let i = 0; i < out.length; i += 1) {
    const x = i / k;
    const j = Math.floor(x);
    const f = x - j;
    out[i] = samples[j] * (1 - f) + samples[Math.min(j + 1, samples.length - 1)] * f;
  }
  return out;
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

test("MFSK 解码：上传合成录音，自动检测中心频率并还原原文（默认 MFSK16）", async ({ page }) => {
  await page.goto("/mfsk-decode");
  await expect(page.getByRole("heading", { name: "MFSK 解码器" })).toBeVisible();

  // 1337.5 Hz 不在任何整数音调边界上：自动检测要把它找回来。
  await upload(page, "mfsk16.wav", mfskWav("THE QUICK BROWN FOX JUMPS OVER 1234567890", { center: 1337.5 }));
  await expect(page.locator("pre")).toContainText("THE QUICK BROWN FOX JUMPS OVER 1234567890", { timeout: 30000 });
  await expect(page.getByText(/中心频率 1337\.\d Hz/)).toBeVisible();
});

test("MFSK 解码：干净录音不出现「已修正」一行，时钟偏慢的长录音会报出修正量", async ({ page }) => {
  // 跟踪要求有信号的部分不少于约 190 个符号（MFSK16 约 12 秒），所以用一段长报文。
  const text = Array.from({ length: 6 }, () => "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG 0123456789").join(" ");
  await page.goto("/mfsk-decode");

  await upload(page, "clean.wav", mfskWav(text, { center: 1200 }));
  await expect(page.locator("pre")).toContainText("THE QUICK BROWN FOX", { timeout: 60000 });
  // 自动检测落在 1/8 个音调间隔内，可能是 1199.x 也可能是 1200.x；这里只要「结果行已出现」。
  await expect(page.getByText(/中心频率 (1199|1200)\.\d Hz/)).toBeVisible();
  await expect(page.getByTestId("decode-tracking")).toHaveCount(0);

  await page.reload();
  await upload(page, "slow.wav", mfskWav(text, { center: 1200, clockPpm: 1000 }));
  await expect(page.locator("pre")).toContainText("THE QUICK BROWN FOX", { timeout: 60000 });
  // 注入 +1000 ppm；对拍里估值与注入值相差 < 150 ppm。
  const note = page.getByTestId("decode-tracking");
  await expect(note).toContainText("已修正录音时钟误差");
  const ppm = Number(/([+-]\d+) ppm/.exec((await note.textContent()) ?? "")?.[1]);
  expect(ppm).toBeGreaterThan(850);
  expect(ppm).toBeLessThan(1150);
});

test("MFSK 解码：很大的时钟误差（−12000 ppm）的短录音也能还原，报出的误差与注入值一致", async ({ page }) => {
  // 只有约 190 个符号：块缩短后才够块数去跟踪；估出大误差后把录音重采样回名义时钟再解一遍。
  await page.goto("/mfsk-decode");
  await upload(page, "fast.wav", mfskWav("CQ CQ DE BG1AAA K ".repeat(3), { center: 1200, clockPpm: -12000 }));
  await expect(page.locator("pre")).toContainText("CQ CQ DE BG1AAA K CQ CQ DE BG1AAA K CQ CQ DE BG1AAA K", { timeout: 60000 });
  const note = page.getByTestId("decode-tracking");
  await expect(note).toContainText("已修正录音时钟误差");
  const ppm = Number(/([+-]\d+) ppm/.exec((await note.textContent()) ?? "")?.[1]);
  expect(ppm).toBeGreaterThan(-12400);
  expect(ppm).toBeLessThan(-11600);
  // 中心频率按录音自己的轴报：1200 / (1 − 0.012) ≈ 1214.6 Hz。
  await expect(page.getByText(/中心频率 121[3-6]\.\d Hz/)).toBeVisible();
});

test("MFSK 解码：长交织变体 MFSK64L（不给中心频率，正文前后没有乱码）", async ({ page }) => {
  await page.goto("/mfsk-decode");
  await pickOption(page, "MFSK 子模式", "MFSK64L");
  // 64L 只用到 5 个音调、九成以上是最低音调，粗检测会偏好几个音调；1337.5 Hz 也不在任何整数音调边界上。
  await upload(page, "mfsk64l.wav", mfskWav("CQ CQ DE BG1AAA 599", { mode: "64L", center: 1337.5 }));
  await expect(page.locator("pre")).toHaveText(/^\s*CQ CQ DE BG1AAA 599\s*$/, { timeout: 60000 });
  // 检测精度是 1/8 个音调间隔（62.5 / 8 ≈ 7.8 Hz），不是恰好 1337.5。
  const line = page.getByText(/中心频率 [\d.]+ Hz/);
  await expect(line).toBeVisible();
  const detected = Number(/中心频率 ([\d.]+) Hz/.exec((await line.textContent()) ?? "")?.[1]);
  expect(Math.abs(detected - 1337.5)).toBeLessThanOrEqual(62.5 / 8);
});

test("MFSK 解码：与信号等幅 / 更强的恒定载波加噪声下仍能还原（fldigi 自己的接收机在这里解不出）", async ({ page }) => {
  const text = Array.from({ length: 6 }, () => "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG 0123456789").join(" ");
  await page.goto("/mfsk-decode");
  // 等幅的载波走「减载波」，比信号强一倍的走「抹掉那个音调位」；两条路径各验一次。
  for (const carrier of [1, 2]) {
    await page.getByLabel(/中心频率/).fill("1500");
    await upload(page, `carrier${carrier}.wav`, mfskWav(text, { center: 1500, carrier, noise: 3 }));
    await expect(page.locator("pre")).toContainText("THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG 0123456789", { timeout: 60000 });
    expect((await page.locator("pre").textContent())?.split("THE QUICK BROWN FOX").length ?? 0).toBeGreaterThan(5);
    await page.getByRole("button", { name: "解码其他文件" }).click();
  }
});

test("MFSK 解码：有图片报头但录音里没有图像时保留正文并提示没解出图片", async ({ page }) => {
  await page.goto("/mfsk-decode");
  // 640 × 480 的图像段要 7 百多万个样本，这段录音里远远没有：不能画一幅几乎全黑的大图。
  await upload(page, "header-only.wav", mfskWav("CQ Pic:640x480C;"));
  await expect(page.locator("pre")).toContainText("CQ Pic:640x480C;", { timeout: 60000 });
  await expect(page.getByTestId("decode-picture")).toHaveCount(0);
  await expect(page.getByTestId("decode-picture-skipped")).toBeVisible();
});

test("MFSK 解码：图片模式（Pic:…; 报头 + 模拟调频图像）显示出图片，正文里没有图像段的乱码", async ({ page }) => {
  const [w, h] = [32, 24];
  const rgb = new Uint8Array(w * h * 3);
  for (let y = 0; y < h; y += 1) {
    for (let x = 0; x < w; x += 1) {
      rgb.set([x < w / 2 ? 220 : 40, y < h / 2 ? 180 : 70, x < w / 2 ? 30 : 210], 3 * (y * w + x));
    }
  }
  await page.goto("/mfsk-decode");
  // 不给中心频率：自动检测；图片发射的 MFSK 文本在录音开头。
  await upload(page, "picture.wav", mfskPictureWav("CQ ", w, h, rgb));
  const figure = page.getByTestId("decode-picture");
  await expect(figure).toBeVisible({ timeout: 60000 });
  await expect(figure.locator("figcaption")).toContainText("32 × 24");
  await expect(page.locator("pre")).toHaveText(/^\s*CQ Pic:32x24C;\s*$/);
  // 画出来的像素：四个象限中心的颜色与发出去的一致（FM 解调的容差 ±3）。
  const got = await figure.locator("img").evaluate(async (img: HTMLImageElement) => {
    await img.decode();
    const canvas = document.createElement("canvas");
    canvas.width = img.naturalWidth;
    canvas.height = img.naturalHeight;
    const ctx = canvas.getContext("2d")!;
    ctx.drawImage(img, 0, 0);
    return [[8, 6], [24, 6], [8, 18], [24, 18]].map(([x, y]) => Array.from(ctx.getImageData(x, y, 1, 1).data.slice(0, 3)));
  });
  const want = [[220, 180, 30], [40, 180, 210], [220, 70, 30], [40, 70, 210]];
  got.forEach((px, i) => px.forEach((v, c) => expect(Math.abs(v - want[i][c])).toBeLessThanOrEqual(3)));
  await expect(figure.getByRole("link", { name: /下载|PNG/ })).toHaveAttribute("download", "mfsk-picture-1.png");
});

/**
 * 解出图片之后的结果区在手机宽度下不许把整页撑出横向滚动（三语 × 320 / 390 px）。
 * `mobile_overflow` / `i18n_layout` 只扫「还没上传」的路由，扫不到这块：图、图注、下载按钮、「没解出图」说明
 * 都是上传之后才出现的。`E2E_UI_STYLE=modern` 切到现代风（默认像素风），两档各跑一遍，互相不能代替。
 * 128 × 8 的图：放大上限是 min(宽 × 8, 1024) = 1024 px，比任何手机视口都宽，只有 `width: 100%` 能把它收住。
 */
test("MFSK 解码：图片结果在 320 / 390px 下不横向溢出（三语）", async ({ page }) => {
  test.setTimeout(180_000);
  const [w, h] = [128, 8];
  const rgb = new Uint8Array(w * h * 3);
  for (let i = 0; i < w * h; i += 1) rgb.set([(i % w) * 2, 120, 255 - (i % w) * 2], 3 * i);
  const wav = mfskPictureWav("CQ ", w, h, rgb);
  const style = process.env.E2E_UI_STYLE === "modern" ? "modern" : "pixel";
  const overflow = () => page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);

  for (const lang of ["zh", "en", "es"]) {
    await page.goto("/mfsk-decode");
    await page.evaluate(([l, s]) => {
      localStorage.setItem("locale", l);
      localStorage.setItem("ui:uiStyle", s);
      if (s === "modern") localStorage.setItem("theme", "dark");
    }, [lang, style]);
    await page.goto("/mfsk-decode");
    await upload(page, "picture-wide.wav", wav);
    const figure = page.getByTestId("decode-picture");
    await expect(figure).toBeVisible({ timeout: 60000 });
    await expect(figure.locator("img")).toHaveAttribute("src", /^data:image\/png/);
    for (const width of [320, 390]) {
      await page.setViewportSize({ width, height: 900 });
      // 检测器自测：塞一个 2000px 宽的块，量出来必须溢出，否则下面的「0」什么都不证明。
      await page.evaluate(() => {
        const probe = document.createElement("div");
        probe.id = "overflow-probe";
        probe.style.cssText = "width:2000px;height:1px";
        document.body.appendChild(probe);
      });
      expect(await overflow(), `${lang} ${width}px：检测器没有量出故意造的溢出`).toBeGreaterThan(100);
      await page.evaluate(() => document.getElementById("overflow-probe")?.remove());
      expect(await overflow(), `${lang} ${style} ${width}px：图片结果撑出了横向滚动`).toBeLessThanOrEqual(1);
      const box = await figure.locator("img").boundingBox();
      expect(box && box.x >= 0 && box.x + box.width <= width + 1, `${lang} ${width}px：图片越出视口`).toBe(true);
    }
  }
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
