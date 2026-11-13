//! THOR 解码器的 Web Worker 绑定。
//!
//! 把纯 Rust 的 [`ham_web_thor`] 解码器通过 wasm-bindgen 暴露给 JS，配合
//! `public/decode-worker.js`（统一解码 Worker 协议）在专用线程运行。
//!
//! 构建方式：`wasm-bindgen --target no-modules --out-name thor_worker`。

use wasm_bindgen::prelude::*;

/// 协议里不放非有限数：跟踪拟合异常时可能是 `inf` / `NaN`，页面拿去做格式化会写出 `NaN ppm`。
fn finite_or_zero(value: f64) -> f64 {
  if value.is_finite() { value } else { 0.0 }
}

/// 在 Worker 线程解码 THOR 音频字节，返回 `{ text, id, centerHz, clockPpm, driftHzPerS, pictures }`。
///
/// 统一解码 Worker 协议：导出名固定为 `decode(bytes, params)`。`params`：
/// - `mode`：子模式（`"16"`、`"50x2"`、`"micro"` 等，缺省 `"16"`）；
/// - `centerHz`：信号中心频率；缺省 / 非数字 = 自动检测；
/// - `reverse`：边带反转（LSB 录音），缺省 `false`。
///
/// `id` 是二次信道（ID 信标）文本，可能为空；`centerHz` 回报实际使用的中心频率（自动检测时就是检测结果）；
/// `clockPpm` / `driftHzPerS` 是跟踪并修正掉的时钟误差与频漂，没有跟踪到时精确为 0；
/// `pictures` 是录音里解出的图片数组（可为空），每项 `{ width, height, color, rgba }`，`rgba` 是 `Uint8Array`（RGBA，宽 × 高 × 4）；
/// `pictureSkipped` 为真表示正文里有图片报头但没有解出图片（采样率不符等）。
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
  let mode = ham_web_thor::parse_mode(&mode_name)
    .ok_or_else(|| JsValue::from_str(&format!("unsupported-submode:{mode_name}")))?;
  let (samples, rate) = ham_web_wav::parse(bytes).map_err(|e| e.code())?;
  // 音调带（含两侧各一个邻位）必须放得进 Nyquist，否则全部混叠，之后只会报误导性的 no-signal。
  let min_rate = ham_web_thor::min_sample_rate(mode);
  if rate < min_rate {
    return Err(JsValue::from_str(&format!("rate-too-low:{min_rate}")));
  }
  if let Some(c) = center
    && !ham_web_thor::center_in_range(mode, rate, c)
  {
    return Err(JsValue::from_str(&format!("center-hz-out-of-range:{c}")));
  }
  // 只发 ID 信标、主信道没有流量的台站也是有效解码：主信道与二次信道都空才算无信号。
  let d = ham_web_thor::decode_audio(&samples, rate, mode, center, reverse)
    .filter(|d| !d.text.trim().is_empty() || !d.id.trim().is_empty() || !d.pictures.is_empty())
    .ok_or_else(|| JsValue::from_str("no-signal:THOR"))?;
  let obj = js_sys::Object::new();
  let set = |k: &str, v: JsValue| {
    js_sys::Reflect::set(&obj, &JsValue::from_str(k), &v).map_err(|e| format!("{e:?}"))
  };
  set("text", JsValue::from_str(&d.text))?;
  set("id", JsValue::from_str(&d.id))?;
  set("centerHz", JsValue::from_f64(d.center_hz))?;
  set("clockPpm", JsValue::from_f64(finite_or_zero(d.clock_ppm)))?;
  set(
    "driftHzPerS",
    JsValue::from_f64(finite_or_zero(d.drift_hz_per_s)),
  )?;
  let pictures = js_sys::Array::new();
  for p in &d.pictures {
    let pic = js_sys::Object::new();
    let put = |k: &str, v: JsValue| {
      js_sys::Reflect::set(&pic, &JsValue::from_str(k), &v).map_err(|e| format!("{e:?}"))
    };
    put("width", JsValue::from_f64(p.width as f64))?;
    put("height", JsValue::from_f64(p.height as f64))?;
    put("color", JsValue::from_bool(p.color))?;
    // 页面直接喂给 `ImageData`，要 RGBA；`to_rgba` 给出的是一整块连续字节，`Uint8Array::from` 一次拷进 JS 堆。
    put(
      "rgba",
      js_sys::Uint8Array::from(p.to_rgba().as_slice()).into(),
    )?;
    pictures.push(&pic);
  }
  set("pictures", pictures.into())?;
  set("pictureSkipped", JsValue::from_bool(d.picture_skipped))?;
  Ok(obj.into())
}
