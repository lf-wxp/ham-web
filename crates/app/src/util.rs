//! 浏览器相关的小工具。

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::i18n::{t, tf};
use ham_web_core::qsl_image;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

/// 当前时间（毫秒时间戳）。
pub fn now_ms() -> i64 {
  js_sys::Date::now() as i64
}

/// 本机时区相对 UTC 的偏移（分钟，东八区 = 480）。
///
/// `getTimezoneOffset()` 返回「本地比 UTC 晚多少分钟」（东八区是 -480），取负才是偏移。
pub fn utc_offset_minutes() -> i32 {
  -js_sys::Date::new_0().get_timezone_offset() as i32
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

/// 「组件还挂着吗」的守卫，配合 `spawn_local` 的异步续体使用。
///
/// `leptos::task::spawn_local` **不会**随组件卸载取消：`await` 之后若再碰组件里创建的
/// 响应式值，而组件已被卸载（信号随之释放），**读**（`get` / `get_value` / `with`）会
/// panic 并带走整个 wasm 实例，**写**（`set` / `update`）则是静默空操作 —— 那笔写永远
/// 丢失且没有任何提示。两种情况都必须靠这个守卫提前返回 ——
/// 与 `ui/popover.rs` 里记的是同一个坑。
///
/// 必须在**组件体**里调用：它靠 `on_cleanup` 置位，而 `on_cleanup` 在没有 Owner 上下文时
/// 是**静默空操作**（事件回调、定时器回调里调用会拿不到保护）。用法：
///
/// ```ignore
/// let alive = util::mount_guard();
/// spawn_local(async move {
///   let data = fetch().await;
///   if !alive() {
///     return; // 用户已经离开这个页面，信号不再有效
///   }
///   result.set(data);
/// });
/// ```
pub fn mount_guard() -> impl Fn() -> bool + Clone + 'static {
  let disposed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
  let flag = disposed.clone();
  leptos::prelude::on_cleanup(move || flag.store(true, Ordering::Relaxed));
  move || !disposed.load(Ordering::Relaxed)
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

/// 复阻抗文案：`R ± jX`（X 为负时用减号，不会写成 `+ j-50.0`）。
///
/// 数字格式由 `f` 决定：`/nec` 按量级挑小数位、`/smith` 固定一位 —— 两边共用同一处
/// 符号逻辑，免得一处改成 `−`、另一处还是 `-`。
#[must_use]
pub fn fmt_z(r: f64, x: f64, f: impl Fn(f64) -> String) -> String {
  let sign = if x < 0.0 { "−" } else { "+" };
  format!("{} {sign} j{}", f(r), f(x.abs()))
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

/// 滚动到元素可见位置，让出「导航栏 + 吸顶页头」的高度。
///
/// 点目录 / 表格跳转时，`scroll_into_view` 把目标滚到视口顶部，开头会被吸顶的
/// 页头盖住 —— 页头高度随译文长度与折行变化，固定的 `scroll-mt-24` 跟不上。
/// 这里按 `main header` 的**实际底边**算偏移（与 `StickyPanel` 同一个选择器口径），
/// 目标顶部落在页头下方 12px 处。
pub fn scroll_to_element(el: &web_sys::Element) {
  let header_bottom = document()
    .query_selector("main header")
    .ok()
    .flatten()
    .map(|h| h.get_bounding_client_rect().bottom())
    .unwrap_or(0.0);
  let y =
    el.get_bounding_client_rect().top() + window().scroll_y().unwrap_or(0.0) - header_bottom - 12.0;
  window().scroll_to_with_x_and_y(0.0, y.max(0.0));
}

/// 页面标题的来源：标题是进入页面时一次性写入的（不在响应式上下文里），切语言时
/// 不会自动重算，因此这里必须只存**可再翻译**的原料（key、实参快照），译文一律在
/// [`apply_title`] 里现查。
///
/// 若存已翻译的文本，切语言时 `t()` 认不出它（既不是 key，也不命中中文反向索引），
/// 标签页会停在旧语言 —— 中文 → 英文之所以看着正常，只是因为反向索引刚好兜住了。
///
/// 这里刻意不用 `RwSignal`：`RwSignal` 是 arena 分配的，会挂到创建时所在的响应式
/// `Owner` 上，随其清理而被释放。首次 `set_title` 发生在页面组件内部，离开该页面后
/// 这个「全局」信号就失效了 —— 之后 `set_title` 静默不生效（标题永远停在第一页），
/// 切语言时 `refresh_title` 访问已释放的信号则直接 panic。标题本身不需要响应式，
/// 用普通 `RefCell` 即可。
#[derive(Clone)]
enum TitleSource {
  /// 从未设置过标题。
  None,
  /// 语义 key（`shell.solar-data`），或运行时才拿到的中文原文（由反向索引兜底）。
  Key(String),
  /// 带占位符的 key + 已确定的实参（如打印页的「打印：错题集」）。
  ///
  /// 实参存**快照**而不是闭包：闭包会捕获页面内的信号，页面卸载后该信号被释放，
  /// 切语言时 `refresh_title` 再调用它就会 panic。
  WithArgs { key: String, args: Vec<String> },
}

thread_local! {
  static TITLE: RefCell<TitleSource> = const { RefCell::new(TitleSource::None) };
}

/// 设置页面标题（浏览器标签，按 [`crate::i18n`] 词典翻译）。
///
/// 传**语义 key**（`set_title("shell.solar-data")`）；只有运行时才拿到的文案
/// （registry 的模块标题、打印页的试卷名）才传中文原文，由反向索引兜底。
pub fn set_title(title: &str) {
  TITLE.with(|slot| *slot.borrow_mut() = TitleSource::Key(title.to_owned()));
  apply_title();
}

/// 设置带占位符的页面标题：`key` 查词典后替换实参，实参本身也过一次词典。
///
/// 实参过词典是为了让 `set_title_with_args("radio.print", &["错题集"])` 这类调用
/// 在切到英文时也能得到「Print: Mistakes」而不是中英混排。
pub fn set_title_with_args(key: &str, args: &[&str]) {
  TITLE.with(|slot| {
    *slot.borrow_mut() = TitleSource::WithArgs {
      key: key.to_owned(),
      args: args.iter().map(|a| (*a).to_owned()).collect(),
    };
  });
  apply_title();
}

/// 语言切换后按当前语言重设标题；未设置过标题时为空操作。
pub fn refresh_title() {
  apply_title();
}

fn apply_title() {
  // 先取出快照再查词典：不把 `RefCell` 借用带进 `t()`（它会订阅响应式信号）。
  // 空 key 保持旧行为 —— 不覆盖 `<title>`，初始 HTML 里的标题更合适。
  let source = TITLE.with(|slot| match &*slot.borrow() {
    TitleSource::None => None,
    TitleSource::Key(k) if k.is_empty() => None,
    other => Some(other.clone()),
  });
  let title = match source {
    None => return,
    Some(TitleSource::Key(k)) => t(&k),
    Some(TitleSource::WithArgs { key, args }) => {
      let resolved: Vec<String> = args.iter().map(|a| t(a)).collect();
      let refs: Vec<&str> = resolved.iter().map(String::as_str).collect();
      tf(&key, &refs)
    }
    Some(TitleSource::None) => return,
  };
  document().set_title(&title);
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

  /// 写入（失败静默，不派发全局「存储已满」警告），返回是否真的写成功。
  ///
  /// 用于 IndexedDB 已兜底的大数据（如通联日志）的 localStorage 快照：快照写满时
  /// 不应反复弹出警告，真正的数据由 IndexedDB 保证。返回值交给调用方判断这一笔
  /// 是否落地 —— [`crate::kv`] 靠它保证「值 + 版本号」这一对要么一起新、要么一起旧。
  #[must_use]
  pub fn set_silent(key: &str, value: &str) -> bool {
    local().is_some_and(|s| s.set_item(key, value).is_ok())
  }

  /// [`set_json`] 的静默版本，见 [`set_silent`]。
  pub fn set_json_silent<T: Serialize>(key: &str, value: &T) {
    if let Ok(s) = serde_json::to_string(value) {
      let _ = set_silent(key, &s);
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

  // 三步里任何一步失败都要让用户知道：静默 `return` 的话页面上什么都没发生，用户会以为
  // 文件已经下载了（`share_score::download_bytes` 失败时同样报这条）。
  let fail = || crate::ui::push_error(crate::i18n::t("common.download-failed"));
  let props = BlobPropertyBag::new();
  props.set_type(mime);
  let parts = js_sys::Array::new();
  parts.push(&JsValue::from_str(content));
  let Ok(blob) = Blob::new_with_str_sequence_and_options(&parts, &props) else {
    fail();
    return;
  };
  let Ok(url) = Url::create_object_url_with_blob(&blob) else {
    fail();
    return;
  };
  let Ok(el) = document().create_element("a") else {
    let _ = Url::revoke_object_url(&url);
    fail();
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
  // 延后回收 URL：`a.click()` 触发的下载在浏览器里是**异步**的，紧跟其后同步
  // `revokeObjectURL` 在部分浏览器 / 大文件下会把下载掐断（URL 已失效）。
  leptos::prelude::set_timeout(
    move || {
      let _ = Url::revoke_object_url(&url);
    },
    std::time::Duration::from_millis(REVOKE_URL_DELAY_MS),
  );
}

/// 下载触发后多久回收 objectURL（毫秒）：够浏览器把下载挂上去，又不至于长期占着内存。
const REVOKE_URL_DELAY_MS: u64 = 60_000;

/// 方案库 → 矩量法求解器的交接键。
///
/// 方案库里的「在求解器中打开」把**模板 id** 写到这个键，`/nec` 挂载时取走并清除，
/// 再按 id 从 `nec_templates` 取正文 —— 存 id 而不是正文：正文动辄十几行，
/// 塞进 localStorage 容易和别的东西混淆，名字与正文也能保持单一来源。
pub const NEC_PENDING_KEY: &str = "nec.pending-template";

/// 把待加载的模板 id 暂存给 `/nec`。
pub fn stash_nec_template(id: &str) {
  if let Some(local) = window().local_storage().ok().flatten() {
    let _ = local.set_item(NEC_PENDING_KEY, id);
  }
}

/// 取出并清除待加载的模板 id（取不到就返回 `None`）。
pub fn take_nec_template() -> Option<String> {
  let local = window().local_storage().ok().flatten()?;
  let id = local.get_item(NEC_PENDING_KEY).ok().flatten()?;
  let _ = local.remove_item(NEC_PENDING_KEY);
  Some(id)
}

/// 根级备份任务持有期间不允许再次导入；提前退出和页面卸载都由 Drop 解锁。
pub struct BackupTaskGuard(leptos::prelude::RwSignal<bool>);

impl Drop for BackupTaskGuard {
  fn drop(&mut self) {
    use leptos::prelude::Set;
    self.0.try_set(false);
  }
}

pub fn begin_backup_task(busy: leptos::prelude::RwSignal<bool>) -> Option<BackupTaskGuard> {
  use leptos::prelude::{GetUntracked, Set};
  if busy.get_untracked() {
    return None;
  }
  busy.set(true);
  Some(BackupTaskGuard(busy))
}

/// 导出模块勾选（见 [`export_backup`]）。
#[derive(Debug, Clone, Copy, Default)]
pub struct BackupModules {
  /// 通联数据（日志 / 台站 / QSL 影像索引）。
  pub log: bool,
  /// QSL 影像本体（IndexedDB 的 `qsl-image:<id>`，数据量大，单独勾选）。
  pub qsl_images: bool,
  /// 其余本地数据。
  pub other: bool,
}

fn insert_backup_value(
  map: &mut std::collections::BTreeMap<String, String>,
  key: String,
  value: String,
) -> Result<(), String> {
  let bytes = ham_web_core::backup_merge::backup_payload_bytes(map);
  let old = map
    .get(&key)
    .map_or(0, |old| key.len().saturating_add(old.len()));
  let next = bytes
    .saturating_sub(old)
    .saturating_add(key.len())
    .saturating_add(value.len());
  if !ham_web_core::backup_merge::backup_size_is_supported(next) {
    return Err(t("tools.backup-too-large"));
  }
  map.insert(key, value);
  Ok(())
}

/// 按模块导出备份：localStorage 里的 key 按 [`ham_web_core::classify_key`] 归组，
/// QSL 影像本体从 IndexedDB 异步读取后并入同一份 JSON（结构不变：`{key: value}`，
/// 导入侧对 `qsl-image:` 前缀走 IndexedDB 直写，避开 localStorage 配额）。
pub async fn export_backup(modules: BackupModules) -> Result<usize, String> {
  let Some(local) = window().local_storage().ok().flatten() else {
    return Err(t("shell.localstorage-unavailable"));
  };
  let len = local.length().unwrap_or(0);
  let mut map = std::collections::BTreeMap::new();
  for i in 0..len {
    if let Ok(Some(key)) = local.key(i)
      && let Ok(Some(value)) = local.get_item(&key)
    {
      let keep = match ham_web_core::classify_key(&key) {
        ham_web_core::BackupModule::Log => modules.log,
        ham_web_core::BackupModule::QslImage => modules.qsl_images, // 快照侧基本不存在
        ham_web_core::BackupModule::Other => modules.other,
      };
      if keep {
        insert_backup_value(&mut map, key, value)?;
      }
    }
  }
  if modules.log
    && let Some(value) = crate::kv::load_large_strict("logbook")
      .await
      .map_err(|_| t("tools.backup-write-failed"))?
  {
    insert_backup_value(&mut map, "logbook".to_owned(), value)?;
  }
  // QSL 影像本体：按索引逐个从 IndexedDB 读回。
  //
  // 索引必须直接从 localStorage 读，而不是从上面按模块过滤过的 `map` 里取 ——
  // 它属于「通联」模块，只勾选「QSL 影像」时 `map` 里没有它，影像会被静默漏掉。
  // 索引本身也一并写进备份：没有它，恢复后的影像没人引用、界面上看不到。
  if modules.qsl_images {
    if !modules.log
      && let Some(context) = crate::kv::load_large_strict("logbook")
        .await
        .map_err(|_| t("tools.backup-write-failed"))?
    {
      insert_backup_value(&mut map, qsl_image::BACKUP_LOG_KEY.to_owned(), context)?;
    }
    let index = local.get_item(qsl_image::INDEX_KEY).ok().flatten();
    let ids = index
      .as_deref()
      .and_then(|raw| serde_json::from_str::<Vec<u64>>(raw).ok())
      .unwrap_or_default();
    if let Some(index) = index {
      insert_backup_value(&mut map, qsl_image::INDEX_KEY.to_owned(), index)?;
    }
    for id in ids {
      let key = qsl_image::key_for(id);
      if let Some(value) = crate::kv::load_store_only_strict(&key)
        .await
        .map_err(|_| t("tools.backup-write-failed"))?
      {
        insert_backup_value(&mut map, key, value)?;
      }
    }
  }
  let count = map.len();
  let Ok(json) = serde_json::to_string(&map) else {
    return Err(t("common.file-read-failed"));
  };
  if !ham_web_core::backup_merge::backup_size_is_supported(json.len()) {
    return Err(t("tools.backup-too-large"));
  }
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
  Ok(count)
}

/// 导入一条 QSL 影像：备份是外部输入，key 与值都按本站写出的格式校验，不合格的
/// 条目跳过（不计入返回的条数）。写入走 [`crate::kv::save_store_only`]。
async fn import_qsl_image(key: &str, value: &str) -> Result<bool, String> {
  if qsl_image::entry_id_from_key(key).is_none() || !qsl_image::is_valid_stored_value(value) {
    return Ok(false);
  }
  if !crate::kv::save_store_only(key, value).await {
    return Err(t("tools.backup-write-failed"));
  }
  Ok(true)
}

/// 从备份 JSON 恢复本地数据，返回恢复的条目数。
///
/// `qsl-image:` 前缀的键是 IndexedDB 直写数据（localStorage 放不下影像本体），
/// 经 [`import_qsl_image`] 校验后写入；其余仍走 localStorage + 权威层回灌。
pub async fn import_backup(json: &str) -> Result<usize, String> {
  let result = import_backup_inner(json).await;
  // 无论成败都要丢弃收藏 / 分组 / 笔记的内存缓存：直接写 localStorage 不会触发本标签页的
  // `storage` 事件，缓存不失效的话，之后的一次收藏就会用旧缓存整表覆盖刚恢复的数据。
  crate::store::invalidate_all();
  result
}

async fn import_backup_inner(json: &str) -> Result<usize, String> {
  let map: std::collections::BTreeMap<String, String> =
    serde_json::from_str(json).map_err(|e| e.to_string())?;
  let Some(local) = window().local_storage().ok().flatten() else {
    return Err(t("shell.localstorage-unavailable"));
  };
  let mut count = 0;
  let has_index = map.contains_key(qsl_image::INDEX_KEY) || map.contains_key("logbook");
  let mut index = std::collections::BTreeSet::new();
  // 旧影像的删除推迟到**所有写入成功之后**：此前预扫描里就直接删，若随后恢复日志或写别的 key
  // 失败，会落得「日志还是旧的、对应的影像已经没了」，且没法回滚。
  let mut stale_images: Vec<String> = Vec::new();
  let mut written_images = std::collections::BTreeSet::new();
  if let Some(incoming) = map.get("logbook") {
    let _: ham_web_core::logbook::Logbook =
      serde_json::from_str(incoming).map_err(|e| e.to_string())?;
    let current = crate::kv::load_large_strict("logbook")
      .await
      .map_err(|_| t("tools.backup-write-failed"))?;
    let retained = ham_web_core::backup_merge::retained_image_ids(current.as_deref(), incoming);
    let mut old_ids: std::collections::BTreeSet<u64> = local
      .get_item(qsl_image::INDEX_KEY)
      .map_err(|_| t("tools.backup-write-failed"))?
      .and_then(|v| serde_json::from_str::<Vec<u64>>(&v).ok())
      .unwrap_or_default()
      .into_iter()
      .collect();
    for raw in [current.as_deref(), Some(incoming.as_str())]
      .into_iter()
      .flatten()
    {
      if let Ok(book) = serde_json::from_str::<ham_web_core::logbook::Logbook>(raw) {
        old_ids.extend(book.entries.iter().map(|e| e.id));
      }
    }
    for id in old_ids {
      let key = qsl_image::key_for(id);
      if retained.contains(&id) {
        if crate::kv::load_store_only_strict(&key)
          .await
          .map_err(|_| t("tools.backup-write-failed"))?
          .is_some()
        {
          index.insert(id);
        }
      } else {
        stale_images.push(key);
      }
    }
  }
  for (k, v) in map {
    if k == qsl_image::BACKUP_LOG_KEY || k.ends_with(crate::kv::VERSION_SUFFIX) {
      continue;
    }
    if k == qsl_image::INDEX_KEY {
      continue;
    }
    if k.starts_with(qsl_image::KEY_PREFIX) {
      if import_qsl_image(&k, &v).await?
        && let Some(id) = qsl_image::entry_id_from_key(&k)
      {
        index.insert(id);
        written_images.insert(k.clone());
        count += 1;
      }
      continue;
    }
    if k == "logbook" {
      if !crate::kv::restore_large(&k, &v).await {
        return Err(t("tools.backup-write-failed"));
      }
    } else {
      local
        .set_item(&k, &v)
        .map_err(|_| t("tools.backup-write-failed"))?;
    }
    count += 1;
  }
  // 全部写入成功后才清理旧影像。备份里同一个 id 的影像已经覆盖写入，不能再删；删除失败只留下
  // 一份不在索引里的孤儿（不可见、只占空间），不值得让已经成功的恢复整体报错。
  for key in stale_images {
    if !written_images.contains(&key) {
      let _ = crate::kv::remove_store_only(&key).await;
    }
  }
  if has_index || !index.is_empty() {
    let value = serde_json::to_string(&index).map_err(|e| e.to_string())?;
    local
      .set_item(qsl_image::INDEX_KEY, &value)
      .map_err(|_| t("tools.backup-write-failed"))?;
    count += 1;
  }
  Ok(count)
}

/// 从备份 JSON **合并**导入：已知类型（日志、收藏、错题本、统计等）做并集 / 累加合并，
/// 其余 key 保留本机不覆盖。返回合并写入的条目数。
pub async fn import_backup_merge(json: &str) -> Result<usize, String> {
  let result = import_backup_merge_inner(json).await;
  // 理由同 [`import_backup`]：合并也是直接写 localStorage。
  crate::store::invalidate_all();
  result
}

async fn import_backup_merge_inner(json: &str) -> Result<usize, String> {
  let map: std::collections::BTreeMap<String, String> =
    serde_json::from_str(json).map_err(|e| e.to_string())?;
  let Some(local) = window().local_storage().ok().flatten() else {
    return Err(t("shell.localstorage-unavailable"));
  };
  let mut count = 0;
  let mut ids = std::collections::BTreeMap::new();
  let current_log = crate::kv::load_large_strict("logbook")
    .await
    .map_err(|_| t("tools.backup-write-failed"))?;
  if let Some(backup) = map.get("logbook") {
    let (merged, mapping) =
      ham_web_core::backup_merge::merge_logbook_with_ids(current_log.as_deref(), backup)
        .ok_or_else(|| t("tools.backup-write-failed"))?;
    if !crate::kv::restore_large("logbook", &merged).await {
      return Err(t("tools.backup-write-failed"));
    }
    ids = mapping;
    count += 1;
  } else if let Some(context) = map.get(qsl_image::BACKUP_LOG_KEY) {
    ids = ham_web_core::backup_merge::image_ids_for_existing_log(current_log.as_deref(), context);
  } else if current_log.is_none() {
    // 旧版仅影像备份没有日志上下文；仅在本机没有日志时保留原始 ID。
    ids.extend(
      map
        .keys()
        .filter_map(|k| qsl_image::entry_id_from_key(k).map(|id| (id, id))),
    );
  }
  let mut index: std::collections::BTreeSet<u64> = local
    .get_item(qsl_image::INDEX_KEY)
    .ok()
    .flatten()
    .and_then(|v| serde_json::from_str::<Vec<u64>>(&v).ok())
    .unwrap_or_default()
    .into_iter()
    .collect();
  for (k, v) in map {
    if matches!(
      k.as_str(),
      "logbook" | qsl_image::INDEX_KEY | qsl_image::BACKUP_LOG_KEY
    ) || k.ends_with(crate::kv::VERSION_SUFFIX)
    {
      continue;
    }
    if k.starts_with(qsl_image::KEY_PREFIX) {
      let Some(target) = qsl_image::entry_id_from_key(&k)
        .and_then(|id| ids.get(&id))
        .copied()
      else {
        continue;
      };
      let key = qsl_image::key_for(target);
      if crate::kv::load_store_only_strict(&key)
        .await
        .map_err(|_| t("tools.backup-write-failed"))?
        .is_some()
      {
        index.insert(target);
      } else if import_qsl_image(&key, &v).await? {
        index.insert(target);
        count += 1;
      }
      continue;
    }
    let current = local.get_item(&k).ok().flatten();
    if let Some(merged) = ham_web_core::backup_merge::merge_value(&k, current.as_deref(), &v) {
      local
        .set_item(&k, &merged)
        .map_err(|_| t("tools.backup-write-failed"))?;
      count += 1;
    }
  }
  if !index.is_empty() {
    let value = serde_json::to_string(&index).map_err(|e| e.to_string())?;
    local
      .set_item(qsl_image::INDEX_KEY, &value)
      .map_err(|_| t("tools.backup-write-failed"))?;
    count += 1;
  }
  Ok(count)
}

/// 一次读取的字节上限：超过就直接拒绝。
///
/// 备份 JSON、ADIF 日志、QSL 报告都在几百 KB 量级，64 MB 只是兜底 —— 防止用户误选
/// 一个几百 MB 的文件把 wasm 内存整块吃光（读进 `String` 还会再翻一倍）。
const MAX_READ_BYTES: f64 = ham_web_core::backup_merge::MAX_BACKUP_BYTES as f64;

/// 读取文件文本内容（超过 [`MAX_READ_BYTES`] 视为失败）。
pub async fn read_file_text(file: &web_sys::File) -> Option<String> {
  if file.size() > MAX_READ_BYTES {
    return None;
  }
  let blob: web_sys::Blob = file.clone().into();
  wasm_bindgen_futures::JsFuture::from(blob.text())
    .await
    .ok()?
    .as_string()
}

/// 当前浏览器通知权限；浏览器没有通知 API 时返回 `None`。
///
/// 必须先探测：iOS Safari 非主屏 PWA 环境下没有 `window.Notification`，而
/// `Notification::permission()` 的绑定不带 `catch`，直接调用会抛 `ReferenceError` 让 wasm 中止。
pub fn notification_permission() -> Option<web_sys::NotificationPermission> {
  let supported = js_sys::Reflect::has(&window(), &wasm_bindgen::JsValue::from_str("Notification"))
    .unwrap_or(false);
  supported.then(web_sys::Notification::permission)
}

/// 发送浏览器通知（仅在已授予通知权限时生效）。
pub fn notify(title: &str) {
  use web_sys::{Notification, NotificationPermission};
  if notification_permission() != Some(NotificationPermission::Granted) {
    return;
  }
  let _ = Notification::new(title);
}

/// 请求浏览器通知权限。
pub fn request_notify_permission() {
  use web_sys::{Notification, NotificationPermission};
  if notification_permission() == Some(NotificationPermission::Default) {
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

thread_local! {
  /// 防抖槽位：`名称 → (定时器 id, 待触发闭包)`。
  ///
  /// 闭包必须由这里持有 —— `setTimeout` 只拿一个 JS 函数，Rust 侧不留引用的话
  /// 闭包会被立刻释放，回调触发时访问的是已失效的指针。
  static DEBOUNCE_SLOTS: RefCell<HashMap<&'static str, (i32, JsValue)>> =
    RefCell::new(HashMap::new());
}

/// 防抖：`delay_ms` 内的重复调用只保留最后一次（按 `key` 分槽，互不干扰）。
///
/// 用于「输入框每敲一个字符就触发重算」的场景 —— 几何重建、矩量法求解、canvas 重绘
/// 在主线程上是重活，逐键跑会肉眼可见地卡。
///
/// **组件卸载前必须 [`cancel_debounce`]**：定时器回调会在页面销毁后触发，
/// 触碰已释放的响应式信号会直接 panic。
pub fn debounce(key: &'static str, delay_ms: i32, f: impl FnOnce() + 'static) {
  let cb = Closure::once_into_js(f);
  let handle = window()
    .set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), delay_ms);
  DEBOUNCE_SLOTS.with(|slots| {
    let mut slots = slots.borrow_mut();
    if let Some((id, _)) = slots.remove(key) {
      window().clear_timeout_with_handle(id);
    }
    if let Ok(id) = handle {
      slots.insert(key, (id, cb));
    }
  });
}

/// 撤销 `key` 上待触发的防抖任务（组件 `on_cleanup` 里调用）。
pub fn cancel_debounce(key: &'static str) {
  DEBOUNCE_SLOTS.with(|slots| {
    if let Some((id, _)) = slots.borrow_mut().remove(key) {
      window().clear_timeout_with_handle(id);
    }
  });
}

/// 把 `JsValue` 错误转为可读字符串。
pub fn js_error_message(err: &JsValue) -> String {
  if let Some(e) = err.dyn_ref_error() {
    return e;
  }
  if let Some(s) = err.as_string() {
    return s;
  }
  // `DOMException`（IndexedDB / 文件读取失败）不是 `js_sys::Error`：按「有 message 属性」兜底，
  // 否则用户会看到 `JsValue(DOMException ...)` 的 Debug 串。
  js_sys::Reflect::get(err, &"message".into())
    .ok()
    .and_then(|m| m.as_string())
    .filter(|m| !m.is_empty())
    .unwrap_or_else(|| format!("{err:?}"))
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
