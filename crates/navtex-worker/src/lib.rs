//! NAVTEX 解码器的 Web Worker 绑定。
//!
//! 把纯 Rust 的 [`ham_web_navtex`] 解码器通过 wasm-bindgen 暴露给 JS，配合
//! `public/decode-worker.js`（统一解码 Worker 协议）在专用线程运行。
//!
//! 构建方式：`wasm-bindgen --target no-modules --out-name navtex_worker`。

use wasm_bindgen::prelude::*;

fn set(obj: &js_sys::Object, key: &str, value: JsValue) -> Result<(), JsValue> {
  js_sys::Reflect::set(obj.as_ref(), &JsValue::from_str(key), &value).map(|_| ())
}

/// 在 Worker 线程解码 NAVTEX 音频字节，返回 `{ messages }`。
///
/// 统一解码 Worker 协议：导出名固定为 `decode(bytes, params)`，NAVTEX 没有
/// 参数。每个报文：`{ transmitter, category, serial, text, corrected, errors,
/// chars, accepted }`。
#[wasm_bindgen]
pub fn decode(bytes: &[u8], _params: &JsValue) -> Result<JsValue, JsValue> {
  let (samples, rate) = ham_web_apt::parse_wav(bytes).map_err(|e| e.code())?;
  let messages = ham_web_navtex::decode_audio(&samples, rate)
    .filter(|m| !m.is_empty())
    .ok_or_else(|| JsValue::from_str("no-signal:NAVTEX"))?;
  let arr = js_sys::Array::new();
  for m in messages {
    let obj = js_sys::Object::new();
    set(
      &obj,
      "transmitter",
      JsValue::from_str(&m.transmitter.to_string()),
    )?;
    set(&obj, "category", JsValue::from_str(&m.category.to_string()))?;
    set(&obj, "serial", JsValue::from(m.serial))?;
    set(&obj, "text", JsValue::from_str(&m.text))?;
    set(&obj, "corrected", JsValue::from_f64(m.corrected as f64))?;
    set(&obj, "errors", JsValue::from_f64(m.errors as f64))?;
    set(&obj, "chars", JsValue::from_f64(m.chars as f64))?;
    set(&obj, "accepted", JsValue::from_bool(m.accepted))?;
    arr.push(&obj);
  }
  let out = js_sys::Object::new();
  set(&out, "messages", arr.into())?;
  Ok(out.into())
}
