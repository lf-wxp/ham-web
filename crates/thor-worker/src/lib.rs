//! THOR 解码器的 Web Worker 绑定。
//!
//! 把纯 Rust 的 [`ham_web_thor`] 解码器通过 wasm-bindgen 暴露给 JS，配合
//! `public/decode-worker.js`（统一解码 Worker 协议）在专用线程运行。
//!
//! 构建方式：`wasm-bindgen --target no-modules --out-name thor_worker`。

use ham_web_thor::EncodeError;
use wasm_bindgen::prelude::*;

/// 协议里不放非有限数：跟踪拟合异常时可能是 `inf` / `NaN`，页面拿去做格式化会写出 `NaN ppm`。
fn finite_or_zero(value: f64) -> f64 {
  if value.is_finite() { value } else { 0.0 }
}

/// 在 Worker 线程解码 THOR 音频字节，返回 `{ text, id, centerHz, clockPpm, driftHzPerS, pictures }`。
///
/// 统一解码 Worker 协议：导出名固定为 `decode(bytes, params)`。`params`：
/// - `mode`：子模式（`"16"`、`"50x2"`、`"micro"` 等，缺省 `"16"`）；
/// - `centerHz`：信号中心频率；缺省 / 非数字 = 自动检测；
/// - `reverse`：边带反转（LSB 录音），缺省 `false`。
///
/// `id` 是二次信道（ID 信标）文本，可能为空；`centerHz` 回报实际使用的中心频率（自动检测时就是检测结果）；
/// `clockPpm` / `driftHzPerS` 是跟踪并修正掉的时钟误差与频漂，没有跟踪到时精确为 0；
/// `pictures` 是录音里解出的图片数组（可为空），每项 `{ width, height, color, rgba }`，`rgba` 是 `Uint8Array`（RGBA，宽 × 高 × 4）；
/// `pictureSkipped` 为真表示正文里有图片报头但没有解出图片（采样率不符等）。
#[wasm_bindgen]
pub fn decode(bytes: &[u8], params: &JsValue) -> Result<JsValue, JsValue> {
  let mode_name = js_sys::Reflect::get(params, &"mode".into())
    .ok()
    .and_then(|v| v.as_string())
    .unwrap_or_else(|| "16".to_owned());
  let center = js_sys::Reflect::get(params, &"centerHz".into())
    .ok()
    .and_then(|v| v.as_f64());
  let reverse = js_sys::Reflect::get(params, &"reverse".into())
    .ok()
    .and_then(|v| v.as_bool())
    .unwrap_or(false);
  let mode = ham_web_thor::parse_mode(&mode_name)
    .ok_or_else(|| JsValue::from_str(&format!("unsupported-submode:{mode_name}")))?;
  let (samples, rate) = ham_web_wav::parse(bytes).map_err(|e| e.code())?;
  // 音调带（含两侧各一个邻位）必须放得进 Nyquist，否则全部混叠，之后只会报误导性的 no-signal。
  let min_rate = ham_web_thor::min_sample_rate(mode);
  if rate < min_rate {
    return Err(JsValue::from_str(&format!("rate-too-low:{min_rate}")));
  }
  if let Some(c) = center
    && !ham_web_thor::center_in_range(mode, rate, c)
  {
    return Err(JsValue::from_str(&format!("center-hz-out-of-range:{c}")));
  }
  // 只发 ID 信标、主信道没有流量的台站也是有效解码：主信道与二次信道都空才算无信号。
  let d = ham_web_thor::decode_audio(&samples, rate, mode, center, reverse)
    .filter(|d| !d.text.trim().is_empty() || !d.id.trim().is_empty() || !d.pictures.is_empty())
    .ok_or_else(|| JsValue::from_str("no-signal:THOR"))?;
  let obj = js_sys::Object::new();
  let set = |k: &str, v: JsValue| {
    js_sys::Reflect::set(&obj, &JsValue::from_str(k), &v).map_err(|e| format!("{e:?}"))
  };
  set("text", JsValue::from_str(&d.text))?;
  set("id", JsValue::from_str(&d.id))?;
  set("centerHz", JsValue::from_f64(d.center_hz))?;
  set("clockPpm", JsValue::from_f64(finite_or_zero(d.clock_ppm)))?;
  set(
    "driftHzPerS",
    JsValue::from_f64(finite_or_zero(d.drift_hz_per_s)),
  )?;
  let pictures = js_sys::Array::new();
  for p in &d.pictures {
    let pic = js_sys::Object::new();
    let put = |k: &str, v: JsValue| {
      js_sys::Reflect::set(&pic, &JsValue::from_str(k), &v).map_err(|e| format!("{e:?}"))
    };
    put("width", JsValue::from_f64(p.width as f64))?;
    put("height", JsValue::from_f64(p.height as f64))?;
    put("color", JsValue::from_bool(p.color))?;
    // 页面直接喂给 `ImageData`，要 RGBA；`to_rgba` 给出的是一整块连续字节，`Uint8Array::from` 一次拷进 JS 堆。
    put(
      "rgba",
      js_sys::Uint8Array::from(p.to_rgba().as_slice()).into(),
    )?;
    pictures.push(&pic);
  }
  set("pictures", pictures.into())?;
  set("pictureSkipped", JsValue::from_bool(d.picture_skipped))?;
  Ok(obj.into())
}

