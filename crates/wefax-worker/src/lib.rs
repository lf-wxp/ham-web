//! WEFAX 解码器的 Web Worker 绑定。
//!
//! 把纯 Rust 的 [`ham_web_wefax`] 解码器通过 wasm-bindgen 暴露给 JS，配合
//! `public/decode-worker.js`（统一解码 Worker 协议）在专用线程运行。
//!
//! 构建方式：`wasm-bindgen --target no-modules --out-name wefax_worker`。

use wasm_bindgen::prelude::*;

/// 在 Worker 线程解码 WEFAX 音频字节，返回 `{ width, height, gray: Uint8Array }`。
///
/// 统一解码 Worker 协议：导出名固定为 `decode(bytes, params)`；WEFAX 无参数，`params` 忽略。
#[wasm_bindgen]
pub fn decode(bytes: &[u8], _params: &JsValue) -> Result<JsValue, JsValue> {
  // 错误码由 `ham_web_wefax::Error::code()` 统一给出（`NoSync` 已是 `no-signal:WEFAX`），
  // 这里不再二次映射 —— 同一件事两处判断迟早会漂移。
  let img = ham_web_wefax::decode(bytes).map_err(|e| JsValue::from_str(e.code()))?;
  let obj = js_sys::Object::new();
  set(&obj, "width", JsValue::from(img.width as u32))?;
  set(&obj, "height", JsValue::from(img.height as u32))?;
  set(&obj, "gray", to_uint8(&img.gray))?;
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
