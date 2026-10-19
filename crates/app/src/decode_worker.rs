//! 解码器页面的共享 Worker 桥：统一协议（`{ codec, bytes, params }` → `{ ok, result }`）。
//!
//! 各解码页（APT / SSTV / WSPR / RTTY / NAVTEX / LRPT / DominoEX / Olivia / FT8 / Feld Hell /
//! WEFAX / 软 TNC）此前
//! 各自维护一份「spawn worker + 读文件 + 组消息 + 拆回复」的样板，统一入口见
//! `public/decode-worker.js`；这里把其中的公共部分（spawn / 读文件 / 组信封 /
//! 超时兜底 / 按码本地化）收敛成一份，代际、取消与重置的编排在
//! [`crate::decode_session::DecodeSession`]（页面一律经它接入，不再手写生命周期）。

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use web_sys::{File, Worker};

use crate::i18n::{t, tf};
use crate::util::js_error_message;

/// 解码超时（秒）：录音过长或 worker 卡死（OOM / 协议错误未回信）时的兜底。
pub const DECODE_TIMEOUT_SECS: u32 = 300;

/// 在组件体创建定时器槽：完成、取消或页面卸载时清除，不能在异步续体里注册清理。
pub fn decode_timeout_slot(processing: RwSignal<bool>) -> StoredValue<Option<TimeoutHandle>> {
  let timer = StoredValue::new(None::<TimeoutHandle>);
  Effect::new(move |_| {
    if !processing.get() {
      clear_decode_timeout(timer);
    }
  });
  on_cleanup(move || clear_decode_timeout(timer));
  timer
}

/// 清除当前批次定时器；已卸载的槽位直接忽略。
pub fn clear_decode_timeout(timer: StoredValue<Option<TimeoutHandle>>) {
  if let Some(Some(handle)) = timer.try_get_value() {
    handle.clear();
    timer.set_value(None);
  }
}

/// 解码超时兜底：`DECODE_TIMEOUT_SECS` 后若该批次仍在处理中（代际未变且 `processing`
/// 未复位），调用 `cancel` 终止并把错误文案置为 `message()` —— 正常完成时 `processing`
/// 已复位，回调直接跳过。文案由调用方给（词条字面量写在调用点；LRPT 的录音大、耗时长，
/// 提示里要说明原因）。各解码页经 `DecodeSession` 共用这一份实现。
pub fn arm_decode_timeout_with(
  timer: StoredValue<Option<TimeoutHandle>>,
  generation: StoredValue<u32>,
  batch: u32,
  processing: RwSignal<bool>,
  error: RwSignal<Option<String>>,
  cancel: impl Fn(()) + 'static,
  message: impl Fn() -> String + 'static,
) {
  clear_decode_timeout(timer);
  let handle = set_timeout_with_handle(
    move || {
      if generation.try_get_value() == Some(batch) && processing.try_get_untracked() == Some(true) {
        clear_decode_timeout(timer);
        cancel(());
        error.set(Some(message()));
      }
    },
    std::time::Duration::from_secs(u64::from(DECODE_TIMEOUT_SECS)),
  );
  if let Ok(handle) = handle {
    timer.set_value(Some(handle));
  }
}

/// 消息闭包随组件释放，避免每次进出页面遗忘一个 Closure。
///
/// **必须在组件体（有 Owner）内调用**：没有 Owner 时 `on_cleanup` 是空操作，闭包会在本函数
/// 返回时立即释放，返回的 `Function` 随即悬空。
pub fn retain_message_handler(
  callback: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::MessageEvent)>,
) -> js_sys::Function {
  let callback = send_wrapper::SendWrapper::new(callback);
  let function = callback
    .as_ref()
    .unchecked_ref::<js_sys::Function>()
    .clone();
  on_cleanup(move || drop(callback));
  function
}

