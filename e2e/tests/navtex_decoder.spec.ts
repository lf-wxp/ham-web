import { expect, test } from "./fixtures";
import { R2F, R2L } from "./navtex_varicode_data";

/**
 * 独立 TS 实现：NAVTEX（SITOR-B）编码器 —— 第三套实现，与 Rust 核心互不共享代码。
 * CCIR 476 码表取自 `navtex_varicode_data.ts`；采用 LSB 位序、五字符延迟、
 * REP/ALPHA 相位和标准报头。合成测试不代表真实弱信号全文互通已验证。
 * 页面 wasm 解码必须还原这里编码的报文。
 */

const MARK = 1904.0;
const SPACE = 2104.0;
const RATE = 8000;
const SPB = 80;

const LTRS = 0x5a;
const FIGS = 0x36;
const ALPHA = 0x0f;
const REP = 0x66;

const figureChars = new Set(Object.values(R2F));

function charCode(ch: string): number {
  if (ch === "\r") return 0x78;
  if (ch === "\n") return 0x6c;
  const letters = Object.entries(R2L).find(([, v]) => v === ch);
  if (letters) return Number(letters[0]);
  const figs = Object.entries(R2F).find(([, v]) => v === ch);
  if (figs) return Number(figs[0]);
  return LTRS; // 未收录 → 字母换挡（不会改变换挡状态）
}

function push(bits: number[], code: number) {
  for (let j = 0; j < 7; j += 1) bits.push((code >> j) & 1);
}

/** 标准报文码流，含空格、FIGS 序号及 CR/LF；不复用 Rust 生成器。 */
function messageCodes(b1: string, b2: string, serial: number, text: string): number[] {
  const codes: number[] = [LTRS];
  let figs = false;
  for (const ch of `ZCZC ${b1}${b2}${String(serial).padStart(2, "0")}\r\n${text}\r\nNNNN`) {
    const wantFigs = !Object.values(R2L).includes(ch) && figureChars.has(ch);
    if (wantFigs !== figs) {
      codes.push(wantFigs ? FIGS : LTRS);
      figs = wantFigs;
    }
    codes.push(charCode(ch));
  }
  return codes;
}

/** 文本 → 16 位 PCM WAV（NAVTEX 100 波特 FSK @ 8 kHz，相位段 + 双副本）。 */
function navtexWav(text: string, phasingSecs = 1.2): Buffer {
  const bits: number[] = [];
  const phasingPairs = Math.max(3, Math.floor((phasingSecs * 100) / 14));
  for (let i = 0; i < phasingPairs; i += 1) {
    push(bits, REP);
    push(bits, ALPHA);
  }
  const codes = messageCodes("F", "A", 79, text);
  for (let i = 0; i < codes.length + 2; i += 1) {
    push(bits, codes[i] ?? 0x6a);
    push(bits, i >= 2 ? (codes[i - 2] ?? ALPHA) : ALPHA);
  }

  const samples: number[] = [];
  for (const b of bits) {
    const f = b ? MARK : SPACE;
    for (let i = 0; i < SPB; i += 1) {
      const t = samples.length / RATE;
      samples.push(Math.sin((2 * Math.PI * f * t) / 1));
    }
  }
  const data = Buffer.alloc(samples.length * 2);
  samples.forEach((s, i) => {
    data.writeInt16LE(Math.max(-32767, Math.min(32767, Math.round(s * 32000))), i * 2);
  });
  const head = Buffer.alloc(44);
  head.write("RIFF", 0);
  head.writeUInt32LE(36 + data.length, 4);
  head.write("WAVEfmt ", 8);
  head.writeUInt32LE(16, 16);
  head.writeUInt16LE(1, 20);
  head.writeUInt16LE(1, 22);
  head.writeUInt32LE(RATE, 24);
  head.writeUInt32LE(RATE * 2, 28);
  head.writeUInt16LE(2, 32);
  head.writeUInt16LE(16, 34);
  head.write("data", 36);
  head.writeUInt32LE(data.length, 40);
  return Buffer.concat([head, data]);
}

test("NAVTEX 解码：上传合成录音，解码出报文（FA 79）", async ({ page }) => {
  await page.goto("/navtex-decode");
  await expect(page.getByRole("heading", { name: "NAVTEX 解码器" })).toBeVisible();

  const wav = navtexWav("GALE WARNING IN SOUTHERN ADRIATIC 25 KNOTS");
  await page.locator('input[type="file"]').setInputFiles({
    name: "navtex.wav",
    mimeType: "audio/wav",
    buffer: wav,
  });
  await expect(page.getByText("FA 79")).toBeVisible({ timeout: 30000 });
  await expect(page.getByText(/GALE WARNING IN SOUTHERN ADRIATIC 25 KNOTS/)).toBeVisible();
  await expect(page.getByText("通过").first()).toBeVisible();
});

test("NAVTEX 解码：坏样本给出错误提示而非空白", async ({ page }) => {
  await page.goto("/navtex-decode");
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
  await expect(page.getByText(/文件中没有可解码的音频数据|解码失败/).first()).toBeVisible();
});
