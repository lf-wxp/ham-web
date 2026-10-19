//! Feld Hell 解码器的 Web Worker 绑定。
//!
//! 把纯 Rust 的 [`ham_web_hell`] 解码器通过 wasm-bindgen 暴露给 JS，配合
//! `public/decode-worker.js`（统一解码 Worker 协议）在专用线程运行。
//!
//! 构建方式：`wasm-bindgen --target no-modules --out-name hell_worker`。

use wasm_bindgen::prelude::*;

/// 在 Worker 线程解码 Feld Hell 音频字节，返回 `{ text }`。
///
/// 统一解码 Worker 协议：导出名固定为 `decode(bytes, params)`；Feld Hell 无参数，
/// `params` 忽略（页面侧发 `null`）。
#[wasm_bindgen]
pub fn decode(bytes: &[u8], _params: &JsValue) -> Result<JsValue, JsValue> {
  let (samples, rate) = ham_web_wav::parse(bytes).map_err(|e| e.code())?;
  let text = ham_web_hell::decode_audio(&samples, rate)
    .ok_or_else(|| JsValue::from_str("no-signal:HELL"))?;
  let obj = js_sys::Object::new();
  js_sys::Reflect::set(
    obj.as_ref(),
    &JsValue::from_str("text"),
    &JsValue::from_str(&text),
  )
  .map_err(|e| format!("{e:?}"))?;
  Ok(obj.into())
}
