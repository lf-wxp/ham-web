//! RTTY 解码器的 Web Worker 绑定。
//!
//! 把纯 Rust 的 [`ham_web_rtty`] 解码器通过 wasm-bindgen 暴露给 JS，配合
//! `public/decode-worker.js`（统一解码 Worker 协议）在专用线程运行。
//!
//! 构建方式：`wasm-bindgen --target no-modules --out-name rtty_worker`。

use wasm_bindgen::prelude::*;

/// 在 Worker 线程解码 RTTY 音频字节，返回 `{ text }`。
///
/// 统一解码 Worker 协议：导出名固定为 `decode(bytes, params)`，`params.markHz` 为传号频率
/// （默认 2125 Hz；空号 = 传号 + 170 Hz）。
#[wasm_bindgen]
pub fn decode(bytes: &[u8], params: &JsValue) -> Result<JsValue, JsValue> {
  let mark_hz = js_sys::Reflect::get(params, &"markHz".into())
    .ok()
    .and_then(|v| v.as_f64())
    .unwrap_or(2125.0);
  let (samples, rate) = ham_web_apt::parse_wav(bytes).map_err(|e| e.code())?;
  // mark_hz 非法（0 / 负 / 超 Nyquist / NaN）会让 Goertzel 全 NaN 静默产出空结果；
  // 空号（mark + 170 Hz）越过 Nyquist 会混叠到 mark 附近，判别直接失效。两者都在
  // 这里显式拒绝，而不是让用户对着空文本猜。
  if !mark_hz.is_finite()
    || mark_hz <= 0.0
    || mark_hz + ham_web_rtty::SHIFT_HZ >= f64::from(rate) / 2.0
  {
    return Err(JsValue::from_str(&format!(
      "mark-hz-out-of-range:{mark_hz}"
    )));
  }
  let text = ham_web_rtty::decode_audio(&samples, rate, mark_hz)
    .ok_or_else(|| JsValue::from_str("no-signal:RTTY"))?;
  let obj = js_sys::Object::new();
  js_sys::Reflect::set(
    obj.as_ref(),
    &JsValue::from_str("text"),
    &JsValue::from_str(&text),
  )
  .map_err(|e| format!("{e:?}"))?;
  Ok(obj.into())
}
