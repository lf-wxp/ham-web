//! WSPR 解码 Web Worker 桥接：结果类型与消息解析。

use wasm_bindgen::JsValue;

use crate::i18n::{t, tf};
use crate::util::js_error_message;

/// Worker 返回的解码结果。
#[derive(Clone)]
pub struct WorkerPayload {
  pub callsign: String,
  pub locator: String,
  pub power: u32,
  pub text: String,
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
    callsign: get_str(&result, "callsign")?,
    locator: get_str(&result, "locator")?,
    power: get_u32(&result, "power")?,
    text: get_str(&result, "text")?,
  })
}

fn get_u32(obj: &JsValue, key: &str) -> Result<u32, String> {
  js_sys::Reflect::get(obj, &key.into())
    .ok()
    .and_then(|v| v.as_f64())
    .map(|v| v as u32)
    .ok_or_else(|| tf("common.field-is-missing", &[(key)]))
}

fn get_str(obj: &JsValue, key: &str) -> Result<String, String> {
  js_sys::Reflect::get(obj, &key.into())
    .ok()
    .and_then(|v| v.as_string())
    .ok_or_else(|| tf("common.field-is-missing", &[(key)]))
}
