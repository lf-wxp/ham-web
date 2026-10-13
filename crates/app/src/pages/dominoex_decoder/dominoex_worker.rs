//! DominoEX 解码 Web Worker 桥接：结果类型与消息解析。

use wasm_bindgen::JsValue;

use crate::decode_worker::get_str;

/// Worker 返回的解码结果。
#[derive(Clone)]
pub struct WorkerPayload {
  /// 主信道文本。
  pub text: String,
  /// 二次信道（ID 信标）文本，可能为空。
  pub id: String,
}

/// 解析统一信封里的内层 `result`（信封 `{ ok, result?, error? }` 已由
/// `crate::decode_worker::parse_outcome` 拆掉）。
pub fn parse_message(result: &JsValue) -> Result<WorkerPayload, String> {
  // `id` 是可选字段（无 ID 信标时不发），`get_str` 是必填语义，这里保留手写可选读取。
  let id = js_sys::Reflect::get(result, &"id".into())
    .ok()
    .and_then(|v| v.as_string())
    .unwrap_or_default();
  Ok(WorkerPayload {
    text: get_str(result, "text")?,
    id,
  })
}
