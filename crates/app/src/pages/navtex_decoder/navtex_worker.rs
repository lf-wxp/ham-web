//! NAVTEX 解码 Web Worker 桥接：结果类型与消息解析。

use wasm_bindgen::{JsCast, JsValue};

use crate::decode_worker::{get_bool, get_str, get_u32};
use crate::i18n::tf;

/// 一条 NAVTEX 报文。
#[derive(Clone)]
pub struct WorkerMessage {
  /// B1：发射台覆盖区字母。
  pub transmitter: String,
  /// B2：报文类别字母。
  pub category: String,
  /// B3B4：序号。
  pub serial: u32,
  /// 报文正文（错误字符以 ■ 占位）。
  pub text: String,
  /// 由另一副本修正的字符数。
  pub corrected: usize,
  /// 无法修正的错误字符数。
  pub errors: usize,
  /// 报文总字符数。
  pub chars: usize,
  /// 是否通过 4% 错误率验收门限。
  pub accepted: bool,
}

fn field(obj: &JsValue, key: &str) -> Result<JsValue, String> {
  js_sys::Reflect::get(obj, &key.into()).map_err(|_| tf("common.field-is-missing", &[key]))
}

/// 解析统一信封里的内层 `result`（信封 `{ ok, result?, error? }` 已由
/// `crate::decode_worker::parse_outcome` 拆掉）。
pub fn parse_message(result: &JsValue) -> Result<Vec<WorkerMessage>, String> {
  let messages = field(result, "messages")?;
  let arr = messages
    .dyn_ref::<js_sys::Array>()
    .ok_or_else(|| tf("common.field-is-missing", &["messages"]))?;
  let mut out = Vec::with_capacity(arr.length() as usize);
  for i in 0..arr.length() {
    let obj = arr.get(i);
    out.push(WorkerMessage {
      transmitter: get_str(&obj, "transmitter")?,
      category: get_str(&obj, "category")?,
      serial: get_u32(&obj, "serial")?,
      text: get_str(&obj, "text")?,
      corrected: get_u32(&obj, "corrected")? as usize,
      errors: get_u32(&obj, "errors")? as usize,
      chars: get_u32(&obj, "chars")? as usize,
      accepted: get_bool(&obj, "accepted")?,
    });
  }
  Ok(out)
}
