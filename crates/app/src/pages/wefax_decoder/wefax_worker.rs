//! WEFAX 解码 Web Worker 桥接：结果类型与消息解析。

use wasm_bindgen::JsValue;

use crate::decode_worker::{get_f64, get_str, get_u8s, get_u32};
use crate::i18n::tf;

/// Worker 返回的解码结果（单幅灰度图）。
#[derive(Clone)]
pub struct WorkerPayload {
  pub width: u32,
  pub height: u32,
  pub gray: Vec<u8>,
  /// 实际采用的格式：`"fax"`（真实传真）或 `"simplified"`（简化模型）。
  pub format: String,
  /// 简化模型实际采用的解调方式：`"fm"` / `"am"`；真实传真为空。
  pub modulation: String,
  /// 真实传真的诊断信息；简化模型下全是 0 / 空。
  pub fax: FaxInfo,
}

/// 真实传真的诊断信息。
#[derive(Clone, Default)]
pub struct FaxInfo {
  /// 实测行速率（LPM）。
  pub lpm: f64,
  /// 检测到的相位线行数；0 = 没找到（行首位置没有依据）。
  pub phasing_lines: u32,
  /// 实际采用的黑 / 白电平（Hz）。
  pub black_hz: f64,
  pub white_hz: f64,
  /// 电平来源：`"phasing"` / `"nominal"` / `"stretched"`。
  pub levels: String,
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
  let format = get_str(result, "format")?;
  let (modulation, fax) = if format == "fax" {
    (
      String::new(),
      FaxInfo {
        lpm: get_f64(result, "lpm")?,
        phasing_lines: get_u32(result, "phasingLines")?,
        black_hz: get_f64(result, "blackHz")?,
        white_hz: get_f64(result, "whiteHz")?,
        levels: get_str(result, "levels")?,
      },
    )
  } else {
    (get_str(result, "modulation")?, FaxInfo::default())
  };
  Ok(WorkerPayload {
    width,
    height,
    gray,
    format,
    modulation,
    fax,
  })
}
