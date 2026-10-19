//! 软 TNC 的 Worker 桥接：结果类型与消息解析。

use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

use crate::decode_worker::{get_bool, get_str};
use crate::i18n::{t, tf};

/// Worker 返回的单个 AX.25 帧。
#[derive(Clone, PartialEq, Eq)]
pub(super) enum TncFrame {
  /// 帧层解析出了呼号与信息域。
  Ui {
    source: String,
    destination: String,
    info: String,
  },
  /// FCS 通过但不是 UI 帧：原样给十六进制串。
  Raw(String),
}

/// Worker 返回的解码结果。
#[derive(Clone, PartialEq, Eq)]
pub(super) struct WorkerPayload {
  pub(super) frames: Vec<TncFrame>,
  /// 录音超过 Worker 的时长上限，只解了前面一段。
  pub(super) truncated: bool,
}

/// 解析统一信封里的内层 `result`（信封 `{ ok, result?, error? }` 已由
/// `crate::decode_worker::parse_outcome` 拆掉）。
///
/// 一帧都没解出来时返回错误文案而不是空结果：页面要明确告诉用户「没收到帧」，
/// 而不是显示一个空列表；录音被截断时文案要补上「只处理了前一段」，否则用户会以为整段
/// 录音都没有帧。
pub(super) fn parse_message(result: &JsValue) -> Result<WorkerPayload, String> {
  let frames = js_sys::Reflect::get(result, &JsValue::from_str("frames"))
    .ok()
    .and_then(|v| v.dyn_into::<js_sys::Array>().ok())
    .ok_or_else(|| tf("common.field-is-missing", &["frames"]))?;
  let mut out = Vec::new();
  for value in frames.iter() {
    let kind = get_str(&value, "kind")?;
    match kind.as_str() {
      "ui" => out.push(TncFrame::Ui {
        source: get_str(&value, "source")?,
        destination: get_str(&value, "destination")?,
        info: get_str(&value, "info")?,
      }),
      "raw" => out.push(TncFrame::Raw(get_str(&value, "hex")?)),
      // Worker 与页面版本不一致才会到这里：协议细节按惯例保留英文码，便于对照。
      other => return Err(format!("bad-frame-kind:{other}")),
    }
  }
  // 先读 `truncated`：没解出帧时它决定该说「没有帧」还是「只处理了前 60 秒」。
  let truncated = get_bool(result, "truncated")?;
  if out.is_empty() {
    return Err(if truncated {
      t("tools.soft-tnc-no-frame-truncated")
    } else {
      t("tools.soft-tnc-no-frame")
    });
  }
  Ok(WorkerPayload {
    frames: out,
    truncated,
  })
}