/// 把 Worker 的 `error` 事件转成一条合成的 `{ ok:false, error }` 消息，交给页面原有的
/// `onmessage` 处理（走同一套代际 / 超时 / 本地化路径，不必逐页修改）。
///
/// 错误码区分两种来源：没有 `message` 的是加载失败（脚本 404 / 被拦截 / 语法错误），
/// 有 `message` 的是运行期异常（如 wasm trap）——两者的修复动作不同，不能都报
/// 「解码模块加载失败」误导排障。
///
/// 参数类型必须是 `Event` 而不是 `ErrorEvent`：脚本加载失败时浏览器派发的是**普通
/// `Event`**（没有 `message` 属性），声明成 `ErrorEvent` 会让 wasm-bindgen 在转换时抛错、
/// 消息永远到不了页面（实测：`isErrorEvent === false`、`ev.message === undefined`）。
/// 所以 `message` 按属性取，取不到就按加载失败处理。
///
/// 转发体是 Rust 闭包而不是 `new Function`：动态求值在严格 CSP（禁 `unsafe-eval`）下会被
/// 拦截，届时 Worker 不回信、只能等 300 秒超时。闭包的生命周期与 `onmessage` 一致，
/// 由 [`retain_error_handler`] 挂在调用方的 Owner 上。
pub fn error_handler(onmessage: &js_sys::Function) -> Closure<dyn FnMut(web_sys::Event)> {
  let handler = onmessage.clone();
  Closure::new(move |ev: web_sys::Event| {
    ev.prevent_default();
    let message = js_sys::Reflect::get(&ev, &"message".into())
      .ok()
      .and_then(|v| v.as_string())
      .unwrap_or_default();
    let error = if message.is_empty() {
      "worker-load-failed".to_owned()
    } else {
      format!("worker-run-failed:{message}")
    };
    // 合成一条与 Worker 回信同形的消息（`{ data: { ok, error } }`），页面侧只认这一条路径。
    let data = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&data, &"ok".into(), &JsValue::FALSE);
    let _ = js_sys::Reflect::set(&data, &"error".into(), &JsValue::from_str(&error));
    let event = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&event, &"data".into(), &data);
    let _ = handler.call1(&JsValue::NULL, &event);
  })
}

/// `error` 闭包随组件释放；所有权约定与 [`retain_message_handler`] 完全相同：
/// **必须在组件体（有 Owner）内调用**。
pub fn retain_error_handler(callback: Closure<dyn FnMut(web_sys::Event)>) -> js_sys::Function {
  let callback = send_wrapper::SendWrapper::new(callback);
  let function = callback
    .as_ref()
    .unchecked_ref::<js_sys::Function>()
    .clone();
  on_cleanup(move || drop(callback));
  function
}

/// 创建解码 Worker（统一入口，不再按解码器分文件）。
///
/// 除 `onmessage` 外还挂 `onerror`：`/decode-worker.js` 或 wasm 胶水 404 / 被拦截 / 语法错误时，
/// 浏览器只触发 `error` 事件、不会有任何回信，界面会一直转圈到超时。
///
/// 两个回调的所有权都在调用方（见 [`retain_message_handler`] / [`retain_error_handler`]）：
/// 同一个会话重建 Worker 时复用同一对函数，不必每次新建闭包。
pub fn spawn_worker(onmessage: &js_sys::Function, onerror: &js_sys::Function) -> Option<Worker> {
  let w = Worker::new("/decode-worker.js").ok()?;
  w.set_onmessage(Some(onmessage));
  w.set_onerror(Some(onerror));
  Some(w)
}

/// 终止解码 Worker：先摘掉消息处理函数，再 `terminate()`。
///
/// `terminate()` 只保证后续不再执行，杀不掉**已经排进主线程任务队列**的消息 —— 那些
/// 回调会去调已释放的 Rust 闭包（wasm-bindgen 抛错，污染 e2e 的 `pageerror` 断言）。
/// 清掉 `onmessage` 后，队列里的消息不会再进 Rust。
pub fn shutdown_worker(worker: &Worker) {
  worker.set_onmessage(None);
  worker.set_onerror(None);
  worker.terminate();
}

/// 读取用户选择的文件为 `ArrayBuffer`（留在 JS 堆里，不进 wasm 内存）。
pub async fn read_file_buffer(file: &File) -> Result<js_sys::ArrayBuffer, String> {
  let blob: web_sys::Blob = file.clone().into();
  let buf = wasm_bindgen_futures::JsFuture::from(blob.array_buffer())
    .await
    .map_err(|e| js_error_message(&e))?;
  Ok(buf.unchecked_into())
}

/// 读取用户选择的文件为字节（WAV 等，解码在 Worker 里做）。
pub async fn read_file_bytes(file: &File) -> Result<Vec<u8>, String> {
  let buf = read_file_buffer(file).await?;
  Ok(js_sys::Uint8Array::new(&buf).to_vec())
}

/// 组装并发送统一信封：`{ codec, bytes, params }`（`bytes` 以拷贝方式传入）。
pub fn post_decode(
  worker: &Worker,
  codec: &str,
  bytes: &[u8],
  params: &JsValue,
) -> Result<(), JsValue> {
  let ab = js_sys::ArrayBuffer::new(bytes.len() as u32);
  let view = js_sys::Uint8Array::new(&ab);
  view.copy_from(bytes);
  let msg = envelope(codec, &ab, params);
  worker.post_message(&msg)
}

