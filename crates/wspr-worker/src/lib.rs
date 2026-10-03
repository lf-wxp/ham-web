//! WSPR 解码器的 Web Worker 绑定。
//!
//! 把纯 Rust 的 [`ham_web_wspr`] 解码器通过 wasm-bindgen 暴露给 JS，配合
//! `public/wspr-worker/worker.js` 在专用线程运行。
//!
//! 构建方式：`wasm-bindgen --target no-modules --out-name wspr_worker`。

use wasm_bindgen::prelude::*;

/// 在 Worker 线程解码 WSPR 音频字节，返回 `{ callsign, locator, power, text }`。
#[wasm_bindgen]
pub fn wspr_decode(bytes: &[u8], base_hz: f64) -> Result<JsValue, JsValue> {
  let (samples, rate) = ham_web_apt::parse_wav(bytes).map_err(|e| e.to_string())?;
  let msg = ham_web_wspr::decode_audio(&samples, base_hz, rate)
    .ok_or_else(|| JsValue::from_str("解码失败：未识别出有效 WSPR 消息"))?;
  let obj = js_sys::Object::new();
  set(&obj, "callsign", JsValue::from_str(&msg.callsign))?;
  set(&obj, "locator", JsValue::from_str(&msg.locator))?;
  set(&obj, "power", JsValue::from(msg.power as u32))?;
  set(&obj, "text", JsValue::from_str(&msg.to_string()))?;
  Ok(obj.into())
}

fn set(obj: &js_sys::Object, key: &str, value: JsValue) -> Result<(), JsValue> {
  js_sys::Reflect::set(obj.as_ref(), &JsValue::from_str(key), &value).map(|_| ())
}
