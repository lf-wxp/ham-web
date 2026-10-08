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
use leptos::prelude::*;

const LOG_KEY: &str = "logbook";
const STATION_KEY: &str = "station-info";

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

/// 同步读本地快照：首屏立即有内容可渲染；版本比较在异步的权威层读取里完成。
fn load_logbook() -> Logbook {
  crate::kv::load_snapshot(LOG_KEY).map_or_else(Logbook::default, |json| parse_logbook(&json))
}

fn save_logbook(logbook: &Logbook) {
  LOG_DIRTY.with(|c| c.set(true));
  // 统一门面：localStorage 静默快照 + IndexedDB 权威（异步），见 `crate::kv::save_large`。
  if let Ok(json) = serde_json::to_string(logbook) {
    crate::kv::save_large(LOG_KEY, &json);
  }
}

fn load_station() -> StationInfo {
  crate::util::storage::get_json(STATION_KEY).unwrap_or_default()
}

fn save_station(station: &StationInfo) {
  crate::util::storage::set_json(STATION_KEY, station);
}

/// 通联日志共享状态：跨页面共享的响应式 store，统一承载日志与本台信息，
/// 并保证每次变更后持久化到 `localStorage`。
#[derive(Clone, Copy)]
pub struct LogStore {
  pub logbook: RwSignal<Logbook>,
  pub station: RwSignal<StationInfo>,
  /// 存了卡片影像的通联 id（localStorage 索引的响应式镜像，供列表渲染相机标记）。
  pub qsl_images: RwSignal<std::collections::HashSet<u64>>,
}

impl LogStore {
  /// 把当前日志写入 `localStorage`（任何日志变更后调用）。
  pub fn persist(self) {
    save_logbook(&self.logbook.get_untracked());
  }

  /// 把当前本台信息写入 `localStorage`（保存本台信息时调用）。
  pub fn persist_station(self) {
    save_station(&self.station.get_untracked());
  }

  /// 快捷添加一条日志（供 DX 热点等外部入口使用），并持久化。
  pub fn quick_add(self, callsign: &str, freq_mhz: &str, mode: &str) {
    self.logbook.update(|lb| {
      let mut entry = LogEntry {
        id: lb.next_id(),
        date: utc_today(),
        time: utc_now_time(),
        freq: freq_mhz.to_owned(),
        mode: mode.to_owned(),
        callsign: callsign.trim().to_uppercase(),
        ..Default::default()
      };
      entry.fill_location();
      lb.entries.push(entry);
    });
    self.persist();
  }

  /// 导入 ADIF 记录，跳过重复通联，返回 `(导入数, 重复数)`。
  pub fn import(self, records: Vec<AdifRecord>) -> (usize, usize) {
    let mut result = (0, 0);
    self.logbook.update(|lb| result = lb.import(records));
    if result.0 > 0 {
      self.persist();
    }
    result
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
    station: RwSignal::new(load_station()),
    qsl_images: RwSignal::new(qsl_image::index()),
  };
  provide_context(store);

  // 跨标签页同步：其他标签页修改日志 / 本台信息时，本页同步刷新。`storage` 事件
  // 只在「其他」标签页写入时触发，本页自身写入不触发，因此不会产生回环。
  let _ = window_event_listener_untyped("storage", move |ev: web_sys::Event| {
    let key = js_sys::Reflect::get(ev.as_ref(), &"key".into())
      .ok()
      .and_then(|v| v.as_string());
    match key.as_deref() {
      Some(LOG_KEY) => store.logbook.set(load_logbook()),
      Some(STATION_KEY) => store.station.set(load_station()),
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
