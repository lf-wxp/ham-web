//! MFSK 解码 Web Worker 桥接：结果类型与消息解析。

use wasm_bindgen::JsValue;

use crate::decode_worker::{get_f64, get_str, get_u32};

/// Worker 返回的解码结果。
#[derive(Clone)]
pub struct WorkerPayload {
  /// 解码出的文本。
  pub text: String,
  /// 实际使用的信号中心频率（Hz）：留空时是自动检测的结果。
  pub center_hz: f64,
  /// 译码用的交织深度（MFSK4 / 8 会在两种深度间择优）。
  pub depth: u32,
}

/// 解析统一信封里的内层 `result`（信封 `{ ok, result?, error? }` 已由
/// `crate::decode_worker::parse_outcome` 拆掉）。
pub fn parse_message(result: &JsValue) -> Result<WorkerPayload, String> {
  Ok(WorkerPayload {
    text: get_str(result, "text")?,
    center_hz: get_f64(result, "centerHz")?,
    depth: get_u32(result, "depth")?,
  })
}
