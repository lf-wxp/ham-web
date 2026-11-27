//! WEFAX 解码器的 Web Worker 绑定。
//!
//! 把纯 Rust 的 [`ham_web_wefax`] 解码器通过 wasm-bindgen 暴露给 JS，配合
//! `public/decode-worker.js`（统一解码 Worker 协议）在专用线程运行。
//!
//! 构建方式：`wasm-bindgen --target no-modules --out-name wefax_worker`。

use ham_web_wefax::Modulation;
use ham_web_wefax::fax::{self, Ioc, LevelSource, Params};
use wasm_bindgen::prelude::*;

/// 在 Worker 线程解码 WEFAX 音频字节，返回 `{ width, height, gray: Uint8Array, format, … }`。
///
/// 统一解码 Worker 协议：导出名固定为 `decode(bytes, params)`。`params`：
/// - `format`：`"fax"`（缺省，真实传真：连续的调频副载波、IOC 576 / 288、相位线）/ `"simplified"`
///   （本站最初自拟的简化模型：每行 380 像素 + 行首同步脉冲，只用于合成录音）；
/// - `lpm`：行速率（`"120"` 等，缺省 `"120"`；只对 `"fax"` 有效）；
/// - `ioc`：合作指数（`"576"` / `"288"`，缺省 `"576"`；只对 `"fax"` 有效）；
/// - `modulation`：`"auto"`（缺省）/ `"fm"` / `"am"`（只对 `"simplified"` 有效）。
///
/// 不认识的值一律按缺省处理。结果里的 `format` 是实际采用的格式：
/// - `"fax"`：另有 `lpm`（实测行速率）、`phasingLines`（找到的相位线条数，0 = 没找到、行首位置没有依据）、
///   `blackHz` / `whiteHz`（实际采用的黑白电平）、`levels`（`"phasing"` / `"nominal"` / `"stretched"`）；
/// - `"simplified"`：另有 `modulation`（实际采用的 `"fm"` / `"am"`）。
#[wasm_bindgen]
pub fn decode(bytes: &[u8], params: &JsValue) -> Result<JsValue, JsValue> {
  let text = |key: &str| {
    js_sys::Reflect::get(params, &key.into())
      .ok()
      .and_then(|v| v.as_string())
  };
  let obj = js_sys::Object::new();
  // 错误码由 `ham_web_wefax::Error::code()` 统一给出（`NoSync` 已是 `no-signal:WEFAX`），
  // 这里不再二次映射 —— 同一件事两处判断迟早会漂移。
  let fail = |e: ham_web_wefax::Error| JsValue::from_str(e.code());
  let image = if text("format").as_deref() == Some("simplified") {
    let force = match text("modulation").as_deref() {
      Some("fm") => Some(Modulation::Fm),
      Some("am") => Some(Modulation::Am),
      _ => None,
    };
    let decoded = ham_web_wefax::decode_with(bytes, force).map_err(fail)?;
    set(&obj, "format", JsValue::from_str("simplified"))?;
    set(
      &obj,
      "modulation",
      JsValue::from_str(decoded.modulation.code()),
    )?;
    decoded.image
  } else {
    let lpm = text("lpm")
      .and_then(|v| v.parse::<u32>().ok())
      .filter(|v| fax::LPM_CHOICES.contains(v))
      .unwrap_or(120);
    let ioc = text("ioc")
      .and_then(|v| Ioc::from_code(&v))
      .unwrap_or(Ioc::I576);
    let decoded = fax::decode(bytes, Params { lpm, ioc }).map_err(fail)?;
    set(&obj, "format", JsValue::from_str("fax"))?;
    set(&obj, "lpm", JsValue::from_f64(decoded.lpm_measured))?;
    set(
      &obj,
      "phasingLines",
      JsValue::from(decoded.phasing_lines.unwrap_or(0) as u32),
    )?;
    set(&obj, "blackHz", JsValue::from_f64(decoded.black_hz))?;
    set(&obj, "whiteHz", JsValue::from_f64(decoded.white_hz))?;
    let levels = match decoded.levels {
      LevelSource::Phasing => "phasing",
      LevelSource::Nominal => "nominal",
      LevelSource::Stretched => "stretched",
    };
    set(&obj, "levels", JsValue::from_str(levels))?;
    decoded.image
  };
  set(&obj, "width", JsValue::from(image.width as u32))?;
  set(&obj, "height", JsValue::from(image.height as u32))?;
  set(&obj, "gray", to_uint8(&image.gray))?;
  Ok(obj.into())
}

fn set(obj: &js_sys::Object, key: &str, value: JsValue) -> Result<(), JsValue> {
  js_sys::Reflect::set(obj.as_ref(), &JsValue::from_str(key), &value).map(|_| ())
}

/// 把字节拷贝为一个独立的 JS `Uint8Array`（可被安全克隆）。
fn to_uint8(data: &[u8]) -> JsValue {
  let arr = js_sys::Uint8Array::new_with_length(data.len() as u32);
  arr.copy_from(data);
  arr.into()
}
