//! Olivia 解码器的 Web Worker 绑定。
//!
//! 把纯 Rust 的 [`ham_web_olivia`] 解码器通过 wasm-bindgen 暴露给 JS，配合
//! `public/decode-worker.js`（统一解码 Worker 协议）在专用线程运行。
//!
//! 构建方式：`wasm-bindgen --target no-modules --out-name olivia_worker`。

use wasm_bindgen::prelude::*;

/// 在 Worker 线程解码 Olivia 音频字节，返回 `{ text }`。
///
/// 统一解码 Worker 协议：导出名固定为 `decode(bytes, params)`，
/// `params.mode` 为子模式（如 `"32/1000@1500"`，缺省即该值）。
#[wasm_bindgen]
pub fn decode(bytes: &[u8], params: &JsValue) -> Result<JsValue, JsValue> {
  let mode = js_sys::Reflect::get(params, &"mode".into())
    .ok()
    .and_then(|v| v.as_string())
    .unwrap_or_else(|| "32/1000@1500".to_owned());
  let (symbols, bandwidth, center) = ham_web_olivia::parse_submode(&mode)
    .ok_or_else(|| JsValue::from_str(&format!("unsupported-submode:{mode}")))?;
  // parse_submode 只做语法解析：`"4/500@750"` 这类参数组合语法合法但语义非法
  // （fsep 125 Hz ≠ 标准 31.25 Hz），在这里按标准子模式表拒掉（音调数 + 带宽
  // 一致 = 31.25 波特）。
  if ham_web_olivia::bits_per_symbol(symbols).is_none()
    || !bandwidth.is_finite()
    || (bandwidth - ham_web_olivia::BAUD * f64::from(symbols)).abs() > 1e-6
  {
    return Err(JsValue::from_str(&format!(
      "olivia-params:{symbols},{bandwidth}"
    )));
  }
  let (samples, rate) = ham_web_apt::parse_wav(bytes).map_err(|e| e.code())?;
  // 音调带的上沿必须低于 Nyquist，否则全部混叠，之后只会报误导性的 `no-signal`。
  // `center` 非有限（如 `"32/1000@NaN"`）时比较恒为 false 会被放行，最终由
  // `decode_audio` 兜底误报成无信号 —— 在这里与 Nyquist 一起显式拒绝。
  if !center.is_finite() || center + bandwidth / 2.0 >= f64::from(rate) / 2.0 {
    return Err(JsValue::from_str(&format!(
      "olivia-params:{symbols},{bandwidth}@{center}"
    )));
  }
  let text = ham_web_olivia::decode_audio(&samples, rate, symbols, bandwidth, center)
    .filter(|t| !t.is_empty())
    .ok_or_else(|| JsValue::from_str("no-signal:Olivia"))?;
  let obj = js_sys::Object::new();
  js_sys::Reflect::set(
    obj.as_ref(),
    &JsValue::from_str("text"),
    &JsValue::from_str(&text),
  )
  .map_err(|e| format!("{e:?}"))?;
  Ok(obj.into())
}
