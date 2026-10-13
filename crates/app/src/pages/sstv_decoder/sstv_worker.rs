//! SSTV 解码 Web Worker 桥接：结果类型与消息解析。

use wasm_bindgen::JsValue;

use crate::decode_worker::{get_f64, get_str, get_u8s, get_u32};
use crate::i18n::tf;

/// Worker 返回的原始解码结果（RGB 像素 + 元信息）。
pub struct WorkerPayload {
  pub width: u32,
  pub height: u32,
  pub mode: String,
  pub source_sample_rate: u32,
  pub duration_seconds: f64,
  pub rgba: Vec<u8>,
}

/// 解析统一信封里的内层 `result`（信封 `{ ok, result?, error? }` 已由
/// `crate::decode_worker::parse_outcome` 拆掉）。
pub fn parse_message(result: &JsValue) -> Result<WorkerPayload, String> {
  let width = get_u32(result, "width")?;
  let height = get_u32(result, "height")?;
  let rgba = get_u8s(result, "rgba")?;
  // 长度与尺寸不符会在 `ImageData` 构造处报英文 DOMException：在这里按协议错误
  // 就地拒绝（与 lrpt_worker 同一口径）。
  let expected = (width as usize)
    .checked_mul(height as usize)
    .and_then(|n| n.checked_mul(4));
  if width == 0 || height == 0 || expected != Some(rgba.len()) {
    return Err(tf("common.field-is-missing", &["rgba"]));
  }
  Ok(WorkerPayload {
    width,
    height,
    mode: get_str(result, "mode")?,
    source_sample_rate: get_u32(result, "source_sample_rate")?,
    duration_seconds: get_f64(result, "duration_seconds")?,
    rgba,
  })
}
