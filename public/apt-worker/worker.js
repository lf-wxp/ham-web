// NOAA APT 解码 Web Worker：在专用线程运行 Rust WASM DSP。
// 由 wasm-bindgen --target no-modules 生成的 apt_worker.js（glue）配合使用。
importScripts('./apt_worker.js');

const ready = wasm_bindgen('./apt_worker_bg.wasm');

self.onmessage = async (event) => {
  try {
    await ready;
    const bytes = new Uint8Array(event.data);
    const result = wasm_bindgen.apt_decode(bytes);
    self.postMessage({ ok: true, result });
  } catch (err) {
    self.postMessage({ ok: false, error: String((err && err.message) || err) });
  }
};
