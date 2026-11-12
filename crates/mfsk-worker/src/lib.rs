//! MFSK 解码器的 Web Worker 绑定。
//!
//! 把纯 Rust 的 [`ham_web_mfsk`] 解码器通过 wasm-bindgen 暴露给 JS，配合
//! `public/decode-worker.js`（统一解码 Worker 协议）在专用线程运行。
//!
//! 构建方式：`wasm-bindgen --target no-modules --out-name mfsk_worker`。

use wasm_bindgen::prelude::*;

/// 在 Worker 线程解码 MFSK 音频字节，返回 `{ text, centerHz, depth }`。
///
/// 统一解码 Worker 协议：导出名固定为 `decode(bytes, params)`。`params`：
/// - `mode`：子模式（`"16"` 等，缺省 `"16"`）；
/// - `centerHz`：信号中心频率；缺省 / 非数字 = 自动检测；
/// - `reverse`：边带反转（LSB 录音），缺省 `false`。
///
/// `centerHz` 在结果里回报实际使用的中心频率（自动检测时就是检测结果），`depth` 是译码用的交织深度。
#[wasm_bindgen]
pub fn decode(bytes: &[u8], params: &JsValue) -> Result<JsValue, JsValue> {
  let mode_name = js_sys::Reflect::get(params, &"mode".into())
    .ok()
    .and_then(|v| v.as_string())
    .unwrap_or_else(|| "16".to_owned());
  let center = js_sys::Reflect::get(params, &"centerHz".into())
    .ok()
    .and_then(|v| v.as_f64());
  let reverse = js_sys::Reflect::get(params, &"reverse".into())
    .ok()
    .and_then(|v| v.as_bool())
    .unwrap_or(false);
  let mode = ham_web_mfsk::parse_mode(&mode_name)
    .ok_or_else(|| JsValue::from_str(&format!("unsupported-submode:{mode_name}")))?;
  let (samples, rate) = ham_web_wav::parse(bytes).map_err(|e| e.code())?;
  // 音调带（含两侧各一个邻位）必须放得进 Nyquist，否则全部混叠，之后只会报误导性的 no-signal。
  let min_rate = ham_web_mfsk::min_sample_rate(mode);
  if rate < min_rate {
    return Err(JsValue::from_str(&format!("rate-too-low:{min_rate}")));
  }
  if let Some(c) = center
    && !ham_web_mfsk::center_in_range(mode, rate, c)
  {
    return Err(JsValue::from_str(&format!("center-hz-out-of-range:{c}")));
  }
  let d = ham_web_mfsk::decode_audio(&samples, rate, mode, center, reverse)
    .filter(|d| !d.text.trim().is_empty())
    .ok_or_else(|| JsValue::from_str("no-signal:MFSK"))?;
  let obj = js_sys::Object::new();
  let set = |k: &str, v: JsValue| {
    js_sys::Reflect::set(&obj, &JsValue::from_str(k), &v).map_err(|e| format!("{e:?}"))
  };
  set("text", JsValue::from_str(&d.text))?;
  set("centerHz", JsValue::from_f64(d.center_hz))?;
  set("depth", JsValue::from_f64(d.depth as f64))?;
  Ok(obj.into())
}
