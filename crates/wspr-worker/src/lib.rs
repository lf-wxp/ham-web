//! WSPR 解码器的 Web Worker 绑定。
//!
//! 把纯 Rust 的 [`ham_web_wspr`] 解码器通过 wasm-bindgen 暴露给 JS，配合
//! `public/decode-worker.js`（统一解码 Worker 协议）在专用线程运行。
//!
//! 构建方式：`wasm-bindgen --target no-modules --out-name wspr_worker`。

use wasm_bindgen::prelude::*;

/// 在 Worker 线程解码 WSPR 音频字节，返回 `{ callsign, locator, power, text }`。
///
/// 统一解码 Worker 协议：导出名固定为 `decode(bytes, params)`，`params.baseHz` 为基带频率。
#[wasm_bindgen]
pub fn decode(bytes: &[u8], params: &JsValue) -> Result<JsValue, JsValue> {
  let base_hz = js_sys::Reflect::get(params, &"baseHz".into())
    .ok()
    .and_then(|v| v.as_f64())
    .unwrap_or(1500.0);
  let (samples, rate) = ham_web_apt::parse_wav(bytes).map_err(|e| e.code())?;
  // 非法基带频率（0 / 负 / 超 Nyquist / NaN）会让解调静默产出空结果，显式拒绝，
  // 别让用户对着「没识别出消息」猜是信号问题还是参数问题（RTTY 同款校验）。
  if !base_hz.is_finite() || base_hz <= 0.0 || base_hz >= f64::from(rate) / 2.0 {
    return Err(JsValue::from_str(&format!(
      "base-hz-out-of-range:{base_hz}"
    )));
  }
  let msg = ham_web_wspr::decode_audio(&samples, base_hz, rate)
    .ok_or_else(|| JsValue::from_str("no-signal:WSPR"))?;
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