/// 同 [`post_decode`]，但直接把读文件得到的 `ArrayBuffer` **转移**给 Worker：主线程侧
/// 不再有 `Vec<u8>`、也不再拷出第二份 buffer（128MB 级的 LRPT 录音若走拷贝路径，
/// 页面侧 `Vec` + 新 buffer + 结构化克隆并存，峰值约三倍）。转移后 `buffer` 被分离，
/// 调用方不得再使用；Worker 里 `decode(&[u8])` 仍会把它拷进 wasm 内存一次。
pub fn post_decode_transfer(
  worker: &Worker,
  codec: &str,
  buffer: &js_sys::ArrayBuffer,
  params: &JsValue,
) -> Result<(), JsValue> {
  let msg = envelope(codec, buffer, params);
  let transfers = js_sys::Array::new();
  transfers.push(buffer);
  worker.post_message_with_transfer(&msg, &transfers)
}

/// 组装 `{ codec, bytes, params }` 信封（`Reflect::set` 失败按缺字段处理，Worker
/// 侧会以协议错误拒绝）。
fn envelope(codec: &str, ab: &js_sys::ArrayBuffer, params: &JsValue) -> JsValue {
  let obj = js_sys::Object::new();
  let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("codec"), &JsValue::from_str(codec));
  let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("bytes"), &JsValue::from(ab));
  let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("params"), params);
  obj.into()
}

/// 拆统一回复：`{ ok: bool, result?, error? }` → `Ok(result)` / `Err(error)`。
///
/// 每页拿到 `result`（一个 JS 对象）后按自己的字段解析。`error` 若是 Worker 的错误码
/// （见 [`localize_error`]）会在这里换成当前语言的文案；协议损坏（缺 `error`）时返回
/// 空串 —— 页面侧有 `e.is_empty()` 的词条兜底，这里不硬编码界面文案。
pub fn parse_outcome(data: &JsValue) -> Result<JsValue, String> {
  let ok = js_sys::Reflect::get(data, &"ok".into())
    .ok()
    .and_then(|v| v.as_bool())
    .unwrap_or(false);
  if !ok {
    let err = js_sys::Reflect::get(data, &"error".into())
      .ok()
      .and_then(|v| v.as_string())
      .unwrap_or_default();
    return Err(localize_error(&err));
  }
  js_sys::Reflect::get(data, &"result".into()).map_err(|e| js_error_message(&e))
}

/// 读结果对象的原始字段。`Reflect::get` 对缺失属性返回 `Ok(undefined)` 而不是 `Err`，
/// 所以「缺字段」要靠后面的类型转换失败来发现（见下面的 `get_*`）。
fn raw_field(obj: &JsValue, key: &str) -> Result<JsValue, String> {
  js_sys::Reflect::get(obj, &key.into()).map_err(|_| tf("common.field-is-missing", &[key]))
}

/// 必填数值字段（`undefined` / 非数字都按缺失处理，不静默变成 0）。
pub fn get_f64(obj: &JsValue, key: &str) -> Result<f64, String> {
  raw_field(obj, key)?
    .as_f64()
    .ok_or_else(|| tf("common.field-is-missing", &[key]))
}

/// 必填非负整数字段。NaN / 无穷 / 负数 / 超范围一律按缺失拒绝：`f64 as u32` 对 NaN
/// 会静默得 0，对超范围值饱和到极值 —— 两种结果都会在后面的 `ImageData` 构造处
/// 以英文 DOMException 报错，不如在这里按协议错误说清楚。
pub fn get_u32(obj: &JsValue, key: &str) -> Result<u32, String> {
  let v = get_f64(obj, key)?;
  if !v.is_finite() || !(0.0..=f64::from(u32::MAX)).contains(&v) {
    return Err(tf("common.field-is-missing", &[key]));
  }
  Ok(v as u32)
}

/// 必填字符串字段。
pub fn get_str(obj: &JsValue, key: &str) -> Result<String, String> {
  raw_field(obj, key)?
    .as_string()
    .ok_or_else(|| tf("common.field-is-missing", &[key]))
}

/// 必填布尔字段。
pub fn get_bool(obj: &JsValue, key: &str) -> Result<bool, String> {
  raw_field(obj, key)?
    .as_bool()
    .ok_or_else(|| tf("common.field-is-missing", &[key]))
}

/// 必填 `Uint8Array` 字段（拷出为 `Vec<u8>`）。
pub fn get_u8s(obj: &JsValue, key: &str) -> Result<Vec<u8>, String> {
  let v = raw_field(obj, key)?;
  let arr: js_sys::Uint8Array = v
    .dyn_into()
    .map_err(|_| tf("common.field-is-not-a", &[key]))?;
  Ok(arr.to_vec())
}

