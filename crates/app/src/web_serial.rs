//! Web Serial 胶水：KISS TNC 与 VNA 读数两页共用。
//!
//! `web-sys` 的 Serial 绑定还在 unstable 特性后面，这里统一走 `Reflect` 调 JS，
//! 把「取 `navigator.serial` / 调方法 / 等 Promise / 开关端口」收在一处 —— 两个页面
//! 各抄一份时，修一处坑（例如 `Reflect::apply` 的实参展开）另一处就会漏掉。

use wasm_bindgen::{JsCast, JsValue};
use web_sys::js_sys;

/// 一个已打开的串口。
pub struct Conn {
  pub port: JsValue,
  pub reader: JsValue,
  pub writer: JsValue,
}

/// `navigator.serial`；浏览器不支持（Firefox / Safari）时返回 `None`。
///
/// `Reflect::get` 对缺失属性返回 `Ok(undefined)` 而不是 `Err`，必须显式过滤，否则 `supported`
/// 永远为真、「不支持」提示不出现，点「连接」还会对 `undefined` 调方法抛 TypeError。
pub fn serial() -> Option<JsValue> {
  js_sys::Reflect::get(&web_sys::window()?.navigator(), &"serial".into())
    .ok()
    .filter(|v| !v.is_undefined() && !v.is_null())
}

pub fn prop(obj: &JsValue, key: &str) -> Result<JsValue, JsValue> {
  js_sys::Reflect::get(obj, &key.into()).map_err(|e| format!("{e:?}").into())
}

pub async fn call(obj: &JsValue, method: &str, args: &[JsValue]) -> Result<JsValue, JsValue> {
  wasm_bindgen_futures::JsFuture::from(start_call(obj, method, args)?).await
}

/// 调用 `obj.method(...args)` 并拿到它返回的 Promise（不等待）。
fn start_call(obj: &JsValue, method: &str, args: &[JsValue]) -> Result<js_sys::Promise, JsValue> {
  let f = prop(obj, method)?;
  let f = f
    .dyn_into::<js_sys::Function>()
    .map_err(|_| "not a function")?;
  // `Reflect::apply` 把 args 展开成实参列表 —— `f.call1(obj, arr)` 会把整个
  // 数组当成唯一实参，`open` 读不到 baudRate、`write` 的 BufferSource 也不对。
  let arr = js_sys::Array::from_iter(args.iter().cloned());
  let promise = js_sys::Reflect::apply(&f, obj, &arr)?;
  // 保留 rejection 的原始对象（DOMException 等）：调用方要靠 `name` 区分
  // 「用户取消端口选择」（NotFoundError）与真实错误，换成字符串就丢了这个信息。
  Ok(js_sys::Promise::from(promise))
}

/// 超时错误的固定文本（调用方用 [`is_timeout`] 识别）。
const TIMEOUT_MSG: &str = "serial timeout";

/// 这个错误是不是 [`read_chunk_within`] 的超时。
pub fn is_timeout(e: &JsValue) -> bool {
  e.as_string().is_some_and(|s| s == TIMEOUT_MSG)
}

/// `ms` 毫秒后 reject 的 Promise，用来和读操作 `Promise.race`。
fn timeout_promise(ms: i32) -> js_sys::Promise {
  js_sys::Promise::new(&mut |_resolve, reject| {
    let fire = wasm_bindgen::closure::Closure::once_into_js(move || {
      let _ = reject.call1(&JsValue::NULL, &JsValue::from_str(TIMEOUT_MSG));
    });
    if let Some(w) = web_sys::window() {
      let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(fire.unchecked_ref(), ms);
    }
  })
}

pub fn call_sync(obj: &JsValue, method: &str) -> Result<JsValue, JsValue> {
  let f = prop(obj, method)?;
  let f = f
    .dyn_into::<js_sys::Function>()
    .map_err(|_| "not a function")?;
  f.call0(obj)
}

/// 弹出端口选择框并以 `baud` 打开，同时拿到读写句柄。
pub async fn open_serial(baud: u32) -> Result<Conn, JsValue> {
  // 不支持时页面根本不给点「连接」，这里只在未来放开按钮时兜底 ——
  // 错误细节用英文（与 worker 错误同惯例）。
  let serial = serial().ok_or_else(|| JsValue::from_str("Web Serial not supported"))?;
  let port = call(&serial, "requestPort", &[]).await?;
  let opts = js_sys::Object::new();
  js_sys::Reflect::set(&opts, &"baudRate".into(), &baud.into())?;
  call(&port, "open", &[opts.into()]).await?;
  let reader = call_sync(&prop(&port, "readable")?, "getReader")?;
  let writer = call_sync(&prop(&port, "writable")?, "getWriter")?;
  Ok(Conn {
    port,
    reader,
    writer,
  })
}

pub async fn close_serial(conn: Conn) {
  let _ = call(&conn.reader, "cancel", &[]).await;
  let _ = call_sync(&conn.reader, "releaseLock");
  // 有在途写入时 writer 的锁释放会抛错，随后 port.close() 因流仍被锁而 reject、端口保持打开：
  // 先 abort 丢弃队列并结束写流，再释放锁。
  let _ = call(&conn.writer, "abort", &[]).await;
  let _ = call_sync(&conn.writer, "releaseLock");
  let _ = call(&conn.port, "close", &[]).await;
}

/// 这个错误是不是「用户在端口选择框里点了取消」（不该当成错误提示）。
pub fn is_user_cancel(e: &JsValue) -> bool {
  prop(e, "name")
    .ok()
    .and_then(|n| n.as_string())
    .is_some_and(|n| n == "NotFoundError")
}

/// 读一块数据：`Ok(Some(bytes))` / 流结束 `Ok(None)` / 读失败 `Err`；
/// `ms` 毫秒内没有数据就以 [`is_timeout`] 错误返回。
///
/// 超时后那次 `read()` 仍悬着：调用方应随后 [`close_serial`]（`reader.cancel()` 会让它结束）。
pub async fn read_chunk_within(reader: &JsValue, ms: i32) -> Result<Option<Vec<u8>>, JsValue> {
  let read = start_call(reader, "read", &[])?;
  let raced = js_sys::Promise::race(&js_sys::Array::of2(&read, &timeout_promise(ms)));
  let chunk = wasm_bindgen_futures::JsFuture::from(raced).await?;
  chunk_bytes(&chunk)
}

fn chunk_bytes(chunk: &JsValue) -> Result<Option<Vec<u8>>, JsValue> {
  if prop(chunk, "done").ok().is_some_and(|d| d.is_truthy()) {
    return Ok(None);
  }
  let v = prop(chunk, "value")?;
  Ok(Some(
    v.dyn_into::<js_sys::Uint8Array>()
      .map(|a| a.to_vec())
      .unwrap_or_default(),
  ))
}

/// 写一段字节。
pub async fn write_bytes(writer: &JsValue, bytes: &[u8]) -> Result<(), JsValue> {
  let arr = js_sys::Uint8Array::from(bytes);
  call(writer, "write", &[arr.into()]).await.map(|_| ())
}
