//! 通联日志：在线记录、本地持久化、ADIF / CSV 导入导出。数据模型与导入导出逻辑见
//! [`ham_web_core::logbook`]，这里只负责响应式 store 与 `localStorage` 持久化。

mod awards_panel;
mod bar_list;
mod card_image_mark;
mod contest_log;
mod entry_form;
mod entry_list;
mod form_state;
mod grid_cell;
mod grid_fill;
mod grid_filter;
mod grid_geo;
mod grid_map;
mod log_health_panel;
mod log_helpers;
mod log_page;
mod log_stats_panel;
mod qsl_badge;
mod qsl_image;
mod qsl_sync_dialog;
mod station_panel;

pub use contest_log::ContestLogPage;
pub use grid_map::GridMap;
pub use log_page::LogPage;

use ham_web_core::adif::AdifRecord;
pub use ham_web_core::logbook::{LogEntry, Logbook, StationInfo};
pub use ham_web_core::station::{Removal, StationBook, StationProfile};
use leptos::prelude::*;

const LOG_KEY: &str = "logbook";
/// 老的单值本台信息：**当前台站的镜像**，仍在写（RBN / PSK Reporter 页面与备份合并读它）。
const STATION_KEY: &str = "station-info";
/// 台站档案册（唯一真相来源：多台站 + 每条通联的归属）。
const STATION_BOOK_KEY: &str = "station-book";

