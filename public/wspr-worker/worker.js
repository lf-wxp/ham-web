// WSPR 弱信号传播报告解码 Web Worker：在专用线程运行 Rust WASM DSP。
// 由 wasm-bindgen --target no-modules 生成的 wspr_worker.js（glue）配合使用。
importScripts('./wspr_worker.js');

const ready = wasm_bindgen('./wspr_worker_bg.wasm');

self.onmessage = async (event) => {
  try {
    await ready;
    const { bytes, baseHz } = event.data;
    const bytesArr = new Uint8Array(bytes);
    const result = wasm_bindgen.wspr_decode(bytesArr, Number(baseHz) || 1500);
    self.postMessage({ ok: true, result });
  } catch (err) {
    self.postMessage({ ok: false, error: String((err && err.message) || err) });
  }
};
