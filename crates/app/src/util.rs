//! 浏览器相关的小工具。

use std::fmt::Write as _;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::i18n::t;
use wasm_bindgen::JsValue;

/// 当前时间（毫秒时间戳）。
pub fn now_ms() -> i64 {
  js_sys::Date::now() as i64
}

/// 某时间戳（毫秒）所在的本地日期 `YYYY-MM-DD`。
pub fn local_day(ms: f64) -> String {
  let d = js_sys::Date::new(&JsValue::from_f64(ms));
  format!(
    "{:04}-{:02}-{:02}",
    d.get_full_year(),
    d.get_month() + 1,
    d.get_date()
  )
}

/// 今天的本地日期 `YYYY-MM-DD`。
pub fn local_today() -> String {
  local_day(js_sys::Date::now())
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

/// 对 URL 查询参数值做 percent-encode（等价 JS 的 `encodeURIComponent`）。
///
/// 拼 `/browse?q={query}` 这类链接时必须编码：查询串含空格、`&`、`#` 时不编码会截断
/// URL 或凭空多出查询参数。
///
/// 用纯 Rust 实现而不是 `js_sys::encode_uri_component`，是为了让 `cargo test` 在原生
/// 目标下也能覆盖到它（wasm-bindgen 的接口在原生目标下调用即 panic）。
#[must_use]
pub fn encode_uri_component(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  for b in s.as_bytes() {
    match b {
      b'A'..=b'Z'
      | b'a'..=b'z'
      | b'0'..=b'9'
      | b'-'
      | b'_'
      | b'.'
      | b'!'
      | b'~'
      | b'*'
      | b'\''
      | b'('
      | b')' => out.push(char::from(*b)),
      // 其余（含多字节 UTF-8 与 `&` `=` `#` `%` 等）逐字节编码。
      _ => write!(out, "%{b:02X}").expect("写入 String 不会失败"),
    }
  }
  out
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

/// 设置页面标题（英文模式下按 [`crate::i18n`] 词典翻译）。
pub fn set_title(title: &str) {
  document().set_title(&crate::i18n::t(title));
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

/// `localStorage` 读写。读取失败时返回 `None`；写入失败（通常是配额已满）时在 `window` 上派发
/// [`storage::WRITE_FAILED_EVENT`] 事件，由全局提示条提醒用户导出备份。
pub mod storage {
  use serde::Serialize;
  use serde::de::DeserializeOwned;

  /// 写入失败时派发的事件名，`detail` 为失败的 key。
  pub const WRITE_FAILED_EVENT: &str = "ham-storage-write-failed";
  /// 主流浏览器每个源约可存 5M 个 UTF-16 字符（key + value）。
  pub const QUOTA_UNITS: usize = 5 * 1024 * 1024;

  fn local() -> Option<web_sys::Storage> {
    super::window().local_storage().ok().flatten()
  }

  pub fn get(key: &str) -> Option<String> {
    local()?.get_item(key).ok().flatten()
  }

  pub fn set(key: &str, value: &str) {
    if let Some(s) = local()
      && s.set_item(key, value).is_err()
    {
      let init = web_sys::CustomEventInit::new();
      init.set_detail(&key.into());
      if let Ok(ev) = web_sys::CustomEvent::new_with_event_init_dict(WRITE_FAILED_EVENT, &init) {
        let _ = super::window().dispatch_event(&ev);
      }
    }
  }

  /// 各 key 的占用（UTF-16 字符数，key + value），从大到小排列。
  pub fn usage() -> Vec<(String, usize)> {
    let Some(s) = local() else {
      return Vec::new();
    };
    let mut out: Vec<(String, usize)> = (0..s.length().unwrap_or(0))
      .filter_map(|i| s.key(i).ok().flatten())
      .map(|k| {
        let len = s
          .get_item(&k)
          .ok()
          .flatten()
          .map_or(0, |v| v.encode_utf16().count());
        let units = k.encode_utf16().count() + len;
        (k, units)
      })
      .collect();
    out.sort_by_key(|(_, b)| std::cmp::Reverse(*b));
    out
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

  /// 写入（失败静默，不派发全局「存储已满」警告）。
  ///
  /// 用于 IndexedDB 已兜底的大数据（如通联日志）的 localStorage 快照：快照写满时
  /// 不应反复弹出警告，真正的数据由 IndexedDB 保证。
  pub fn set_silent(key: &str, value: &str) {
    if let Some(s) = local() {
      let _ = s.set_item(key, value);
    }
  }

  /// [`set_json`] 的静默版本，见 [`set_silent`]。
  pub fn set_json_silent<T: Serialize>(key: &str, value: &T) {
    if let Ok(s) = serde_json::to_string(value) {
      set_silent(key, &s);
    }
  }
}

/// 复制文本到剪贴板（Clipboard API，失败时静默忽略）。
pub fn copy_text(text: &str) {
  let _ = window().navigator().clipboard().write_text(text);
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
    return Err(t("localStorage 不可用"));
  };
  let mut count = 0;
  for (k, v) in map {
    if local.set_item(&k, &v).is_ok() {
      count += 1;
    }
  }
  Ok(count)
}

/// 从备份 JSON **合并**导入：已知类型（日志、收藏、错题本、统计等）做并集 / 累加合并，
/// 其余 key 保留本机不覆盖。返回合并写入的条目数。
pub fn import_backup_merge(json: &str) -> Result<usize, String> {
  let map: std::collections::BTreeMap<String, String> =
    serde_json::from_str(json).map_err(|e| e.to_string())?;
  let Some(local) = window().local_storage().ok().flatten() else {
    return Err(t("localStorage 不可用"));
  };
  let mut count = 0;
  for (k, v) in map {
    let current = local.get_item(&k).ok().flatten();
    if let Some(merged) = ham_web_core::backup_merge::merge_value(&k, current.as_deref(), &v)
      && local.set_item(&k, &merged).is_ok()
    {
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

/// 等待 `ms` 毫秒。
pub async fn sleep(ms: u32) {
  let p = js_sys::Promise::new(&mut |resolve, _| {
    let _ = window().set_timeout_with_callback_and_timeout_and_arguments_0(
      &resolve,
      i32::try_from(ms).unwrap_or(i32::MAX),
    );
  });
  let _ = wasm_bindgen_futures::JsFuture::from(p).await;
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
