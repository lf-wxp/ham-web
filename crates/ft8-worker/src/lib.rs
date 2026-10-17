//! FT8 / FT4 解码器的 Web Worker 绑定。
//!
//! 把纯 Rust 的 [`ham_web_ft8`] 解码器通过 wasm-bindgen 暴露给 JS，配合
//! `public/decode-worker.js`（统一解码 Worker 协议）在专用线程运行。解码流程本身
//! （按时隙切片、哈希表学习、去重、不支持类型计数）都在 `ham_web_ft8::decode_recording`
//! 里并有原生单测；这里只做 WAV 解析与 JS 对象的搭建。
//!
//! 构建方式：`wasm-bindgen --target no-modules --out-name ft8_worker`。

use ham_web_ft8::{Protocol, RecordingError, decode_recording};
use wasm_bindgen::prelude::*;

/// 在 Worker 线程解码一段 FT8 / FT4 音频（WAV，12000 Hz），返回
/// `{ messages: [{ text, freq_hz, time_sec, sync_db }], unsupported, slots }`。
///
/// - `time_sec`：距录音开头的秒数；
/// - `sync_db`：同步强度（dB），**不是** WSJT-X 口径的 SNR，界面按「同步强度」展示；
/// - `unsupported`：CRC 通过、但消息类型还没有字段解析的条数（DXpedition / EU VHF /
///   Field Day 等），它们不进 `messages`（否则界面上是只有频率的空行），由调用方决定怎么提示；
/// - `slots`：录音被切成多少个时隙逐个解码。
///
/// 统一解码 Worker 协议：导出名固定为 `decode(bytes, params)`，`params.protocol` 为
/// `"ft8"`（默认）或 `"ft4"`。
#[wasm_bindgen]
pub fn decode(bytes: &[u8], params: &JsValue) -> Result<JsValue, JsValue> {
  let protocol = js_sys::Reflect::get(params, &"protocol".into())
    .ok()
    .and_then(|v| v.as_string())
    .map_or(Protocol::Ft8, |s| {
      if s == "ft4" {
        Protocol::Ft4
      } else {
        Protocol::Ft8
      }
    });

  let (samples, rate) = ham_web_apt::parse_wav(bytes).map_err(|e| e.code())?;
  // FT8 / FT4 的时频分析按 12000 Hz 采样率设计，别的采样率直接拒绝，避免静默解错。
  let recording = decode_recording(&samples, rate, protocol).map_err(|e| match e {
    RecordingError::SampleRate(rate) => JsValue::from_str(&format!("ft8-sample-rate:{rate}")),
  })?;

  let messages = js_sys::Array::new();
  for m in &recording.messages {
    let item = js_sys::Object::new();
    set(&item, "text", &JsValue::from_str(&m.text));
    set(&item, "freq_hz", &JsValue::from_f64(f64::from(m.freq_hz)));
    set(&item, "time_sec", &JsValue::from_f64(f64::from(m.time_sec)));
    set(&item, "sync_db", &JsValue::from_f64(f64::from(m.sync_db)));
    messages.push(&item);
  }

  let result = js_sys::Object::new();
  set(&result, "messages", &messages);
  set(
    &result,
    "unsupported",
    &JsValue::from_f64(f64::from(recording.unsupported)),
  );
  set(&result, "slots", &JsValue::from_f64(recording.slots as f64));
  Ok(result.into())
}

/// 给 JS 对象写一个字段。`Reflect::set` 只在目标不是对象时失败，这里的目标都是刚建的普通对象。
fn set(target: &js_sys::Object, key: &str, value: &JsValue) {
  let _ = js_sys::Reflect::set(target, &JsValue::from_str(key), value);
}
