//! APT 解码 Web Worker 桥接：结果类型与消息解析。

use wasm_bindgen::JsValue;

use crate::decode_worker::{get_f64, get_u8s, get_u32};
use crate::i18n::tf;

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

/// 解析统一信封里的内层 `result`（信封 `{ ok, result?, error? }` 已由
/// `crate::decode_worker::parse_outcome` 拆掉）。
pub fn parse_message(result: &JsValue) -> Result<WorkerPayload, String> {
  let width = get_u32(result, "width")?;
  let height = get_u32(result, "height")?;
  let channel_a = get_u8s(result, "channel_a")?;
  let channel_b = get_u8s(result, "channel_b")?;
  let false_color = get_u8s(result, "false_color")?;
  // 长度与尺寸不符会在 `ImageData` 构造处报英文 DOMException：在这里按协议错误
  // 就地拒绝（与 lrpt_worker 同一口径）。
  let expected_gray = (width as usize).checked_mul(height as usize);
  let expected_rgba = expected_gray.and_then(|n| n.checked_mul(4));
  if width == 0
    || height == 0
    || expected_gray != Some(channel_a.len())
    || expected_gray != Some(channel_b.len())
    || expected_rgba != Some(false_color.len())
  {
    return Err(tf("common.field-is-missing", &["channel_a"]));
  }
  Ok(WorkerPayload {
    width,
    height,
    lines: get_u32(result, "lines")?,
    source_sample_rate: get_u32(result, "source_sample_rate")?,
    duration_seconds: get_f64(result, "duration_seconds")?,
    channel_a,
    channel_b,
    false_color,
  })
}
