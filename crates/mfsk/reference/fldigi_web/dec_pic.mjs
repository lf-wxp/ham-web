// 用 fldigi 4.2.13 自己的接收机解 MFSK 图片录音，输出图片并（旁边有 `<wav>.rgb` 原图时）报告逐字节误差。
//
//   FLDIGI_WEB=/tmp/fldigi-web node dec_pic.mjs <模式> <输入.wav> <中心 Hz> [输出.rgb]
//
// 误差的口径与 `examples/decode_picture.rs` 一致（平均绝对误差、最大、> 24 的字节数），两边直接对比。
import fs from 'node:fs';

const root = process.env.FLDIGI_WEB;
if (!root) throw new Error('请先设置 FLDIGI_WEB（fldigi-web 克隆目录，见 README.md）');
const { default: createCore } = await import(`${root}/web/fldigi-core.js`);

const [, , modeName, wavPath, center, outPath] = process.argv;
const core = await createCore();
const modes = JSON.parse(core.UTF8ToString(core._web_modes()));
const mode = modes.find((m) => m.name === modeName);
if (!mode) throw new Error(`没有这个模式 ${modeName}`);
const buf = fs.readFileSync(wavPath);
const rate = buf.readUInt32LE(24);
const n = (buf.length - 44) / 2;
const x = new Float32Array(n + rate * 4);
for (let i = 0; i < n; i++) x[i] = buf.readInt16LE(44 + i * 2) / 32768;

core._web_create(mode.id);
core._web_set_frequency(Number(center));
core._web_set_option(0, 0);
core._web_set_option(1, 0);
const block = core._malloc(512 * 4);
let picture;
let updates = 0;
for (let offset = 0; offset < x.length; offset += 512) {
  const part = x.subarray(offset, offset + 512);
  core.HEAPF32.set(part, block / 4);
  core._web_process(block, part.length);
  const pointer = core._web_take_image_updates();
  const length = core._web_image_updates_size();
  if (length) {
    const size = core._web_image_width() * core._web_image_height() * 3;
    if (!picture || picture.length !== size) picture = new Uint8Array(size);
    for (let i = 0; i < length; i += 2) picture[core.HEAPU32[pointer / 4 + i]] = core.HEAPU32[pointer / 4 + i + 1];
    updates += length / 2;
  }
}
console.log(`fldigi：${core._web_image_width()} x ${core._web_image_height()}，${updates} 个字节更新`);
if (picture && outPath) fs.writeFileSync(outPath, picture);
const truthPath = `${wavPath}.rgb`;
if (picture && fs.existsSync(truthPath)) {
  const want = fs.readFileSync(truthPath);
  if (want.length === picture.length) {
    let sum = 0;
    let max = 0;
    let bad = 0;
    for (let i = 0; i < want.length; i++) {
      const e = Math.abs(picture[i] - want[i]);
      sum += e;
      max = Math.max(max, e);
      if (e > 24) bad++;
    }
    console.log(`  与原图比：平均误差 ${(sum / want.length).toFixed(2)}，最大 ${max}，误差 > 24 的字节 ${bad} / ${want.length}`);
  }
}