/// 把 `码` / `码:参数` 拆开。参数里可以再含 `:`（只在第一个冒号处切）。
fn split_code(raw: &str) -> (&str, &str) {
  raw.split_once(':').unwrap_or((raw, ""))
}

/// 拆 Olivia 的参数错误：`<音调数>,<带宽>` 或 `<音调数>,<带宽>@<中心频率>`。
///
/// worker 用带 `@` 的形态表示「音调带在采样率下摆不下」，与「参数组合不在标准表内」
/// 是两种不同的补救动作（换个录音 vs 换个采样率），所以文案也要分开。
fn split_olivia_params(arg: &str) -> (&str, &str, Option<&str>) {
  let (pair, center) = arg.split_once('@').unwrap_or((arg, ""));
  let (tones, bandwidth) = pair.split_once(',').unwrap_or((pair, ""));
  // 只有「音调数也在」才算带中心频率的形态：`@1500` 这种残缺码应当走两占位那条文案，
  // 而不是显示成「音调带摆不下（空,空,1500）」。
  let center = (!center.is_empty() && !tones.is_empty()).then_some(center);
  (tones, bandwidth, center)
}

/// Worker 的错误是英文错误码而不是界面文案（Worker 在 wasm 里，拿不到当前语言），
/// 这里按码取各语言词条。不认识的错误（wasm trap、协议错误等）原样返回 ——
/// 它们本来就是英文细节。
pub(crate) fn localize_error(raw: &str) -> String {
  let (code, arg) = split_code(raw);
  match code {
    "not-wav" => t("radio.not-a-valid-wav"),
    "unsupported-format" => t("tools.decode-err-unsupported-format"),
    "truncated" => t("tools.decode-err-truncated"),
    "no-audio" => t("tools.decode-err-no-audio"),
    "too-short" => t("tools.decode-err-too-short"),
    "no-vis" => t("tools.decode-err-no-vis"),
    "no-signal" => tf("tools.decode-err-no-signal", &[arg]),
    "unsupported-submode" => tf("tools.decode-err-unsupported-submode", &[arg]),
    "olivia-params" => {
      let (tones, bandwidth, center) = split_olivia_params(arg);
      match center {
        Some(center) => tf(
          "tools.decode-err-olivia-params-range",
          &[tones, bandwidth, center],
        ),
        None => tf("tools.decode-err-olivia-params", &[tones, bandwidth]),
      }
    }
    "mark-hz-out-of-range" => tf("tools.decode-err-mark-hz", &[arg]),
    "base-hz-out-of-range" => tf("tools.decode-err-base-hz", &[arg]),
    "ft8-sample-rate" => tf("tools.decode-err-ft8-sample-rate", &[arg]),
    "need-sample-rate" => tf("tools.decode-err-lrpt-sample-rate", &[arg]),
    "need-stereo" => tf("tools.decode-err-lrpt-stereo", &[arg]),
    "worker-load-failed" => t("tools.decode-err-worker-load-failed"),
    "worker-run-failed" => tf("tools.decode-err-worker-run-failed", &[arg]),
    "rate-too-low" => tf("tools.decode-err-rate-too-low", &[arg]),
    "input-too-large" => tf("tools.decode-err-input-too-large", &[arg]),
    "image-incomplete" => tf("tools.decode-err-lrpt-incomplete", &[arg]),
    _ => raw.to_owned(),
  }
}

#[cfg(test)]
mod tests {
  use super::{split_code, split_olivia_params};

  #[test]
  fn error_codes_split_at_the_first_colon() {
    assert_eq!(split_code("no-signal:RTTY"), ("no-signal", "RTTY"));
    assert_eq!(split_code("not-wav"), ("not-wav", ""));
    // 参数里的冒号不再切（未知错误整段原样返回，不能被误切）。
    assert_eq!(split_code("a:b:c"), ("a", "b:c"));
    assert_eq!(
      split_code("unknown codec: x"),
      ("unknown codec", " x"),
      "含空格的前缀不是已知码，调用方会整段回退"
    );
  }

  #[test]
  fn olivia_params_split_tones_bandwidth_and_optional_centre() {
    assert_eq!(
      split_olivia_params("32,1000"),
      ("32", "1000", None),
      "标准表外的组合只有两个字段"
    );
    assert_eq!(
      split_olivia_params("32,1000@1500"),
      ("32", "1000", Some("1500")),
      "带 @ 的第三形态是音调带越界的中心频率"
    );
    // 字段缺失时不能 panic，交给占位符显示空串。
    assert_eq!(split_olivia_params("32"), ("32", "", None));
    // 只有 `@` 没有音调数：按残缺码处理，别走「音调带越界」那条文案。
    assert_eq!(split_olivia_params("@1500"), ("", "", None));
  }
}
