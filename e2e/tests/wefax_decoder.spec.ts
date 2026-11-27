import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, pickOption, test, type Page } from "./fixtures";

/**
 * 在测试里合成一段 WEFAX 音频（WAV）—— 不依赖仓库的 Rust 样本生成器，
 * 编码与解码分属两套实现，能过才是真解码。
 *
 * 参数与 crates/wefax/src/lib.rs 一致：AM 视频（幅度 = 亮度）、8000 Hz、
 * 2 行/秒（120 LPM）、每行同步脉冲 200 样本 + 380 像素 × 10 样本。
 */
function wefaxWav(height = 16): Buffer {
  const RATE = 8000;
  const LINE = 4000; // 0.5 s
  const SYNC = 200; // 5%
  const PIXELS = 380;
  const PP = 10;
  const samples: number[] = [];
  for (let row = 0; row < height; row += 1) {
    for (let i = 0; i < SYNC; i += 1) samples.push(1.0);
    for (let col = 0; col < PIXELS; col += 1) {
      const amp = col / PIXELS; // 0..1 渐变
      for (let i = 0; i < PP; i += 1) samples.push(amp);
    }
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

/** 页面默认是真实传真格式；这些用例用的是简化模型的合成录音，先把格式切过去。 */
async function useSimplified(page: Page) {
  await pickOption(page, "格式", "简化模型（自拟口径，仅合成录音）");
}

test("WEFAX 解码：上传合成录音，解码出灰度图", async ({ page }) => {
  await page.goto("/wefax-decode");
  await useSimplified(page);
  await expect(page.getByRole("heading", { name: "WEFAX 解码器" })).toBeVisible();

  await page.setInputFiles('input[type="file"]', {
    name: "wefax.wav",
    mimeType: "audio/wav",
    buffer: wefaxWav(16),
  });

  // Worker 后台解码后，结果区显示 380 × 16 的尺寸与图像。wasm 首次加载较慢，放宽到 30s。
  await expect(page.getByText(/380 × 16/)).toBeVisible({ timeout: 30000 });
  await expect(page.getByRole("img", { name: "WEFAX 解码图像" })).toBeVisible();
});

test("WEFAX 解码：坏样本给出错误提示而非空白", async ({ page }) => {
  await page.goto("/wefax-decode");
  await page.setInputFiles('input[type="file"]', {
    name: "empty.wav",
    mimeType: "audio/wav",
    buffer: wefaxWav(0).subarray(0, 44), // 只有 WAV 头
  });
  await expect(page.getByText("解码失败").first()).toBeVisible();
});

// ---- 调频副载波（真实短波传真的口径：1500 Hz 黑、2300 Hz 白）----

const W = 380;
const H = 6;
/** 每行都有硬边：全档灰阶、黑白交替、整行白、整行黑、伪随机、成块黑白（与核心单测同一套图案，各自独立写的）。 */
function grayAt(row: number, col: number): number {
  switch (row) {
    case 0:
      return Math.floor((col * 255) / (W - 1));
    case 1:
      return col % 2 === 0 ? 0 : 255;
    case 2:
      return 255;
    case 3:
      return 0;
    case 4:
      return (col * 37 + 11) % 256;
    default:
      return Math.floor(col / 19) % 2 === 0 ? 255 : 0;
  }
}

function wav16(samples: number[], rate = 8000): Buffer {
  const dataLen = samples.length * 2;
  const out = Buffer.alloc(44 + dataLen);
  out.write("RIFF", 0);
  out.writeUInt32LE(36 + dataLen, 4);
  out.write("WAVEfmt ", 8);
  out.writeUInt32LE(16, 16);
  out.writeUInt16LE(1, 20);
  out.writeUInt16LE(1, 22);
  out.writeUInt32LE(rate, 24);
  out.writeUInt32LE(rate * 2, 28);
  out.writeUInt16LE(2, 32);
  out.writeUInt16LE(16, 34);
  out.write("data", 36);
  out.writeUInt32LE(dataLen, 40);
  samples.forEach((v, i) => out.writeInt16LE(Math.round(v * 32767), 44 + i * 2));
  return out;
}

/**
 * 独立写的调频发端：相位连续，每行 200 样本的黑（1500 Hz）同步 + 380 个像素 × 10 样本，
 * 像素亮度 g（0 白 … 255 黑）对应 2300 − 800·g/255 Hz。`shiftHz` 模拟接收机调谐不准。
 */
function fmWav(shiftHz = 0, amplitude = 0.8): Buffer {
  const out: number[] = [];
  let phase = 0;
  const emit = (hz: number, n: number) => {
    for (let i = 0; i < n; i += 1) {
      out.push(amplitude * Math.sin(phase));
      phase += (2 * Math.PI * hz) / 8000;
    }
  };
  for (let row = 0; row < H; row += 1) {
    emit(1500 + shiftHz, 200);
    for (let col = 0; col < W; col += 1) emit(2300 + shiftHz - (800 * grayAt(row, col)) / 255, 10);
  }
  return wav16(out);
}

/** 把结果区的图读回成灰度数组（0 = 白、255 = 黑，与发端一致；PNG 同源，canvas 不会被污染）。 */
async function shownPixels(page: Page): Promise<{ w: number; h: number; gray: number[] }> {
  return page.evaluate(async () => {
    const img = document.querySelector("section img") as HTMLImageElement;
    await img.decode();
    const canvas = document.createElement("canvas");
    canvas.width = img.naturalWidth;
    canvas.height = img.naturalHeight;
    const ctx = canvas.getContext("2d")!;
    ctx.drawImage(img, 0, 0);
    const d = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
    return {
      w: canvas.width,
      h: canvas.height,
      // 屏幕上白纸是白的（255）；解码器的灰度是「0 = 白、255 = 黑」，这里换回去再比。
      gray: Array.from({ length: canvas.width * canvas.height }, (_, i) => 255 - d[i * 4]),
    };
  });
}

const expectedGray = () =>
  Array.from({ length: W * H }, (_, i) => grayAt(Math.floor(i / W), i % W));

async function upload(page: Page, buffer: Buffer, name = "fax.wav") {
  await page.setInputFiles('input[type="file"]', { name, mimeType: "audio/wav", buffer });
}

// ---- 真实传真格式（连续的调频副载波、IOC 576 / 288、相位线、没有逐行同步）----

/** 与核心单测同一幅图的独立写法：渐变底、粗黑块、每 150 列一条 3 像素细竖线、白块。 */
function chartGray(row: number, col: number, width: number): number {
  const x = col / width;
  if ((row >= 8 && row < 16 && x >= 0.3 && x < 0.5) || col % 150 < 3) return 255;
  if (row >= 22 && row < 28 && x >= 0.6 && x < 0.8) return 0;
  return Math.floor(x * 170) + 20;
}

type FaxOptions = {
  lpm?: number;
  width?: number; // 1810（IOC 576）或 905（IOC 288）
  lines?: number;
  phasing?: number; // 相位线条数；0 = 不发（模拟截取的片段）
  clock?: number; // 发端时钟相对标称的比值
  shift?: number; // 接收机调谐偏差（Hz）
  black?: number;
  white?: number;
  rate?: number;
};

/**
 * 独立写的真实格式发端（不经过被测代码）：1 s 起始音、若干条相位线（每行开头 5% 的白脉冲，其余黑）、
 * 图像各行连续。相位连续，逐像素取常频。时长累计到全局时间轴再取整，行周期不会漂。
 */
function faxWav(o: FaxOptions = {}): Buffer {
  const { lpm = 120, width = 1810, lines = 40, phasing = 20, clock = 1, shift = 0, black = 1500, white = 2300, rate = 11025 } = o;
  const lineS = 60 / lpm / clock;
  const out: number[] = [];
  let phase = 0;
  let t = 0;
  const tone = (hz: number, seconds: number) => {
    t += seconds;
    const end = Math.round(t * rate);
    const step = (2 * Math.PI * (hz + shift)) / rate;
    while (out.length < end) {
      out.push(0.8 * Math.sin(phase));
      phase += step;
    }
  };
  tone(width === 1810 ? 300 : 675, 1);
  for (let k = 0; k < phasing; k += 1) {
    tone(white, 0.05 * lineS);
    tone(black, 0.95 * lineS);
  }
  for (let row = 0; row < lines; row += 1) {
    for (let col = 0; col < width; col += 1) {
      const g = chartGray(row, col, width);
      tone(white - ((white - black) * g) / 255, lineS / width);
    }
  }
  tone(450, 0.5);
  return wav16(out, rate);
}

/** 解出的图与发端的图（前 `lines` 行）的平均绝对误差。 */
function meanError(gray: number[], width: number, lines: number): number {
  let sum = 0;
  for (let i = 0; i < width * lines; i += 1) sum += Math.abs(gray[i] - chartGray(Math.floor(i / width), i % width, width));
  return sum / (width * lines);
}

function regionMean(gray: number[], width: number, rows: [number, number], cols: [number, number]): number {
  let sum = 0;
  let n = 0;
  for (let r = rows[0]; r < rows[1]; r += 1) for (let c = cols[0]; c < cols[1]; c += 1) (sum += gray[r * width + c]), (n += 1);
  return sum / n;
}

test("WEFAX 解码：真实传真格式（默认）——相位线、行首与图像都对，图与发端的图一致", async ({ page }) => {
  await page.goto("/wefax-decode");
  await upload(page, faxWav());
  await expect(page.getByText(/1810 × 40/)).toBeVisible({ timeout: 30000 });
  await expect(page.getByTestId("wefax-used")).toContainText("按真实传真格式解调");
  await expect(page.getByTestId("wefax-used")).toContainText("取自相位线");
  await expect(page.getByTestId("wefax-phasing")).toHaveAttribute("data-lines", "20");
  const shown = await shownPixels(page);
  expect([shown.w, shown.h]).toEqual([1810, 40]);
  // 每个像素只有 3 个样本，边缘会被解调抹开几个像素：平均误差几个灰阶，粗黑块与白块的内部分得开。
  expect(meanError(shown.gray, 1810, 40)).toBeLessThan(5);
  expect(regionMean(shown.gray, 1810, [9, 15], [600, 880])).toBeGreaterThan(245);
  expect(regionMean(shown.gray, 1810, [23, 27], [1130, 1420])).toBeLessThan(10);
  // 找到了相位线，行首已知，不给「转正」滑块。
  await expect(page.getByRole("slider")).toHaveCount(0);
});

test("WEFAX 解码：屏幕上白底是白的、黑块是黑的（不是底片）", async ({ page }) => {
  await page.goto("/wefax-decode");
  await upload(page, faxWav());
  await expect(page.getByText(/1810 × 40/)).toBeVisible({ timeout: 30000 });
  // 直接读屏幕上的颜色（不经过 shownPixels 的换算）：粗黑块（图里第 9–15 行、第 600–880 列）要接近黑，
  // 白块（第 23–27 行、第 1130–1420 列）要接近白。
  const [black, white] = await page.evaluate(async () => {
    const img = document.querySelector("section img") as HTMLImageElement;
    await img.decode();
    const canvas = document.createElement("canvas");
    canvas.width = img.naturalWidth;
    canvas.height = img.naturalHeight;
    const ctx = canvas.getContext("2d")!;
    ctx.drawImage(img, 0, 0);
    const at = (x: number, y: number) => ctx.getImageData(x, y, 1, 1).data[0];
    return [at(740, 12), at(1275, 25)];
  });
  expect(black, "黑块应显示成黑").toBeLessThan(20);
  expect(white, "白块应显示成白").toBeGreaterThan(235);
});

test("WEFAX 解码：发收时钟差 0.3% 时量出实际行速率，图不斜", async ({ page }) => {
  await page.goto("/wefax-decode");
  await upload(page, faxWav({ clock: 1.003 }));
  // 末尾 0.5 s 的停止音恰好是一行，行数 40 或 41 都对（取决于取整）。
  await expect(page.getByText(/1810 × (40|41)/)).toBeVisible({ timeout: 30000 });
  const text = (await page.getByTestId("wefax-used").textContent()) ?? "";
  const measured = Number(/实测行速率 ([\d.]+) LPM/.exec(text)?.[1]);
  expect(Math.abs(measured - 120 * 1.003)).toBeLessThan(0.02);
  // 不修斜率，40 行会错开 40 × 0.3% × 1810 ≈ 220 个像素，误差会大得多。
  expect(meanError((await shownPixels(page)).gray, 1810, 40)).toBeLessThan(5);
});

test("WEFAX 解码：接收机失谐 / 带宽被削窄时黑白电平取自相位线", async ({ page }) => {
  for (const [shift, black, white] of [
    [80, 1500, 2300],
    [0, 1725, 2095],
  ]) {
    await page.goto("/wefax-decode");
    await upload(page, faxWav({ shift, black, white }));
    await expect(page.getByText(/1810 × 40/)).toBeVisible({ timeout: 30000 });
    const text = (await page.getByTestId("wefax-used").textContent()) ?? "";
    expect(text, `${shift}/${black}/${white}`).toContain("取自相位线");
    const [b, w] = [/黑电平 (\d+) Hz/, /白电平 (\d+) Hz/].map((re) => Number(re.exec(text)?.[1]));
    expect(Math.abs(b - (black + shift)), "黑电平").toBeLessThan(5);
    expect(Math.abs(w - (white + shift)), "白电平").toBeLessThan(5);
    expect(meanError((await shownPixels(page)).gray, 1810, 40)).toBeLessThan(5);
  }
});

test("WEFAX 解码：行速率与 IOC 选项都起作用（页面上的六档行速率与 worker 认的一致）", async ({ page }) => {
  for (const lpm of [60, 90, 100, 120, 180, 240]) {
    await page.goto("/wefax-decode");
    await pickOption(page, "行速率（LPM）", `${lpm} LPM`);
    // 相位线要持续够 8 s 才采信：按行速率折算条数。
    await upload(page, faxWav({ lpm, phasing: Math.ceil((10 * lpm) / 60), lines: 30 }));
    await expect(page.getByText(/1810 × (3\d|4\d)/)).toBeVisible({ timeout: 30000 });
    const text = (await page.getByTestId("wefax-used").textContent()) ?? "";
    const measured = Number(/实测行速率 ([\d.]+) LPM/.exec(text)?.[1]);
    expect(Math.abs(measured - lpm), `${lpm} LPM`).toBeLessThan(lpm * 2e-4);
  }
  // 选错行速率：同一段 60 LPM 的录音按默认的 120 LPM 解，行数对不上。
  await page.goto("/wefax-decode");
  await upload(page, faxWav({ lpm: 60, phasing: 10, lines: 30 }));
  await expect(page.getByTestId("wefax-used")).toBeVisible({ timeout: 30000 });
  await expect(page.getByText(/1810 × 30/)).toHaveCount(0);
  // IOC 288：每行 905 个像素。
  await page.goto("/wefax-decode");
  await pickOption(page, "合作指数（IOC）", "IOC 288");
  await upload(page, faxWav({ width: 905 }));
  await expect(page.getByText(/905 × 40/)).toBeVisible({ timeout: 30000 });
});

test("WEFAX 解码：没有相位线时提示行首位置没有依据，滑块把图转正（不重新解码）", async ({ page }) => {
  await page.goto("/wefax-decode");
  await upload(page, faxWav({ phasing: 0 }));
  await expect(page.getByTestId("wefax-phasing")).toHaveAttribute("data-lines", "0", { timeout: 30000 });
  await expect(page.getByTestId("wefax-phasing")).toContainText("没有找到相位线");
  const slider = page.getByRole("slider", { name: "行首位置" });
  await expect(slider).toBeVisible();
  const track = page.locator("div.flex.w-\\[200\\%\\]");
  // 滑块在 0 时不平移（`translateX(-0%)` 算出来是单位矩阵）。
  await expect(track).toHaveCSS("transform", "matrix(1, 0, 0, 1, 0, 0)");
  await slider.fill("50");
  // 平移 = 滑块百分比的一半（内层宽度是图宽的 200%）：50% → 25% 内层宽。
  const matrix = await track.evaluate((el) => getComputedStyle(el).transform);
  const tx = Number(/matrix\(1, 0, 0, 1, (-?[\d.]+), 0\)/.exec(matrix)?.[1]);
  const inner = await track.evaluate((el) => el.getBoundingClientRect().width);
  expect(Math.abs(tx + inner * 0.25)).toBeLessThan(1);
  // 并排的两份图只有第一份有替代文字，读屏不会念两遍。
  await expect(page.getByRole("img", { name: "WEFAX 解码图像" })).toHaveCount(1);
  await expect(page.locator('section img[aria-hidden="true"]')).toHaveCount(1);
});

test("WEFAX 解码：真实录音——Northwood 的相位线与 DWD 气象图的电平、行速率（经 worker 与页面）", async ({ page }) => {
  const sample = (name: string) => readFileSync(join(__dirname, "../../crates/wefax/samples", name));
  await page.goto("/wefax-decode");
  await upload(page, sample("northwood_intro_8k.wav"), "northwood.wav");
  await expect(page.getByTestId("wefax-phasing")).toBeVisible({ timeout: 30000 });
  const lines = Number(await page.getByTestId("wefax-phasing").getAttribute("data-lines"));
  expect(lines).toBeGreaterThanOrEqual(35);
  await expect(page.getByTestId("wefax-used")).toContainText("取自相位线");

  // DWD：接收机带宽被削窄（黑 ≈ 1747、白 ≈ 2051 Hz），发收时钟差 0.18%（120.22 LPM）。
  await page.goto("/wefax-decode");
  await upload(page, sample("dwd_chart_8k.wav"), "dwd.wav");
  await expect(page.getByTestId("wefax-used")).toBeVisible({ timeout: 30000 });
  const text = (await page.getByTestId("wefax-used").textContent()) ?? "";
  expect(Math.abs(Number(/实测行速率 ([\d.]+) LPM/.exec(text)?.[1]) - 120.22)).toBeLessThan(0.03);
  expect(Math.abs(Number(/黑电平 (\d+) Hz/.exec(text)?.[1]) - 1747)).toBeLessThan(20);
  expect(Math.abs(Number(/白电平 (\d+) Hz/.exec(text)?.[1]) - 2051)).toBeLessThan(20);
  expect(text).toContain("取自相位线");
});

test("WEFAX 解码：噪声与静音给出失败提示，不画出一幅假图", async ({ page }) => {
  let state = 7;
  const noise = Array.from({ length: 11025 * 10 }, () => {
    state = (state * 1103515245 + 12345) & 0x7fffffff;
    return (state / 0x7fffffff) * 1.6 - 0.8;
  });
  for (const samples of [noise, new Array(11025 * 10).fill(0)]) {
    await page.goto("/wefax-decode");
    await upload(page, wav16(samples));
    await expect(page.getByText("解码失败").first()).toBeVisible({ timeout: 30000 });
    await expect(page.locator("section img")).toHaveCount(0);
  }
});

test("WEFAX 解码：调频副载波自动判断，解出的图逐像素等于原图（含黑白硬边）", async ({ page }) => {
  await page.goto("/wefax-decode");
  await useSimplified(page);
  await upload(page, fmWav());
  await expect(page.getByText(/380 × 6/)).toBeVisible({ timeout: 30000 });
  await expect(page.getByTestId("wefax-used")).toContainText("调频副载波");
  const shown = await shownPixels(page);
  expect([shown.w, shown.h]).toEqual([W, H]);
  expect(shown.gray, "每个像素都应与发端的灰度一致").toEqual(expectedGray());
});

test("WEFAX 解码：调幅视频仍按调幅解，并说明用了哪种", async ({ page }) => {
  await page.goto("/wefax-decode");
  await useSimplified(page);
  await upload(page, wefaxWav(16));
  await expect(page.getByText(/380 × 16/)).toBeVisible({ timeout: 30000 });
  await expect(page.getByTestId("wefax-used")).toContainText("调幅视频");
});

test("WEFAX 解码：接收机调谐差 ±50 Hz 与录音电平很低时仍逐像素一致", async ({ page }) => {
  for (const [shift, amplitude] of [
    [50, 0.8],
    [-50, 0.8],
    [0, 0.04],
  ]) {
    await page.goto("/wefax-decode");
    await useSimplified(page);
    await upload(page, fmWav(shift, amplitude));
    await expect(page.getByText(/380 × 6/)).toBeVisible({ timeout: 30000 });
    const shown = await shownPixels(page);
    expect(shown.gray, `调谐偏差 ${shift} Hz、幅度 ${amplitude}`).toEqual(expectedGray());
  }
});

test("WEFAX 解码：手选的解调方式说了算——调频录音按调幅解、调幅录音按调频解都给出失败提示", async ({ page }) => {
  await page.goto("/wefax-decode");
  await useSimplified(page);
  await pickOption(page, "解调方式", "调幅视频（幅度 = 亮度）");
  await upload(page, fmWav());
  await expect(page.getByText("解码失败").first()).toBeVisible({ timeout: 30000 });
  await expect(page.locator("section img")).toHaveCount(0);

  await page.goto("/wefax-decode");
  await useSimplified(page);
  await pickOption(page, "解调方式", "调频副载波（1500 / 2300 Hz）");
  await upload(page, wefaxWav(16));
  await expect(page.getByText("解码失败").first()).toBeVisible({ timeout: 30000 });
  await expect(page.locator("section img")).toHaveCount(0);
});

test("WEFAX 解码：手选与录音一致时照常解出（调频手选）", async ({ page }) => {
  await page.goto("/wefax-decode");
  await useSimplified(page);
  await pickOption(page, "解调方式", "调频副载波（1500 / 2300 Hz）");
  await upload(page, fmWav());
  await expect(page.getByText(/380 × 6/)).toBeVisible({ timeout: 30000 });
  expect((await shownPixels(page)).gray).toEqual(expectedGray());
});

test("WEFAX 解码：英文 / 西语界面都有译文，不留中文", async ({ page }) => {
  for (const code of ["en", "es"]) {
    await page.addInitScript((c) => localStorage.setItem("locale", c), code);
    // 上传前：格式、行速率、IOC 的标签与提示。
    await page.goto("/wefax-decode");
    await expect(page.locator("main")).not.toContainText(/[\u4e00-\u9fff]/);
    // 有相位线与没有相位线两种结果各验一遍（后者多一个滑块与提示）。
    for (const phasing of [20, 0]) {
      await page.goto("/wefax-decode");
      await upload(page, faxWav({ phasing }), "fax.wav");
      await expect(page.getByTestId("wefax-used")).toBeVisible({ timeout: 30000 });
      await expect(page.locator("main")).not.toContainText(/[\u4e00-\u9fff]/);
    }
  }
});

/** 整页与卡片各自的横向溢出（像素，≤ 1 视为没有溢出）。 */
async function overflowOf(page: Page) {
  // 现代风点按钮会放一圈 560 ms 的按压涟漪（`.md-ping`），动画放大时短暂伸出视口，之后自清理。
  await expect(page.locator(".md-ping")).toHaveCount(0);
  return page.evaluate(() => {
    const el = document.querySelector("main section") as HTMLElement;
    return {
      page: document.documentElement.scrollWidth - window.innerWidth,
      section: el.scrollWidth - el.clientWidth,
    };
  });
}

test("WEFAX 解码：溢出检测器自测——塞进 900px 宽的元素时必须量得出溢出", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 800 });
  await page.goto("/wefax-decode");
  const before = await overflowOf(page);
  expect(before.page).toBeLessThanOrEqual(1);
  expect(before.section).toBeLessThanOrEqual(1);
  await page.evaluate(() => {
    const wide = document.createElement("div");
    wide.style.cssText = "width:900px;height:4px";
    document.querySelector("main section")!.appendChild(wide);
  });
  const after = await overflowOf(page);
  expect(after.section).toBeGreaterThan(100);
  expect(after.page).toBeGreaterThan(100);
});

// 三语里最长的说明与下拉选项在 320 px 下不能把整页撑出横向滚动；两套皮肤各量一次（上传前与出结果后各一次）。
for (const lang of ["zh", "en", "es"]) {
  for (const style of ["pixel", "modern"]) {
    test(`WEFAX 解码：320px 下不横向溢出（${lang} · ${style}）`, async ({ page }) => {
      await page.setViewportSize({ width: 320, height: 800 });
      await page.addInitScript(
        ([code, ui]) => {
          localStorage.setItem("locale", code);
          localStorage.setItem("ui:uiStyle", ui);
          if (ui === "modern") localStorage.setItem("theme", "dark");
        },
        [lang, style],
      );
      await page.goto("/wefax-decode");
      let over = await overflowOf(page);
      expect(over.page, "上传前整页横向溢出").toBeLessThanOrEqual(1);
      // 没有相位线的结果：多一行说明与一个「行首位置」滑块，是版面最挤的一种。
      await upload(page, faxWav({ phasing: 0 }));
      await expect(page.getByTestId("wefax-used")).toBeVisible({ timeout: 30000 });
      over = await overflowOf(page);
      expect(over.page, "出结果后整页横向溢出").toBeLessThanOrEqual(1);
    });
  }
}
