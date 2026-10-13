//! DominoEX 解码器的 Web Worker 绑定。
//!
//! 把纯 Rust 的 [`ham_web_dominoex`] 解码器通过 wasm-bindgen 暴露给 JS，配合
//! `public/decode-worker.js`（统一解码 Worker 协议）在专用线程运行。
//!
//! 构建方式：`wasm-bindgen --target no-modules --out-name dominoex_worker`。

use wasm_bindgen::prelude::*;

/// 在 Worker 线程解码 DominoEX 音频字节，返回 `{ text, id }`。
///
/// 统一解码 Worker 协议：导出名固定为 `decode(bytes, params)`，
/// `params.mode` 为子模式（`"8"` 等，缺省即 `"8"`）；`id` 为二次信道文本
/// （ID 信标，可能为空）。
#[wasm_bindgen]
pub fn decode(bytes: &[u8], params: &JsValue) -> Result<JsValue, JsValue> {
  let mode = js_sys::Reflect::get(params, &"mode".into())
    .ok()
    .and_then(|v| v.as_string())
    .unwrap_or_else(|| "8".to_owned());
  let (baud, spacing) = ham_web_dominoex::parse_mode(&mode)
    .ok_or_else(|| JsValue::from_str(&format!("unsupported-submode:{mode}")))?;
  let (samples, rate) = ham_web_apt::parse_wav(bytes).map_err(|e| e.code())?;
  // 音调带上沿（约 1000 + 9×间隔 Hz）必须低于 Nyquist，否则全部混叠，之后只会报误导性的 no-signal。
  let top = ham_web_dominoex::BASEFREQ + ham_web_dominoex::NUMTONES as f64 * spacing / 2.0;
  if top >= f64::from(rate) / 2.0 {
    return Err(JsValue::from_str(&format!(
      "rate-too-low:{}",
      (top * 2.0).ceil() as u32 + 1
    )));
  }
  let (text, id) = ham_web_dominoex::decode_audio(&samples, rate, baud, spacing)
    .filter(|(t, _)| !t.is_empty())
    .ok_or_else(|| JsValue::from_str("no-signal:DominoEX"))?;
  let obj = js_sys::Object::new();
  js_sys::Reflect::set(&obj, &JsValue::from_str("text"), &JsValue::from_str(&text))
    .map_err(|e| format!("{e:?}"))?;
  js_sys::Reflect::set(&obj, &JsValue::from_str("id"), &JsValue::from_str(&id))
    .map_err(|e| format!("{e:?}"))?;
  Ok(obj.into())
}
