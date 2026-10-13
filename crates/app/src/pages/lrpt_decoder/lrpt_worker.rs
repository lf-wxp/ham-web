//! LRPT 解码 Web Worker 桥接：结果类型与消息解析。

use wasm_bindgen::JsValue;

use crate::decode_worker::{get_u8s, get_u32};
use crate::i18n::tf;

/// Worker 返回的解码结果。
#[derive(Clone)]
pub struct WorkerPayload {
  /// 图像宽度（像素）。
  pub width: u32,
  /// 图像高度（像素）。
  pub height: u32,
  /// RGB 三平面交错的像素字节。
  pub rgb: Vec<u8>,
  /// 渲染后的 PNG data URL（进入页面后由渲染函数生成）。
  pub image_url: String,
}

/// 解析统一信封里的内层 `result`（信封 `{ ok, result?, error? }` 已由
/// `crate::decode_worker::parse_outcome` 拆掉）。
pub fn parse_message(result: &JsValue) -> Result<WorkerPayload, String> {
  let width = get_u32(result, "width")?;
  let height = get_u32(result, "height")?;
  let rgb = get_u8s(result, "rgb")?;
  // 长度对不上时 `ImageData::new` 会抛 DOM 异常，用户看到的是英文原文：在这里就地拒绝。
  let expected = (width as usize)
    .checked_mul(height as usize)
    .and_then(|n| n.checked_mul(3));
  if width == 0 || height == 0 || expected != Some(rgb.len()) {
    return Err(tf("common.field-is-missing", &["rgb"]));
  }
  Ok(WorkerPayload {
    width,
    height,
    rgb,
    image_url: String::new(),
  })
}
