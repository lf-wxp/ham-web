//! NOAA APT 解码器的 Web Worker 绑定。
//!
//! 把纯 Rust 的 [`ham_web_apt`] 解码器通过 wasm-bindgen 暴露给 JS，配合
//! `public/apt-worker/worker.js` 在专用线程运行，避免阻塞主线程 UI。
//!
//! 构建方式：`wasm-bindgen --target no-modules --out-name apt_worker`。

use wasm_bindgen::prelude::*;

/// 在 Worker 线程解码 APT 音频字节，返回可被 `postMessage` 结构化克隆的普通对象。
#[wasm_bindgen]
pub fn apt_decode(bytes: &[u8]) -> Result<JsValue, JsValue> {
  let img = ham_web_apt::decode(bytes).map_err(|e| e.to_string())?;
  let obj = js_sys::Object::new();
  set(&obj, "width", JsValue::from(img.width))?;
  set(&obj, "height", JsValue::from(img.height))?;
  set(&obj, "lines", JsValue::from(img.lines as u32))?;
  set(
    &obj,
    "source_sample_rate",
    JsValue::from(img.source_sample_rate),
  )?;
  set(
    &obj,
    "duration_seconds",
    JsValue::from(img.duration_seconds),
  )?;
  set(&obj, "channel_a", to_uint8(&img.channel_a))?;
  set(&obj, "channel_b", to_uint8(&img.channel_b))?;
  set(&obj, "false_color", to_uint8(&img.false_color()))?;
  Ok(obj.into())
}

fn set(obj: &js_sys::Object, key: &str, value: JsValue) -> Result<(), JsValue> {
  js_sys::Reflect::set(obj.as_ref(), &JsValue::from_str(key), &value).map(|_| ())
}

/// 把字节拷贝为一个独立的 JS `Uint8Array`（可被安全克隆）。
fn to_uint8(data: &[u8]) -> JsValue {
  let arr = js_sys::Uint8Array::new_with_length(data.len() as u32);
  arr.copy_from(data);
  arr.into()
}
