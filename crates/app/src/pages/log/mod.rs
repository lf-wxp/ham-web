//! 通联日志：在线记录、本地持久化、ADIF / CSV 导入导出。数据模型与导入导出逻辑见
//! [`ham_web_core::logbook`]，这里只负责响应式 store 与 `localStorage` 持久化。

mod awards_panel;
mod bar_list;
mod contest_log;
mod grid_cell;
mod grid_fill;
mod grid_filter;
mod grid_geo;
mod grid_map;
mod log_helpers;
mod log_page;

pub use contest_log::ContestLogPage;
pub use grid_map::GridMap;
pub use log_page::LogPage;

use ham_web_core::adif::AdifRecord;
pub use ham_web_core::logbook::{LogEntry, Logbook, StationInfo};
use leptos::prelude::*;

const LOG_KEY: &str = "logbook";
const STATION_KEY: &str = "station-info";

fn load_logbook() -> Logbook {
  let mut lb: Logbook = crate::util::storage::get_json(LOG_KEY).unwrap_or_default();
  lb.migrate();
  lb
}

fn save_logbook(logbook: &Logbook) {
  crate::util::storage::set_json(LOG_KEY, logbook);
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
  let store = LogStore {
    logbook: RwSignal::new(load_logbook()),
    station: RwSignal::new(load_station()),
  };
  provide_context(store);
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
