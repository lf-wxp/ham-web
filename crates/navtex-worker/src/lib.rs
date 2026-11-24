//! NAVTEX 解码器的 Web Worker 绑定。
//!
//! 把纯 Rust 的 [`ham_web_navtex`] 解码器通过 wasm-bindgen 暴露给 JS，配合
//! `public/decode-worker.js`（统一解码 Worker 协议）在专用线程运行。
//!
//! 构建方式：`wasm-bindgen --target no-modules --out-name navtex_worker`。
//!
//! 除解码（`decode`）外还导出发端（`encode`）：`ham-web-navtex` 是 GPL-3.0-only，
//! 不能链进 MIT 的主包 wasm（`docs/engineering/licensing.md`），所以 NAVTEX 的合成也只能
//! 在这个独立 worker 里做，页面经 `{ codec, op: "encode", params }` 取回基带样本。

use wasm_bindgen::prelude::*;

fn set(obj: &js_sys::Object, key: &str, value: JsValue) -> Result<(), JsValue> {
  js_sys::Reflect::set(obj.as_ref(), &JsValue::from_str(key), &value).map(|_| ())
}

/// 在 Worker 线程解码 NAVTEX 音频字节，返回 `{ messages }`。
///
/// 统一解码 Worker 协议：导出名固定为 `decode(bytes, params)`，NAVTEX 没有
/// 参数。每个报文：`{ transmitter, category, serial, text, corrected, errors,
/// chars, accepted }`。
#[wasm_bindgen]
pub fn decode(bytes: &[u8], _params: &JsValue) -> Result<JsValue, JsValue> {
  let (samples, rate) = ham_web_apt::parse_wav(bytes).map_err(|e| e.code())?;
  let messages = ham_web_navtex::decode_audio(&samples, rate)
    .filter(|m| !m.is_empty())
    .ok_or_else(|| JsValue::from_str("no-signal:NAVTEX"))?;
  let arr = js_sys::Array::new();
  for m in messages {
    let obj = js_sys::Object::new();
    set(
      &obj,
      "transmitter",
      JsValue::from_str(&m.transmitter.to_string()),
    )?;
    set(&obj, "category", JsValue::from_str(&m.category.to_string()))?;
    set(&obj, "serial", JsValue::from(m.serial))?;
    set(&obj, "text", JsValue::from_str(&m.text))?;
    set(&obj, "corrected", JsValue::from_f64(m.corrected as f64))?;
    set(&obj, "errors", JsValue::from_f64(m.errors as f64))?;
    set(&obj, "chars", JsValue::from_f64(m.chars as f64))?;
    set(&obj, "accepted", JsValue::from_bool(m.accepted))?;
    arr.push(&obj);
  }
  let out = js_sys::Object::new();
  set(&out, "messages", arr.into())?;
  Ok(out.into())
}

/// 合成一条 NAVTEX 广播的基带，返回 `{ samples: Float32Array, sampleRate }`。
///
/// 统一 Worker 协议的编码入口（见 `docs/domain/decode-worker-protocol.md`「编码」）：
/// `params` 是 `{ station, category, serial, text, phasingSecs }`。校验全在
/// [`ham_web_navtex::encode_message`]：字段缺失或类型不对按「空值」处理，
/// 同样落到对应字段的 `encode-params:<字段>` 错误码，不另设一条「缺字段」分支。
#[wasm_bindgen]
pub fn encode(params: &JsValue) -> Result<JsValue, JsValue> {
  let field =
    |key: &str| js_sys::Reflect::get(params, &JsValue::from_str(key)).unwrap_or(JsValue::UNDEFINED);
  let samples = ham_web_navtex::encode_message(
    &field("station").as_string().unwrap_or_default(),
    &field("category").as_string().unwrap_or_default(),
    field("serial").as_f64().unwrap_or(f64::NAN),
    &field("text").as_string().unwrap_or_default(),
    field("phasingSecs").as_f64().unwrap_or(f64::NAN),
  )
  .map_err(|e| JsValue::from_str(e.code()))?;
  let out = js_sys::Object::new();
  // `from(&[f32])` 拷出一块独占的 buffer，decode-worker.js 才能把它整块转移给页面。
  set(
    &out,
    "samples",
    js_sys::Float32Array::from(samples.as_slice()).into(),
  )?;
  set(
    &out,
    "sampleRate",
    JsValue::from(ham_web_navtex::SAMPLE_RATE),
  )?;
  Ok(out.into())
}
