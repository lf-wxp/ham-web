//! Feld Hell 解码 Web Worker 桥接：结果类型与消息解析。

use wasm_bindgen::JsValue;

use crate::decode_worker::get_str;

/// Worker 返回的解码结果。
#[derive(Clone)]
pub struct WorkerPayload {
  pub text: String,
}

/// 解析统一信封里的内层 `result`（信封 `{ ok, result?, error? }` 已由
/// `crate::decode_worker::parse_outcome` 拆掉）。
pub fn parse_message(result: &JsValue) -> Result<WorkerPayload, String> {
  Ok(WorkerPayload {
    text: get_str(result, "text")?,
  })
}