thread_local! {
  /// 本页是否已写入过日志：用于避免 IndexedDB 异步加载完成后覆盖用户刚写入的数据。
  static LOG_DIRTY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// 解析一份日志 JSON（顺带做旧数据迁移）。
fn parse_logbook(json: &str) -> Logbook {
  let mut lb: Logbook = serde_json::from_str(json).unwrap_or_default();
  lb.migrate();
  lb
}

fn save_logbook(logbook: &Logbook) {
  LOG_DIRTY.with(|c| c.set(true));
  // 统一门面：localStorage 静默快照 + IndexedDB 权威（异步），见 `crate::kv::save_large`。
  if let Ok(json) = serde_json::to_string(logbook) {
    crate::kv::save_large(LOG_KEY, &json);
  }
}

/// 读台站档案册；本机还没有档案册时，用老的单值本台信息迁出一条档案。
fn load_book() -> StationBook {
  let stored: Option<StationBook> = crate::util::storage::get_json(STATION_BOOK_KEY);
  let mut book = match stored {
    Some(book) => book,
    None => {
      // 首次升级：把 `station-info` 搬成第一条档案，并立刻落盘 —— 否则用户改了别的东西
      // 再刷新，这条迁移会被重做一遍（虽然结果一样，但白写两次）。
      let legacy: StationInfo = crate::util::storage::get_json(STATION_KEY).unwrap_or_default();
      let book = StationBook::from_info(&legacy);
      save_book(&book);
      book
    }
  };
  book.migrate();
  book
}

/// 写台站档案册，并同步 `station-info` 镜像（老读者照旧能拿到「当前台站」）。
fn save_book(book: &StationBook) {
  crate::util::storage::set_json(STATION_BOOK_KEY, book);
  crate::util::storage::set_json(STATION_KEY, &book.active().info());
}

/// 通联日志共享状态：跨页面共享的响应式 store，统一承载日志与本台信息，
/// 并保证每次变更后持久化到 `localStorage`。
#[derive(Clone, Copy)]
pub struct LogStore {
  pub logbook: RwSignal<Logbook>,
  /// 根级备份任务锁：切换路由不会释放，直到落盘及 Store 同步结束。
  pub backup_busy: RwSignal<bool>,
  /// 台站档案册：多台站 + 每条通联的归属（`LogEntry::station_id`）。
  pub station: RwSignal<StationBook>,
  /// 存了卡片影像的通联 id（localStorage 索引的响应式镜像，供列表渲染相机标记）。
  pub qsl_images: RwSignal<std::collections::HashSet<u64>>,
}

impl LogStore {
  /// 备份 / 恢复进行中：本次写入会被稍后的 `refresh_from_storage` 覆回，如实告诉用户没保存。
  ///
  /// 此前三个入口在忙时都静默返回：内存里能看到刚做的编辑，随后恢复结束又被旧数据整个盖掉，
  /// 用户既不知道没保存、也不知道为什么消失。
  fn warn_busy() {
    crate::util::alert(&crate::i18n::t("log.backup-busy-not-saved"));
  }

  /// 把当前日志写入 `localStorage`（任何日志变更后调用）。
  ///
  /// 备份 / 恢复进行中不写，并提示用户（见 [`Self::warn_busy`]）。
  pub fn persist(self) {
    if self.backup_busy.get_untracked() {
      Self::warn_busy();
      return;
    }
    save_logbook(&self.logbook.get_untracked());
  }

  /// 把台站档案册写入 `localStorage`（保存本台信息时调用）。
  pub fn persist_station(self) {
    save_book(&self.station.get_untracked());
  }

  /// 当前台站（ADIF 镜像、卡片设计器、距离估算、RBN 预填都用它）。
  ///
  /// 需要「某条通联的台站」时用 [`StationBook::of_entry`]，
  /// 不要拿这个当代替 —— 多台站日志里每条记录各归各的。
  pub fn active_station(self) -> StationInfo {
    self.station.with(|book| book.active().info())
  }

  /// 备份写入成功或部分失败后同步内存状态，防止下一次编辑把恢复结果覆回旧整库。
  pub async fn refresh_from_storage(self) {
    let loaded = crate::kv::load_large_strict(LOG_KEY).await;
    // `try_with_untracked`：只判断信号是否还活着，不克隆整本日志。
    if self.logbook.try_with_untracked(|_| ()).is_none() {
      return;
    }
    if let Ok(Some(json)) = loaded {
      self.logbook.set(parse_logbook(&json));
      LOG_DIRTY.with(|dirty| dirty.set(true));
    }
    self.station.set(load_book());
    self.qsl_images.set(qsl_image::index());
  }

  /// 快捷添加一条日志（供 DX 热点等外部入口使用），并持久化。
  ///
  /// 返回是否真的添加了：备份 / 恢复进行中返回 `false`（已提示用户），调用方不能再报「已记录」。
  pub fn quick_add(self, callsign: &str, freq_mhz: &str, mode: &str) -> bool {
    if self.backup_busy.get_untracked() {
      Self::warn_busy();
      return false;
    }
    let station_id = self.station.with_untracked(|book| book.active().id);
    self.logbook.update(|lb| {
      let mut entry = LogEntry {
        id: lb.next_id(),
        date: utc_today(),
        time: utc_now_time(),
        freq: freq_mhz.to_owned(),
        mode: mode.to_owned(),
        callsign: callsign.trim().to_uppercase(),
        station_id,
        ..Default::default()
      };
      entry.fill_location();
      lb.entries.push(entry);
    });
    self.persist();
    true
  }

  /// 导入 ADIF 记录，跳过重复通联，返回 `(导入数, 重复数)`。
  ///
  /// 报告里的 `STATION_CALLSIGN` / `MY_GRIDSQUARE` 会在这一步认领台站档案，
  /// 因此导入可能同时改动档案册，一并持久化。
  ///
  /// 备份 / 恢复进行中返回 `None`（已提示用户）：此前返回 `(0, 全部)`，页面会误报
  /// 「跳过 N 条重复」，而实际上一条都没处理。
  pub fn import(self, records: Vec<AdifRecord>) -> Option<(usize, usize)> {
    if self.backup_busy.get_untracked() {
      Self::warn_busy();
      return None;
    }
    // 档案册与日志要一起改（导入会按报告里的 `STATION_CALLSIGN` 认领台站），
    // 先把两份都取出来改完再写回 —— 避免在一个信号的 `update` 里再 `update` 另一个信号。
    let before = self.station.with_untracked(StationBook::clone);
    let mut book = before.clone();
    let mut lb = self.logbook.get_untracked();
    let result = lb.import(records, &mut book);
    // 只有真加了记录才写回：`Logbook::import` 只会往日志里追加，一条都没加时
    // `set` 只是白让整页重算一遍（导入一份全是重复项的 ADIF 时最容易遇到）。
    if result.0 > 0 {
      self.logbook.set(lb);
      self.persist();
    }
    let station_changed = book != before;
    if station_changed {
      self.station.set(book);
      self.persist_station();
    }
    Some(result)
  }
}

/// 初始化日志 store 并提供上下文（在应用根组件调用一次）。
pub fn provide_log_store() {
  let snapshot = crate::kv::load_snapshot(LOG_KEY);
  let store = LogStore {
    logbook: RwSignal::new(
      snapshot
        .as_deref()
        .map_or_else(Logbook::default, parse_logbook),
    ),
    backup_busy: RwSignal::new(false),
    station: RwSignal::new(load_book()),
    qsl_images: RwSignal::new(qsl_image::index()),
  };
  provide_context(store);

  // 跨标签页同步：其他标签页修改日志 / 本台信息时，本页同步刷新。`storage` 事件
  // 只在「其他」标签页写入时触发，本页自身写入不触发，因此不会产生回环。
  let _ = window_event_listener_untyped("storage", move |ev: web_sys::Event| {
    let key = js_sys::Reflect::get(ev.as_ref(), &"key".into())
      .ok()
      .and_then(|v| v.as_string());
    // `newValue` 为 null 表示该 key 被删除。日志快照被删（`kv::save_large` 在配额写满时会
    // 清掉快照）并不代表日志没了——权威层仍在 IndexedDB；这里若照样 `load_logbook()` 会得到
    // 空日志并清空本页内存，随后任何一次新增都会用只含一条记录的日志覆盖权威层。
    let removed = js_sys::Reflect::get(ev.as_ref(), &"newValue".into())
      .ok()
      .is_some_and(|v| v.is_null() || v.is_undefined());
    match key.as_deref() {
      Some(LOG_KEY) if removed => {}
      Some(LOG_KEY) => {
        // 本页已经编辑过就不整表覆盖（与下方首屏校正同一语义）：其他标签页写入的是它
        // 自己的旧快照，这里若照单全收会把本页内存回退成旧数据，随后一次 `persist`
        // 就把本页的编辑写丢。跨标签页并发写本就无法合流（最后写赢），至少保住本页。
        if LOG_DIRTY.with(|c| c.get()) {
          return;
        }
        // 另一标签页可能已把权威层写进 IndexedDB，本页进程内缓存还停在旧值：先失效
        // 缓存再走 `load_large` 的真实读取 + 快照/权威版本仲裁，而不是直读快照。
        crate::idb::cache_remove(LOG_KEY);
        wasm_bindgen_futures::spawn_local(async move {
          if let Some(json) = crate::kv::load_large(LOG_KEY).await {
            store.logbook.set(parse_logbook(&json));
          }
        });
      }
      Some(STATION_BOOK_KEY) => store.station.set(load_book()),
      Some(qsl_image::INDEX_KEY) => store.qsl_images.set(qsl_image::index()),
      _ => {}
    }
  });

  // 由门面比较「快照 / 权威层」的写入版本后给出更新的那一份：大日志时快照可能缺失，
  // 而快照刚写完就刷新页面时又要以快照为准（见 `crate::kv` 的模块文档）。
  // 与首屏读到的快照一致就什么都不做，省掉一次无谓的重渲染。
  wasm_bindgen_futures::spawn_local(async move {
    if let Some(json) = crate::kv::load_large(LOG_KEY).await
      && Some(&json) != snapshot.as_ref()
      && !LOG_DIRTY.with(|c| c.get())
    {
      store.logbook.set(parse_logbook(&json));
    }
  });
}

/// 获取日志 store（需在 `provide_log_store` 之后调用）。
pub fn use_log_store() -> LogStore {
  expect_context::<LogStore>()
}

/// 当前 UTC 日期（YYYY-MM-DD）。
pub fn utc_today() -> String {
  let d = js_sys::Date::new_0();
  let y = d.get_utc_full_year() as i32;
  let m = d.get_utc_month() as i32 + 1;
  let day = d.get_utc_date() as i32;
  format!("{y:04}-{m:02}-{day:02}")
}

/// 当前 UTC 时间（HH:MM）。
pub fn utc_now_time() -> String {
  let d = js_sys::Date::new_0();
  let h = d.get_utc_hours() as i32;
  let m = d.get_utc_minutes() as i32;
  format!("{h:02}:{m:02}")
}
