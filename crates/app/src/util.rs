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

/// 触发浏览器下载一个文本文件（用于 ADIF 等导出）。
pub fn download_text(filename: &str, content: &str, mime: &str) {
  use wasm_bindgen::JsCast;
  use web_sys::{Blob, BlobPropertyBag, Url};

  let props = BlobPropertyBag::new();
  props.set_type(mime);
  let parts = js_sys::Array::new();
  parts.push(&JsValue::from_str(content));
  let Ok(blob) = Blob::new_with_str_sequence_and_options(&parts, &props) else {
    return;
  };
  let Ok(url) = Url::create_object_url_with_blob(&blob) else {
    return;
  };
  let Ok(el) = document().create_element("a") else {
    return;
  };
  let a: web_sys::HtmlAnchorElement = el.unchecked_into();
  a.set_href(&url);
  a.set_download(filename);
  if let Some(b) = body() {
    let _ = b.append_child(&a);
    a.click();
    let _ = b.remove_child(&a);
  }
  let _ = Url::revoke_object_url(&url);
}

/// 导出全部 localStorage 数据为 JSON 备份文件。
pub fn export_backup() {
  let Some(local) = window().local_storage().ok().flatten() else {
    return;
  };
  let len = local.length().unwrap_or(0);
  let mut map = std::collections::BTreeMap::new();
  for i in 0..len {
    if let Ok(Some(key)) = local.key(i)
      && let Ok(Some(value)) = local.get_item(&key)
    {
      map.insert(key, value);
    }
  }
  let Ok(json) = serde_json::to_string(&map) else {
    return;
  };
  let d = js_sys::Date::new_0();
  let stamp = format!(
    "{:04}{:02}{:02}",
    d.get_full_year() as i32,
    d.get_month() as i32 + 1,
    d.get_date() as i32
  );
  download_text(
    &format!("ham-backup-{stamp}.json"),
    &json,
    "application/json",
  );
}

/// 从备份 JSON 恢复 localStorage 数据，返回恢复的条目数。
pub fn import_backup(json: &str) -> Result<usize, String> {
  let map: std::collections::BTreeMap<String, String> =
    serde_json::from_str(json).map_err(|e| e.to_string())?;
  let Some(local) = window().local_storage().ok().flatten() else {
    return Err("localStorage 不可用".to_owned());
  };
  let mut count = 0;
  for (k, v) in map {
    if local.set_item(&k, &v).is_ok() {
      count += 1;
    }
  }
  Ok(count)
}

/// 读取文件文本内容。
pub async fn read_file_text(file: &web_sys::File) -> Option<String> {
  use js_sys::Promise;
  use wasm_bindgen::JsCast;
  use wasm_bindgen::closure::Closure;
  use wasm_bindgen_futures::JsFuture;
  use web_sys::FileReader;

  let reader = FileReader::new().ok()?;
  let target = reader.clone();
  let file = file.clone();

  let ok = JsFuture::from(Promise::new(&mut |resolve, reject| {
    let onload = Closure::once_into_js(move || {
      let _ = resolve.call0(&JsValue::NULL);
    });
    let onerror = Closure::once_into_js(move || {
      let _ = reject.call1(&JsValue::NULL, &JsValue::from_str("读取文件失败"));
    });
    target.set_onload(Some(onload.unchecked_ref()));
    target.set_onerror(Some(onerror.unchecked_ref()));
    let _ = target.read_as_text(&file);
  }))
  .await;

  if ok.is_err() {
    return None;
  }
  reader.result().ok().and_then(|v| v.as_string())
}

/// 发送浏览器通知（仅在已授予通知权限时生效）。
pub fn notify(title: &str) {
  use web_sys::{Notification, NotificationPermission};
  if !matches!(Notification::permission(), NotificationPermission::Granted) {
    return;
  }
  let _ = Notification::new(title);
}

/// 请求浏览器通知权限。
pub fn request_notify_permission() {
  use web_sys::{Notification, NotificationPermission};
  if matches!(Notification::permission(), NotificationPermission::Default) {
    let _ = Notification::request_permission();
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
