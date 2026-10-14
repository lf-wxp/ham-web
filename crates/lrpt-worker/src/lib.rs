//! LRPT 解码器的 Web Worker 绑定。
//!
//! 把纯 Rust 的 [`ham_web_lrpt`] 完整接收链（I/Q → 解调 → Viterbi → ASM → RS →
//! VCDU/MPDU → JPEG → 图像）通过 wasm-bindgen 暴露给 JS。两个解调前端：
//! - 288 kHz 整数采样的 OQPSK（lrpt-encoder 合成信号，[`ham_web_lrpt::decode_stream288`]）；
//! - 任意采样率的 QPSK + 定时 / 载波环路（Meteor-M2 真实过境录音，[`ham_web_lrpt::qpsk`]）。
//!
//! 先跑与采样率匹配的那个，解不出图再试另一个（288 kHz 的真实 QPSK 录音也存在）。
//! 配合 `public/decode-worker.js`（统一解码 Worker 协议）在专用线程运行。
//!
//! 构建方式：`wasm-bindgen --target no-modules --out-name lrpt_worker`。

use wasm_bindgen::prelude::*;

/// 输入字节上限（192 MiB）：页面侧已限 128 MiB，这里是兜底。288 kHz 立体声 f32 展开后约
/// 2 倍体积，再加滤波 / 位流副本，峰值内存可达数倍；超限直接给明确错误，而不是等 wasm OOM trap。
const MAX_INPUT_BYTES: usize = 192 * 1024 * 1024;

/// 最低采样率：72k 符号 × 每符号 2 点（定时环的 Gardner 检测器需要半符号中点）。
const MIN_RATE: u32 = 144_000;

fn set(obj: &js_sys::Object, key: &str, value: JsValue) -> Result<(), JsValue> {
  js_sys::Reflect::set(obj.as_ref(), &JsValue::from_str(key), &value).map(|_| ())
}

/// 在 Worker 线程解码 LRPT 录音（288 kHz 立体声 WAV：左 I 右 Q），返回
/// `{ width, height, rgb }`（RGB 三平面交错的 `Uint8Array`）。
///
/// 统一解码 Worker 协议：导出名固定为 `decode(bytes, params)`，LRPT 没有参数。
/// 采样率至少 [`MIN_RATE`]（每符号 2 个采样点）。
#[wasm_bindgen]
pub fn decode(bytes: &[u8], _params: &JsValue) -> Result<JsValue, JsValue> {
  // `panic = "abort"` 下任何漏网 panic 都是无诊断的 wasm trap：装上 hook 至少能在控制台看到原因。
  console_error_panic_hook::set_once();
  if bytes.len() > MAX_INPUT_BYTES {
    return Err(JsValue::from_str(&format!(
      "input-too-large:{}",
      bytes.len()
    )));
  }
  // 必须用**保留交错**的解析：parse_wav 会把多声道平均成单声道（(I+Q)/2），
  // 对 OQPSK 来说是无意义的混音（wasm 现场抓到：样本数恰好减半）。
  let (samples, rate, channels) =
    ham_web_apt::parse_wav_interleaved(bytes).map_err(|e| e.code())?;
  if rate < MIN_RATE {
    return Err(JsValue::from_str(&format!("need-sample-rate:{rate}")));
  }
  if channels != 2 {
    return Err(JsValue::from_str(&format!("need-stereo:{channels}")));
  }
  let oqpsk = || ham_web_lrpt::decode_stream288(&samples);
  let qpsk = || ham_web_lrpt::qpsk::decode_stream_qpsk(&samples, f64::from(rate));
  let attempts: [&dyn Fn() -> Vec<_>; 2] = if rate == 288_000 {
    [&oqpsk, &qpsk]
  } else {
    // OQPSK 前端只认 288 kHz（整数 4 倍采样）：其他采样率只有 QPSK 一条路。
    [&qpsk, &|| Vec::new()]
  };
  let mut best_frames = 0usize;
  let mut image = None;
  for attempt in attempts {
    let frames = attempt();
    best_frames = best_frames.max(frames.len());
    if let Some(img) = ham_web_lrpt::decode_image(&frames) {
      image = Some(img);
      break;
    }
  }
  let image = image.ok_or_else(|| JsValue::from_str(&format!("image-incomplete:{best_frames}")))?;

  // RGB 三平面交错输出。
  let mut rgb = Vec::with_capacity(image.width * image.height * 3);
  for y in 0..image.height {
    for x in 0..image.width {
      for ch in 0..3 {
        rgb.push(image.channels[ch][y][x]);
      }
    }
  }
  let arr = js_sys::Uint8Array::new_with_length(rgb.len() as u32);
  arr.copy_from(&rgb);

  let out = js_sys::Object::new();
  set(&out, "width", JsValue::from(image.width as u32))?;
  set(&out, "height", JsValue::from(image.height as u32))?;
  set(&out, "rgb", arr.into())?;
  Ok(out.into())
}
