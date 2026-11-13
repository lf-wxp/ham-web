// 用 fldigi 4.2.13 的原版发端发一张测试图（MFSK 图片模式），写成 8 kHz 单声道 WAV，并把「想发的像素」另存成 .rgb。
//
//   FLDIGI_WEB=/tmp/fldigi-web node gen_pic.mjs <模式> <宽> <高> <color|gray> <spp> <中心 Hz> <输出.wav> [bands|ramp]
//
// 模式取 MFSK16 / MFSK32 / MFSK64 / MFSK128（图片尺寸随意，spp = 每像素样本数 8 / 4 / 2），
// 或 THOR11 / THOR16 / THOR22 / THOR25 / THOR32 / THOR44 / THOR56（尺寸只能取 fldigi 的六档：59x74 / 120x150 / 240x300 / 160x120 / 320x240 / 640x480，spp 固定 10）。`<输出.wav>.rgb` 是宽 × 高 × 3 字节的
// 原始 RGB（灰度图三个通道相同，已按 fldigi 的灰度公式 31/61/8 算过）。图案：
//   bands —— 四块纯色（与 fldigi-web 自己的测试同款：左右 R 220/40、上下 G 180/70、左右 B 30/210）；
//   ramp  —— R 随 x、G 随 y 线性变化，B 是 8 像素的棋盘格：能看出像素对齐、行 / 通道顺序与高频响应。
import fs from 'node:fs';

const root = process.env.FLDIGI_WEB;
if (!root) throw new Error('请先设置 FLDIGI_WEB（fldigi-web 克隆目录，见 README.md）');
const { default: createCore } = await import(`${root}/web/fldigi-core.js`);
const { createEncoderSession } = await import(`${root}/web/encoder-session.js`);
const { defaults } = await import(`${root}/web/configuration.js`);
const { imagePlan } = await import(`${root}/web/transmit-image.js`);

const [, , modeName, w, h, kind, spp, center, outPath, pattern = 'bands'] = process.argv;
if (!outPath) {
  console.error('用法: gen_pic.mjs <模式> <宽> <高> <color|gray> <spp> <中心Hz> <输出.wav> [bands|ramp]');
  process.exit(1);
}
const core = await createCore();
const modes = JSON.parse(core.UTF8ToString(core._web_modes()));
const mode = modes.find((m) => m.name === modeName);
if (!mode) throw new Error(`没有这个模式 ${modeName}`);
const gray = kind === 'gray';
// THOR 的格式值是「尺寸序号-gray|color」，MFSK 是 gray / color。
const thorSizes = [[59, 74], [120, 150], [240, 300], [160, 120], [320, 240], [640, 480]];
let format = gray ? 'gray' : 'color';
if (mode.family === 'THOR') {
  const idx = thorSizes.findIndex(([tw, th]) => tw === Number(w) && th === Number(h));
  if (idx < 0) throw new Error(`THOR 只支持 ${thorSizes.map(([tw, th]) => `${tw}x${th}`).join(' / ')}`);
  format = `${idx}-${format}`;
}
const plan = imagePlan(mode, Number(w), Number(h), {
  format,
  width: Number(w),
  spp: Number(spp),
});
const pixels = new Uint8Array(plan.width * plan.height * 3);
for (let y = 0; y < plan.height; y++) {
  for (let x = 0; x < plan.width; x++) {
    const i = (y * plan.width + x) * 3;
    let rgb;
    if (pattern === 'ramp') {
      rgb = [
        Math.round((x * 255) / Math.max(1, plan.width - 1)),
        Math.round((y * 255) / Math.max(1, plan.height - 1)),
        (Math.floor(x / 4) + Math.floor(y / 4)) % 2 ? 230 : 25,
      ];
    } else {
      rgb = [x < plan.width / 2 ? 220 : 40, y < plan.height / 2 ? 180 : 70, x < plan.width / 2 ? 30 : 210];
    }
    if (gray) {
      const g = Math.floor((31 * rgb[0] + 61 * rgb[1] + 8 * rgb[2]) / 100);
      rgb = [g, g, g];
    }
    pixels.set(rgb, i);
  }
}

const messages = [];
const encoder = createEncoderSession(core, (m) => messages.push(m));
encoder.handle({
  type: 'start',
  job: 1,
  text: '',
  image: { ...plan, pixels },
  live: false,
  settings: { ...defaults, mode: mode.id, frequency: Number(center), txFrequency: Number(center) },
});
const started = messages.at(-1);
if (started.type !== 'started') throw new Error(JSON.stringify(started));
const rate = started.rate;
const chunks = [];
let count = 0;
for (let step = 0; step < 100000; step++) {
  encoder.handle({ type: 'pull', job: 1, maximum: 8192 });
  const data = messages.at(-1);
  if (data.samples.length) {
    chunks.push(data.samples);
    count += data.samples.length;
  }
  if (data.done) break;
}
const pcm = Buffer.alloc(44 + count * 2);
pcm.write('RIFF', 0);
pcm.writeUInt32LE(36 + count * 2, 4);
pcm.write('WAVEfmt ', 8);
pcm.writeUInt32LE(16, 16);
pcm.writeUInt16LE(1, 20);
pcm.writeUInt16LE(1, 22);
pcm.writeUInt32LE(rate, 24);
pcm.writeUInt32LE(rate * 2, 28);
pcm.writeUInt16LE(2, 32);
pcm.writeUInt16LE(16, 34);
pcm.write('data', 36);
pcm.writeUInt32LE(count * 2, 40);
let o = 44;
for (const part of chunks) {
  for (const v of part) {
    pcm.writeInt16LE(Math.max(-32768, Math.min(32767, Math.round(v * 32767))), o);
    o += 2;
  }
}
fs.writeFileSync(outPath, pcm);
fs.writeFileSync(`${outPath}.rgb`, pixels);
console.log(`${modeName} ${plan.width}x${plan.height} ${gray ? 'gray' : 'color'} spp ${plan.spp} @${rate} Hz: ${count} samples (${(count / rate).toFixed(1)} s) -> ${outPath}`);
