//! IndexedDB 轻量 KV 封装：承载大体积数据（通联日志），突破 localStorage 5MB 配额。
//!
//! 单 object store（`kv`）的 key-value 模式；异步 API 基于 `wasm-bindgen-futures` 把
//! 回调式 `Idb*` 请求包装为 `Future`（与 [`crate::util::read_file_text`] 同一套路）。
//! 数据库连接缓存于线程局部，避免重复 open。

use std::cell::RefCell;

use js_sys::Promise;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{IdbDatabase, IdbRequest, IdbTransactionMode};

const DB_NAME: &str = "ham-web";
const STORE: &str = "kv";
const VERSION: u32 = 1;

thread_local! {
  static DB: RefCell<Option<IdbDatabase>> = const { RefCell::new(None) };
}

/// 把 `IdbRequest` 的 onsuccess / onerror 包装为 `Future`，resolve 请求的 `result`。
fn request_future(request: &IdbRequest) -> JsFuture {
  let request = request.clone();
  JsFuture::from(Promise::new(&mut |resolve, reject| {
    let ok_req = request.clone();
    let ok = wasm_bindgen::closure::Closure::once_into_js(move || {
      let result = ok_req.result().unwrap_or(JsValue::UNDEFINED);
      let _ = resolve.call1(&JsValue::NULL, &result);
    });
    let err = wasm_bindgen::closure::Closure::once_into_js(move || {
      let _ = reject.call1(
        &JsValue::NULL,
        &JsValue::from_str("indexedDB request failed"),
      );
    });
    request.set_onsuccess(Some(ok.unchecked_ref()));
    request.set_onerror(Some(err.unchecked_ref()));
  }))
}

/// 打开（或复用）数据库连接；首次会创建 `kv` object store。
async fn open_db() -> Result<IdbDatabase, JsValue> {
  if let Some(db) = DB.with(|c| c.borrow().clone()) {
    return Ok(db);
  }
  let factory = crate::util::window()
    .indexed_db()?
    .ok_or_else(|| JsValue::from_str("indexedDB unavailable"))?;
  let request = factory.open_with_u32(DB_NAME, VERSION)?;

  let upgrade = wasm_bindgen::closure::Closure::once_into_js(move |ev: web_sys::Event| {
    let req = ev
      .target()
      .and_then(|t| t.dyn_into::<web_sys::IdbOpenDbRequest>().ok());
    let Some(db) = req
      .and_then(|r| r.result().ok())
      .map(|v| v.unchecked_into::<IdbDatabase>())
    else {
      return;
    };
    // onupgradeneeded 仅在数据库版本变化（首次创建 / 升级）时触发；已存在同名 store
    // 时 create_object_store 会抛错，静默忽略即可。
    let _ = db.create_object_store(STORE);
  });
  request.set_onupgradeneeded(Some(upgrade.unchecked_ref()));

  let result = request_future(&request).await?;
  let db: IdbDatabase = result.unchecked_into();
  DB.with(|c| *c.borrow_mut() = Some(db.clone()));
  Ok(db)
}

/// 读取键值；键不存在返回 `Ok(None)`。
pub async fn get(key: &str) -> Result<Option<String>, JsValue> {
  let db = open_db().await?;
  let tx = db.transaction_with_str_and_mode(STORE, IdbTransactionMode::Readonly)?;
  let store = tx.object_store(STORE)?;
  let req = store.get(&JsValue::from_str(key))?;
  let result = request_future(&req).await?;
  Ok(result.as_string())
}

/// 写入键值（字符串）。
pub async fn set(key: &str, value: &str) -> Result<(), JsValue> {
  let db = open_db().await?;
  let tx = db.transaction_with_str_and_mode(STORE, IdbTransactionMode::Readwrite)?;
  let store = tx.object_store(STORE)?;
  let req = store.put_with_key(&JsValue::from_str(value), &JsValue::from_str(key))?;
  request_future(&req).await?;
  Ok(())
}

/// 删除键（当前未被引用，保留作为 KV 封装的完整 API，供后续清理 / 迁移使用）。
#[allow(dead_code)]
pub async fn remove(key: &str) -> Result<(), JsValue> {
  let db = open_db().await?;
  let tx = db.transaction_with_str_and_mode(STORE, IdbTransactionMode::Readwrite)?;
  let store = tx.object_store(STORE)?;
  let req = store.delete(&JsValue::from_str(key))?;
  request_future(&req).await?;
  Ok(())
}
