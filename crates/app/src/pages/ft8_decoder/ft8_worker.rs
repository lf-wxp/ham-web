//! FT8 / FT4 解码 Web Worker 桥接：结果类型与消息解析。

use wasm_bindgen::JsValue;

use crate::decode_worker::{get_f64, get_str};

/// Worker 返回的单条解码消息。
#[derive(Clone)]
pub struct DecodedMessage {
  pub text: String,
  pub freq_hz: f64,
  pub time_sec: f64,
  /// 同步强度（dB）。不是 WSJT-X 口径的 SNR，界面按「同步强度」展示。
  pub sync_db: f64,
}

/// Worker 返回的解码结果（消息列表 + 未能解析的类型条数 + 时隙数）。
#[derive(Clone)]
pub struct WorkerPayload {
  pub messages: Vec<DecodedMessage>,
  /// CRC 通过、但消息类型还没有字段解析的条数（DXpedition / EU VHF / Field Day 等）。
  pub unsupported: u32,
  /// 录音被切成多少个时隙逐个解码。
  pub slots: u32,
}

/// 解析统一信封里的内层 `result`（信封 `{ ok, result?, error? }` 已由
/// `crate::decode_worker::parse_outcome` 拆掉）。
pub fn parse_message(result: &JsValue) -> Result<WorkerPayload, String> {
  let messages_val =
    js_sys::Reflect::get(result, &"messages".into()).map_err(|_| "messages".to_owned())?;
  let arr = js_sys::Array::from(&messages_val);
  let mut messages = Vec::with_capacity(arr.length() as usize);
  for i in 0..arr.length() {
    let item = arr.get(i);
    messages.push(DecodedMessage {
      text: get_str(&item, "text")?,
      freq_hz: get_f64(&item, "freq_hz")?,
      time_sec: get_f64(&item, "time_sec")?,
      sync_db: get_f64(&item, "sync_db")?,
    });
  }
  // 旧版 worker 没有这两个字段：缺省按 0 / 1 处理，不让整条结果解析失败。
  let count = |key: &str, default: u32| {
    js_sys::Reflect::get(result, &key.into())
      .ok()
      .and_then(|v| v.as_f64())
      .map_or(default, |v| v.max(0.0) as u32)
  };
  Ok(WorkerPayload {
    messages,
    unsupported: count("unsupported", 0),
    slots: count("slots", 1),
  })
}
