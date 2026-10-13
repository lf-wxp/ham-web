//! LRPT 解码器的 Web Worker 绑定。
//!
//! 把纯 Rust 的 [`ham_web_lrpt`] 完整接收链（288 kHz I/Q → RRC → Viterbi →
//! NRZ-M → ASM → RS → VCDU/MPDU → JPEG → 图像）通过 wasm-bindgen 暴露给 JS，
//! 配合 `public/decode-worker.js`（统一解码 Worker 协议）在专用线程运行。
//!
//! 构建方式：`wasm-bindgen --target no-modules --out-name lrpt_worker`。

use wasm_bindgen::prelude::*;

/// 输入字节上限（192 MiB）：页面侧已限 128 MiB，这里是兜底。288 kHz 立体声 f32 展开后约
/// 2 倍体积，再加滤波 / 位流副本，峰值内存可达数倍；超限直接给明确错误，而不是等 wasm OOM trap。
const MAX_INPUT_BYTES: usize = 192 * 1024 * 1024;

fn set(obj: &js_sys::Object, key: &str, value: JsValue) -> Result<(), JsValue> {
  js_sys::Reflect::set(obj.as_ref(), &JsValue::from_str(key), &value).map(|_| ())
}

/// 在 Worker 线程解码 LRPT 录音（288 kHz 立体声 WAV：左 I 右 Q），返回
/// `{ width, height, rgb }`（RGB 三平面交错的 `Uint8Array`）。
///
/// 统一解码 Worker 协议：导出名固定为 `decode(bytes, params)`，LRPT 没有参数。
/// 如实说明：当前链路按 288 kHz 立体声（72k 符号 × 4 采样）设计 —— 其他采样率
/// 的录音先重采样再上传。
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
  if rate != 288_000 {
    return Err(JsValue::from_str(&format!("need-sample-rate:{rate}")));
  }
  if channels != 2 {
    return Err(JsValue::from_str(&format!("need-stereo:{channels}")));
  }
  let frames = ham_web_lrpt::decode_stream288(&samples);
  let image = ham_web_lrpt::decode_image(&frames)
    .ok_or_else(|| JsValue::from_str(&format!("image-incomplete:{}", frames.len())))?;

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
