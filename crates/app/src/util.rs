//! 浏览器相关的小工具。

use std::sync::atomic::{AtomicU32, Ordering};

use wasm_bindgen::JsValue;

/// 当前时间（毫秒时间戳）。
pub fn now_ms() -> i64 {
  js_sys::Date::now() as i64
}

/// `[0, 1)` 随机数。
pub fn random() -> f64 {
  js_sys::Math::random()
}

/// 生成页面内唯一 ID（替代 React `useId`）。
pub fn unique_id(prefix: &str) -> String {
  static NEXT: AtomicU32 = AtomicU32::new(1);
  format!("{prefix}-{}", NEXT.fetch_add(1, Ordering::Relaxed))
}

pub fn window() -> web_sys::Window {
  web_sys::window().expect("window should exist in browser")
}

pub fn document() -> web_sys::Document {
  window().document().expect("document should exist")
}

pub fn body() -> Option<web_sys::HtmlElement> {
  document().body()
}

/// 设置页面标题。
pub fn set_title(title: &str) {
  document().set_title(title);
}

/// 弹出原生提示框。
pub fn alert(msg: &str) {
  let _ = window().alert_with_message(msg);
}

/// 给 `<body>` 增删类名。
pub fn body_class(class: &str, on: bool) {
  if let Some(b) = body() {
    let list = b.class_list();
    let _ = if on {
      list.add_1(class)
    } else {
      list.remove_1(class)
    };
  }
}

/// `localStorage` 读写，失败（隐私模式/配额）时静默忽略。
pub mod storage {
  use serde::Serialize;
  use serde::de::DeserializeOwned;

  fn local() -> Option<web_sys::Storage> {
    super::window().local_storage().ok().flatten()
  }

  pub fn get(key: &str) -> Option<String> {
    local()?.get_item(key).ok().flatten()
  }

  pub fn set(key: &str, value: &str) {
    if let Some(s) = local() {
      let _ = s.set_item(key, value);
    }
  }

  pub fn remove(key: &str) {
    if let Some(s) = local() {
      let _ = s.remove_item(key);
    }
  }

  pub fn get_json<T: DeserializeOwned>(key: &str) -> Option<T> {
    serde_json::from_str(&get(key)?).ok()
  }

  pub fn set_json<T: Serialize>(key: &str, value: &T) {
    if let Ok(s) = serde_json::to_string(value) {
      set(key, &s);
    }
  }
}

/// 把 `JsValue` 错误转为可读字符串。
pub fn js_error_message(err: &JsValue) -> String {
  if let Some(e) = err.dyn_ref_error() {
    return e;
  }
  err.as_string().unwrap_or_else(|| format!("{err:?}"))
}

trait DynRefError {
  fn dyn_ref_error(&self) -> Option<String>;
}

impl DynRefError for JsValue {
  fn dyn_ref_error(&self) -> Option<String> {
    use wasm_bindgen::JsCast;
    self
      .dyn_ref::<js_sys::Error>()
      .map(|e| String::from(e.message()))
  }
}
