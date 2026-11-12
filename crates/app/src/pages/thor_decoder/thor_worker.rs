//! THOR 解码 Web Worker 桥接：结果类型与消息解析。

use wasm_bindgen::JsValue;

use crate::decode_worker::{get_f64, get_str};

/// Worker 返回的解码结果。
#[derive(Clone)]
pub struct WorkerPayload {
  /// 主信道文本。
  pub text: String,
  /// 二次信道（ID 信标）文本，可能为空。
  pub id: String,
  /// 实际使用的信号中心频率（Hz）：留空时是自动检测的结果。
  pub center_hz: f64,
}

/// 解析统一信封里的内层 `result`（信封 `{ ok, result?, error? }` 已由
/// `crate::decode_worker::parse_outcome` 拆掉）。
pub fn parse_message(result: &JsValue) -> Result<WorkerPayload, String> {
  Ok(WorkerPayload {
    text: get_str(result, "text")?,
    id: get_str(result, "id")?,
    center_hz: get_f64(result, "centerHz")?,
  })
}
