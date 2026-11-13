//! THOR 解码 Web Worker 桥接：结果类型与消息解析。

use wasm_bindgen::JsValue;

use crate::decode_worker::{
  PicturePayload, get_bool_or, get_f64, get_f64_or, get_pictures, get_str,
};

/// Worker 返回的解码结果。
#[derive(Clone)]
pub struct WorkerPayload {
  /// 主信道文本。
  pub text: String,
  /// 二次信道（ID 信标）文本，可能为空。
  pub id: String,
  /// 实际使用的信号中心频率（Hz）：留空时是自动检测的结果。
  pub center_hz: f64,
  /// 跟踪并修正掉的时钟误差（ppm），正 = 录音偏慢；没有跟踪到是 0。
  pub clock_ppm: f64,
  /// 跟踪并修正掉的频漂（Hz / 秒）；没有跟踪到是 0。
  pub drift_hz_per_s: f64,
  /// 解出的图片（THOR 的图片模式）。
  pub pictures: Vec<PicturePayload>,
  /// 正文里有图片报头但没有解出图片。
  pub picture_skipped: bool,
}

/// 解析统一信封里的内层 `result`（信封 `{ ok, result?, error? }` 已由
/// `crate::decode_worker::parse_outcome` 拆掉）。
pub fn parse_message(result: &JsValue) -> Result<WorkerPayload, String> {
  Ok(WorkerPayload {
    text: get_str(result, "text")?,
    id: get_str(result, "id")?,
    center_hz: get_f64(result, "centerHz")?,
    // 这三项是后加的字段：缓存里的旧 Worker 没有它们，缺失就按「没有跟踪 / 没有图片」处理。
    clock_ppm: get_f64_or(result, "clockPpm", 0.0)?,
    drift_hz_per_s: get_f64_or(result, "driftHzPerS", 0.0)?,
    pictures: get_pictures(result)?,
    picture_skipped: get_bool_or(result, "pictureSkipped", false)?,
  })
}