/// 参数字段：缺省 / `null` 是 `None`；给了但类型不对按「非法」处理（字符串给空串、数字给 `NaN`），
/// 这样仍落到对应的错误码，而不是悄悄换成缺省值。
fn param(params: &JsValue, key: &str) -> Option<JsValue> {
  let v = js_sys::Reflect::get(params, &JsValue::from_str(key)).unwrap_or(JsValue::UNDEFINED);
  (!v.is_undefined() && !v.is_null()).then_some(v)
}

/// 编码结果：`{ samples: Float32Array, sampleRate }`。`Float32Array::from` 拷出一块独占的 buffer，
/// `decode-worker.js` 才能把它整块转移给页面。
fn encode_result(samples: &[f32], rate: u32) -> Result<JsValue, JsValue> {
  let out = js_sys::Object::new();
  js_sys::Reflect::set(
    &out,
    &JsValue::from_str("samples"),
    &js_sys::Float32Array::from(samples).into(),
  )?;
  js_sys::Reflect::set(&out, &JsValue::from_str("sampleRate"), &JsValue::from(rate))?;
  Ok(out.into())
}

/// `picture` 参数：`{ width, height, color, spp?, rgb: Uint8Array }`。
///
/// 缺省 / `null` 是「不发图片」；给了但字段缺失或类型不对一律回 `encode-params:picture`
/// （不悄悄改成别的值）。尺寸与像素数据是否对得上、子模式有没有图片模式，由核心 crate 的
/// `encode_picture_message` 校验；这里只做 JS ↔ Rust 的转换。
struct PictureParam {
  width: usize,
  height: usize,
  color: bool,
  rgb: Vec<u8>,
}

fn picture_param(params: &JsValue) -> Result<Option<PictureParam>, JsValue> {
  let Some(pic) = param(params, "picture") else {
    return Ok(None);
  };
  let bad = || JsValue::from_str(&EncodeError::Picture.code());
  let get = |key: &str| js_sys::Reflect::get(&pic, &JsValue::from_str(key)).map_err(|_| bad());
  // 非负整数，且不超过一千万：再大的尺寸本来就会被像素上限拒绝，这里先挡住 `as usize` 的饱和。
  let dim = |key: &str| -> Result<usize, JsValue> {
    get(key)?
      .as_f64()
      .filter(|v| v.is_finite() && *v >= 0.0 && v.fract() == 0.0 && *v <= 10_000_000.0)
      .map(|v| v as usize)
      .ok_or_else(bad)
  };
  let rgb: js_sys::Uint8Array = get("rgb")?.dyn_into().map_err(|_| bad())?;
  Ok(Some(PictureParam {
    width: dim("width")?,
    height: dim("height")?,
    color: get("color")?.as_bool().ok_or_else(bad)?,
    rgb: rgb.to_vec(),
  }))
}

/// 合成一段 THOR 基带，返回 `{ samples: Float32Array, sampleRate }`。
///
/// 统一 Worker 协议的编码入口。`params`：`mode`（缺省 `"16"`）、`text`（有 `picture` 时是报头前面的
/// 引导文本，可以为空）、`centerHz`（缺省 1500 Hz）、`id`（ID 信标，走二次信道，可选，至多 64 字符）、
/// `picture`（`{ width, height, color, rgb }`，给了就发一次图片：THOR 11 / 16 / 22 / 25 / 32 / 44 / 56，
/// 尺寸只能是 59×74、120×150、160×120、240×300、320×240、640×480；每像素样本数固定 10，不读 `spp`）。
/// 主信道报文里的 `\x01` 会被过滤（它是二次信道的开关）；校验全在核心 crate
/// （[`ham_web_thor::encode_message`] / [`ham_web_thor::encode_picture_message`]）。
/// 本 crate 是 GPL-3.0，发端只能在这个独立 worker 里。
#[wasm_bindgen]
pub fn encode(params: &JsValue) -> Result<JsValue, JsValue> {
  let mode =
    param(params, "mode").map_or_else(|| "16".to_owned(), |v| v.as_string().unwrap_or_default());
  let text = param(params, "text")
    .and_then(|v| v.as_string())
    .unwrap_or_default();
  let id = param(params, "id")
    .and_then(|v| v.as_string())
    .unwrap_or_default();
  let center = param(params, "centerHz").map(|v| v.as_f64().unwrap_or(f64::NAN));
  let encoded = match picture_param(params)? {
    Some(p) => ham_web_thor::encode_picture_message(
      &text,
      &mode,
      center,
      (p.width, p.height),
      p.color,
      &p.rgb,
    ),
    None => ham_web_thor::encode_message(&text, &mode, center, &id),
  };
  let (samples, rate) = encoded.map_err(|e| JsValue::from_str(&e.code()))?;
  encode_result(&samples, rate)
}
