//! WEFAX 解码 Web Worker 桥接：结果类型与消息解析。

use wasm_bindgen::JsValue;

use crate::decode_worker::{get_u8s, get_u32};
use crate::i18n::tf;

/// Worker 返回的解码结果（单幅灰度图）。
#[derive(Clone)]
pub struct WorkerPayload {
  pub width: u32,
  pub height: u32,
  pub gray: Vec<u8>,
}

/// 解析统一信封里的内层 `result`（信封 `{ ok, result?, error? }` 已由
/// `crate::decode_worker::parse_outcome` 拆掉）。
pub fn parse_message(result: &JsValue) -> Result<WorkerPayload, String> {
  let width = get_u32(result, "width")?;
  let height = get_u32(result, "height")?;
  let gray = get_u8s(result, "gray")?;
  // 长度与尺寸不符会在 `ImageData` 构造处报 DOMException：在这里按协议错误就地拒绝。
  // 文案要指「尺寸对不上」，不能说成「字段 gray 缺失」——字段明明都在，会把排查带偏。
  let expected = (width as usize).checked_mul(height as usize);
  if width == 0 || height == 0 || expected != Some(gray.len()) {
    return Err(tf(
      "tools.decode-err-image-size-mismatch",
      &[&format!("{width}×{height}"), &gray.len().to_string()],
    ));
  }
  Ok(WorkerPayload {
    width,
    height,
    gray,
  })
}
