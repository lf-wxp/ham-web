//! APT 解码 Web Worker 桥接：结果类型与消息解析。

use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

use crate::i18n::{t, tf};
use crate::util::js_error_message;

/// Worker 返回的原始解码结果（像素 + 元信息）。
pub struct WorkerPayload {
  pub width: u32,
  pub height: u32,
  pub lines: u32,
  pub source_sample_rate: u32,
  pub duration_seconds: f64,
  pub channel_a: Vec<u8>,
  pub channel_b: Vec<u8>,
  pub false_color: Vec<u8>,
}

/// 解析 Worker `postMessage` 的消息体：`{ ok: bool, result?, error? }`。
pub fn parse_message(data: &JsValue) -> Result<WorkerPayload, String> {
  let ok = js_sys::Reflect::get(data, &"ok".into())
    .ok()
    .and_then(|v| v.as_bool())
    .unwrap_or(false);
  if !ok {
    let err = js_sys::Reflect::get(data, &"error".into())
      .ok()
      .and_then(|v| v.as_string())
      .unwrap_or_else(|| t("radio.decoding-failed"));
    return Err(err);
  }

  let result = js_sys::Reflect::get(data, &"result".into()).map_err(|e| js_error_message(&e))?;
  Ok(WorkerPayload {
    width: get_u32(&result, "width")?,
    height: get_u32(&result, "height")?,
    lines: get_u32(&result, "lines")?,
    source_sample_rate: get_u32(&result, "source_sample_rate")?,
    duration_seconds: get_f64(&result, "duration_seconds")?,
    channel_a: get_u8s(&result, "channel_a")?,
    channel_b: get_u8s(&result, "channel_b")?,
    false_color: get_u8s(&result, "false_color")?,
  })
}

fn get_u32(obj: &JsValue, key: &str) -> Result<u32, String> {
  js_sys::Reflect::get(obj, &key.into())
    .ok()
    .and_then(|v| v.as_f64())
    .map(|v| v as u32)
    .ok_or_else(|| tf("common.field-is-missing", &[(key)]))
}

fn get_f64(obj: &JsValue, key: &str) -> Result<f64, String> {
  js_sys::Reflect::get(obj, &key.into())
    .ok()
    .and_then(|v| v.as_f64())
    .ok_or_else(|| tf("common.field-is-missing", &[(key)]))
}

fn get_u8s(obj: &JsValue, key: &str) -> Result<Vec<u8>, String> {
  let v = js_sys::Reflect::get(obj, &key.into()).map_err(|e| js_error_message(&e))?;
  let arr: js_sys::Uint8Array = v
    .dyn_into()
    .map_err(|_| tf("common.field-is-not-a", &[(key)]))?;
  Ok(arr.to_vec())
}
