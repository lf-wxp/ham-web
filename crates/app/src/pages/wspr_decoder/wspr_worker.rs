//! WSPR 解码 Web Worker 桥接：结果类型与消息解析。

use wasm_bindgen::JsValue;

use crate::decode_worker::{get_str, get_u32};

/// Worker 返回的解码结果。
#[derive(Clone)]
pub struct WorkerPayload {
  pub callsign: String,
  pub locator: String,
  pub power: u32,
  pub text: String,
}

/// 解析统一信封里的内层 `result`（信封 `{ ok, result?, error? }` 已由
/// `crate::decode_worker::parse_outcome` 拆掉）。
pub fn parse_message(result: &JsValue) -> Result<WorkerPayload, String> {
  Ok(WorkerPayload {
    callsign: get_str(result, "callsign")?,
    locator: get_str(result, "locator")?,
    power: get_u32(result, "power")?,
    text: get_str(result, "text")?,
  })
}
