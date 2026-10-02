// PSK31 合成器（测试用）：与 crates/core/src/psk31.rs 的 synthesize 对应，
// 用于在浏览器环境生成可上传的合成 PSK31 WAV，验证 /psk-decode 端到端解码。

const BAUD = 31.25;

// 可打印字符（32–126）的 Varicode 编码表（左位先，数据源 fldigi varicodetab1）。
const VARICODE: Record<string, string> = {
  " ": "1",
  "!": "111111111",
  '"': "101011111",
  "#": "111110101",
  $: "111011011",
  "%": "1011010101",
  "&": "1010111011",
  "'": "101111111",
  "(": "11111011",
  ")": "11110111",
  "*": "101101111",
  "+": "111011111",
  ",": "1110101",
  "-": "110101",
  ".": "1010111",
  "/": "110101111",
  "0": "10110111",
  "1": "10111101",
  "2": "11101101",
  "3": "11111111",
  "4": "101110111",
  "5": "101011011",
  "6": "101101011",
  "7": "110101101",
  "8": "110101011",
  "9": "110110111",
  ":": "11110101",
  ";": "110111101",
  "<": "111101101",
  "=": "1010101",
  ">": "111010111",
  "?": "1010101111",
  "@": "1010111101",
  A: "1111101",
  B: "11101011",
  C: "10101101",
  D: "10110101",
  E: "1110111",
  F: "11011011",
  G: "11111101",
  H: "101010101",
  I: "1111111",
  J: "111111101",
  K: "101111101",
  L: "11010111",
  M: "10111011",
  N: "11011101",
  O: "10101011",
  P: "11010101",
  Q: "111011101",
  R: "10101111",
  S: "1101111",
  T: "1101101",
  U: "101010111",
  V: "110110101",
  W: "101011101",
  X: "101110101",
  Y: "101111011",
  Z: "1010101101",
  "[": "111110111",
  "\\": "111101111",
  "]": "111111011",
  "^": "1010111111",
  _: "101101101",
  "`": "1011011111",
  a: "1011",
  b: "1011111",
  c: "101111",
  d: "101101",
  e: "11",
  f: "111101",
  g: "1011011",
  h: "101011",
  i: "1101",
  j: "111101011",
  k: "10111111",
  l: "11011",
  m: "111011",
  n: "1111",
  o: "111",
  p: "111111",
  q: "110111111",
  r: "10101",
  s: "10111",
  t: "101",
  u: "110111",
  v: "1111011",
  w: "1101011",
  x: "11011111",
  y: "1011101",
  z: "111010101",
  "{": "1010110111",
  "|": "110111011",
  "}": "1010110101",
  "~": "1011010111",
};

function encode(text: string): boolean[] {
  const bits: boolean[] = [];
  for (const ch of text) {
    const code = VARICODE[ch] ?? "";
    for (const c of code) bits.push(c === "1");
    bits.push(false, false);
  }
  return bits;
}

/** 差分 BPSK 调制：前导一个参考符号，位 0 相位反转、位 1 相位保持。 */
function modulate(bits: boolean[], rate: number, center: number, noise: number): number[] {
  const samplesPerSymbol = rate / BAUD;
  const total = Math.round((bits.length + 1) * samplesPerSymbol);
  const step = (2 * Math.PI * center) / rate;
  const phases: number[] = [0];
  let p = 0;
  for (const bit of bits) {
    if (!bit) p = (p + Math.PI) % (2 * Math.PI);
    phases.push(p);
  }
  const samples: number[] = [];
  let t = 0;
  for (let n = 0; n < total; n++) {
    const sym = Math.min(Math.floor(n / samplesPerSymbol), phases.length - 1);
    samples.push(Math.cos(t + phases[sym]));
    t += step;
    if (t >= 2 * Math.PI) t -= 2 * Math.PI;
  }
  if (noise > 0) {
    let seed = 0x1234_5678;
    for (let i = 0; i < samples.length; i++) {
      seed = (seed * 1_664_525 + 1_013_904_223) >>> 0;
      const u = (seed >>> 8) / 16_777_216;
      samples[i] += noise * (u * 2 - 1);
    }
  }
  return samples;
}

/** 写为 16 位 PCM 单声道 WAV 字节。 */
function toWav(samples: number[], rate: number): Buffer {
  const dataLen = samples.length * 2;
  const out = Buffer.alloc(44 + dataLen);
  out.write("RIFF", 0);
  out.writeUInt32LE(36 + dataLen, 4);
  out.write("WAVE", 8);
  out.write("fmt ", 12);
  out.writeUInt32LE(16, 16);
  out.writeUInt16LE(1, 20); // PCM
  out.writeUInt16LE(1, 22); // 单声道
  out.writeUInt32LE(rate, 24);
  out.writeUInt32LE(rate * 2, 28); // 字节率
  out.writeUInt16LE(2, 32); // 块对齐
  out.writeUInt16LE(16, 34); // 位深
  out.write("data", 36);
  out.writeUInt32LE(dataLen, 40);
  for (let i = 0; i < samples.length; i++) {
    const v = Math.max(-1, Math.min(1, samples[i]));
    out.writeInt16LE(Math.round(v * 32767), 44 + i * 2);
  }
  return out;
}

/** 生成合成 PSK31 WAV（默认 8000 Hz / 1000 Hz 载波 / 无噪声）。 */
export function synthesizeWav(
  text: string,
  rate = 8000,
  center = 1000,
  noise = 0,
): Buffer {
  return toWav(modulate(encode(text), rate, center, noise), rate);
}
