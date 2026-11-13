import fs from 'node:fs';
const root = process.env.FLDIGI_WEB;
if (!root) throw new Error('请先设置 FLDIGI_WEB（fldigi-web 克隆目录，见 README.md）');
const { default: createCore } = await import(`${root}/web/fldigi-core.js`);
const { createEncoderSession } = await import(`${root}/web/encoder-session.js`);

const core = await createCore();
const modes = JSON.parse(core.UTF8ToString(core._web_modes()));
if (process.argv[2] === 'list') {
  console.log(modes.filter(m => /THOR|MFSK/i.test(m.name)).map(m => `${m.id}\t${m.name}\t${m.family}\tenabled=${m.enabled}`).join('\n'));
  process.exit(0);
}
const [,, modeName, text, freq, outPath] = process.argv;
const mode = modes.find(m => m.name === modeName);
if (!mode) { console.error('no such mode', modeName); process.exit(1); }
const msgs = [];
const session = createEncoderSession(core, (m) => msgs.push(m));
session.handle({ type: 'start', job: 1, text, live: false, settings: { mode: mode.id, frequency: Number(freq), callsign: '' } });
const started = msgs.find(m => m.type === 'started');
const chunks = [];
let done = false;
for (let i = 0; i < 100000 && !done; i++) {
  msgs.length = 0;
  session.handle({ type: 'pull', job: 1, maximum: 8192 });
  for (const m of msgs) if (m.type === 'samples') { chunks.push(m.samples); if (m.done) done = true; }
}
const total = chunks.reduce((a, c) => a + c.length, 0);
const rate = started.rate;
const pcm = Buffer.alloc(total * 2);
let o = 0;
for (const c of chunks) for (const v of c) { pcm.writeInt16LE(Math.round(Math.max(-1, Math.min(1, v)) * (v < 0 ? 32768 : 32767)), o); o += 2; }
const head = Buffer.alloc(44);
head.write('RIFF', 0); head.writeUInt32LE(36 + pcm.length, 4); head.write('WAVEfmt ', 8); head.writeUInt32LE(16, 16);
head.writeUInt16LE(1, 20); head.writeUInt16LE(1, 22); head.writeUInt32LE(rate, 24); head.writeUInt32LE(rate * 2, 28);
head.writeUInt16LE(2, 32); head.writeUInt16LE(16, 34); head.write('data', 36); head.writeUInt32LE(pcm.length, 40);
fs.writeFileSync(outPath, Buffer.concat([head, pcm]));
console.log(`${modeName} @${rate} Hz: ${total} samples (${(total / rate).toFixed(1)} s) -> ${outPath}, tx freq ${started.frequency}`);
