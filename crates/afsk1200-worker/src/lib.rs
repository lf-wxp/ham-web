//! AX.25 AFSK1200（软 TNC）解调的 Web Worker 绑定。
//!
//! 把 [`ham_web_core::afsk1200`] 的解调搬到专用线程：符号定时恢复要穷举「位相位 ×
//! NRZI 初始电平」，一段噪声录音就要跑满几十个相位，放在主线程会把页面冻住几十秒。
//!
//! 构建方式：`wasm-bindgen --target no-modules --out-name afsk1200_worker`。

use wasm_bindgen::prelude::*;

use ham_web_core::afsk1200::{MAX_DECODE_SECONDS, SAMPLE_RATE, demodulate_frames, resample};
use ham_web_core::kiss::parse_ui_frame;

/// 在 Worker 线程解码 AX.25 录音，返回 `{ frames, truncated }`。
///
/// 统一解码 Worker 协议：导出名固定为 `decode(bytes, params)`；AFSK1200 无参数。
///
/// `frames` 每项两种形态：
/// - `{ kind: "ui", source, destination, info }`：帧层解析出呼号与信息域；
/// - `{ kind: "raw", hex }`：FCS 通过但不是 UI 帧，交给页面原样展示。
///
/// `truncated` 为 `true` 表示录音超过 [`MAX_DECODE_SECONDS`]，只解了前面一段 ——
/// 页面要把这件事说出来，否则用户会以为后面的帧「没收到」。
#[wasm_bindgen]
pub fn decode(bytes: &[u8], _params: &JsValue) -> Result<JsValue, JsValue> {
  let (samples, rate) = ham_web_wav::parse(bytes).map_err(|e| e.code())?;
  // 先按原始采样率截断再重采样：8 MiB 的 8 kHz 录音是 524 秒，直接升到 48 kHz 会先分配
  // 2500 万样本（100 MB），截断放在重采样之后等于白花这笔内存。
  let max_input = rate as usize * MAX_DECODE_SECONDS as usize;
  let truncated = samples.len() > max_input;
  let samples = &samples[..samples.len().min(max_input)];
  let samples = resample(samples, rate, SAMPLE_RATE);

  let frames = demodulate_frames(&samples, SAMPLE_RATE);
  let arr = js_sys::Array::new();
  for frame in &frames {
    let obj = js_sys::Object::new();
    match parse_ui_frame(frame) {
      Some(ui) => {
        set(&obj, "kind", JsValue::from_str("ui"))?;
        set(&obj, "source", JsValue::from_str(&ui.source))?;
        set(&obj, "destination", JsValue::from_str(&ui.destination))?;
        set(
          &obj,
          "info",
          JsValue::from_str(&String::from_utf8_lossy(&ui.info)),
        )?;
      }
      None => {
        set(&obj, "kind", JsValue::from_str("raw"))?;
        let hex = frame.iter().map(|b| format!("{b:02X}")).collect::<String>();
        set(&obj, "hex", JsValue::from_str(&hex))?;
      }
    }
    arr.push(&obj);
  }

  let out = js_sys::Object::new();
  set(&out, "frames", arr.into())?;
  set(&out, "truncated", JsValue::from_bool(truncated))?;
  Ok(out.into())
}

fn set(obj: &js_sys::Object, key: &str, value: JsValue) -> Result<(), JsValue> {
  js_sys::Reflect::set(obj.as_ref(), &JsValue::from_str(key), &value).map(|_| ())
}
