import fs from 'node:fs';
const { default: createCore } = await import(`${process.env.FLDIGI_WEB ?? '/tmp/fldigi-web'}/web/fldigi-core.js`);
const { createDecoderSession } = await import(`${process.env.FLDIGI_WEB ?? '/tmp/fldigi-web'}/web/decoder-session.js`);

const [,, modeName, wavPath, freq] = process.argv;
const core = await createCore();
const modes = JSON.parse(core.UTF8ToString(core._web_modes()));
const mode = modes.find(m => m.name === modeName);
if (!mode) { console.error('no such mode', modeName); process.exit(1); }
let text = '', secondary = '';
const session = createDecoderSession(core, (m) => { if (m.type === 'decoded') { text += m.text; secondary += m.secondary; } });
const buf = fs.readFileSync(wavPath);
const rate = buf.readUInt32LE(24);
const n = (buf.length - 44) / 2;
const x = new Float32Array(n);
for (let i = 0; i < n; i++) x[i] = buf.readInt16LE(44 + i * 2) / 32768;
// 末尾补 6 秒静音，让解码器管线里压着的字符流出来（与「录音停在电台关发射之后」一致）。
const tail = new Float32Array(rate * 6);
session.handle({ type: 'reset', generation: 1 });
session.handle({ type: 'configure', generation: 1, sequence: 1, settings: { mode: mode.id, frequency: Number(freq), afc: 0, sql: 0 }, retune: true });
const feed = (s) => { for (let o = 0; o < s.length; o += 4096) session.handle({ type: 'audio', generation: 1, rate, samples: s.slice(o, o + 4096) }); };
feed(x); feed(tail);
session.handle({ type: 'flush', generation: 1 });
console.log(JSON.stringify({ mode: modeName, text, secondary }));
