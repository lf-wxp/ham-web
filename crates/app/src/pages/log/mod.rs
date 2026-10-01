//! 通联日志：在线记录、本地持久化、ADIF / CSV 导入导出。
//!
//! 记录字段对齐 ADIF 规范：本台呼号（`STATION_CALLSIGN`）、操作员、本台网格、
//! `RST_SENT` / `RST_RCVD` 分离、对方网格 / 姓名 / QTH 等。

mod bar_list;
mod grid_cell;
mod grid_map;
mod log_page;

pub use grid_map::{CONTINENTS, COUNTRY_LABELS, GridMap, polygon_points, project, simplify};
pub use log_page::LogPage;

use ham_web_core::adif::AdifRecord;
use ham_web_core::frequencies::band_of;
use serde::{Deserialize, Serialize};

/// 本台信息（站点级设置，应用到 ADIF 导出的台站字段）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct StationInfo {
  /// 本台呼号 `STATION_CALLSIGN`。
  callsign: String,
  /// 操作员 `OPERATOR`（空则默认同呼号）。
  operator: String,
  /// 本台网格 `MY_GRIDSQUARE`。
  gridsquare: String,
  /// 设备 `MY_RIG`。
  rig: String,
  /// 天线 `MY_ANTENNA`。
  antenna: String,
}

/// 一条通联记录。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
  id: u64,
  /// 日期（YYYY-MM-DD）。
  date: String,
  /// 时间（HH:MM）。
  time: String,
  /// 频率（MHz，文本）。
  freq: String,
  /// 模式。
  mode: String,
  /// 对方呼号 `CALL`。
  callsign: String,
  /// 遗留：旧版单值 RST，加载后迁移到 `rst_sent` / `rst_rcvd`。
  #[serde(default)]
  rst: String,
  /// 发给对方的信号报告 `RST_SENT`。
  #[serde(default)]
  rst_sent: String,
  /// 收到对方的信号报告 `RST_RCVD`。
  #[serde(default)]
  rst_rcvd: String,
  /// 对方网格 `GRIDSQUARE`。
  #[serde(default)]
  gridsquare: String,
  /// 对方操作员姓名 `NAME`。
  #[serde(default)]
  name: String,
  /// 对方 QTH。
  #[serde(default)]
  qth: String,
  /// 备注 `COMMENT`。
  remark: String,
  /// QSL 卡片是否已寄出 `QSL_SENT`。
  #[serde(default)]
  qsl_sent: bool,
  /// QSL 是否已确认收到 `QSL_RCVD`。
  #[serde(default)]
  qsl_rcvd: bool,
}

/// 全部日志。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Logbook {
  entries: Vec<LogEntry>,
}

const LOG_KEY: &str = "logbook";
const STATION_KEY: &str = "station-info";
const MODES: &[&str] = &[
  "CW", "SSB", "AM", "FM", "RTTY", "PSK31", "FT8", "FT4", "JT65", "SSTV", "DATA", "OTHER",
];

/// 基于已有记录计算下一个不冲突的 ID（刷新页面后仍能避免与已持久化记录冲突）。
fn next_id(entries: &[LogEntry]) -> u64 {
  entries.iter().map(|e| e.id).max().unwrap_or(0) + 1
}

fn load_logbook() -> Logbook {
  let mut lb: Logbook = crate::util::storage::get_json(LOG_KEY).unwrap_or_default();
  // 迁移旧数据：单值 rst → rst_sent / rst_rcvd。
  for e in &mut lb.entries {
    if e.rst_sent.is_empty() && e.rst_rcvd.is_empty() && !e.rst.is_empty() {
      e.rst_sent = e.rst.clone();
      e.rst_rcvd = e.rst.clone();
    }
  }
  lb
}

fn save_logbook(logbook: &Logbook) {
  crate::util::storage::set_json(LOG_KEY, logbook);
}

/// 快捷添加一条日志（供 DX 热点等外部入口使用）。
pub fn quick_add(callsign: &str, freq_mhz: &str, mode: &str) {
  let mut lb = load_logbook();
  lb.entries.push(LogEntry {
    id: next_id(&lb.entries),
    date: utc_today(),
    time: utc_now_time(),
    freq: freq_mhz.to_owned(),
    mode: mode.to_owned(),
    callsign: callsign.trim().to_uppercase(),
    rst: String::new(),
    rst_sent: String::new(),
    rst_rcvd: String::new(),
    gridsquare: String::new(),
    name: String::new(),
    qth: String::new(),
    remark: String::new(),
    qsl_sent: false,
    qsl_rcvd: false,
  });
  save_logbook(&lb);
}

fn load_station() -> StationInfo {
  crate::util::storage::get_json(STATION_KEY).unwrap_or_default()
}

fn save_station(station: &StationInfo) {
  crate::util::storage::set_json(STATION_KEY, station);
}

/// 读取网格地图所需的输入：全部通联记录 + 本台网格（供网格地图页等外部入口复用）。
pub fn grid_map_input() -> (Vec<LogEntry>, String) {
  let lb = load_logbook();
  let st = load_station();
  (lb.entries, st.gridsquare)
}

/// 当前 UTC 日期（YYYY-MM-DD）。
fn utc_today() -> String {
  let d = js_sys::Date::new_0();
  let y = d.get_utc_full_year() as i32;
  let m = d.get_utc_month() as i32 + 1;
  let day = d.get_utc_date() as i32;
  format!("{y:04}-{m:02}-{day:02}")
}

/// 当前 UTC 时间（HH:MM）。
fn utc_now_time() -> String {
  let d = js_sys::Date::new_0();
  let h = d.get_utc_hours() as i32;
  let m = d.get_utc_minutes() as i32;
  format!("{h:02}:{m:02}")
}

/// 一个 ADIF 字段。
fn adif_field(name: &str, value: &str) -> String {
  format!("<{name}:{}>{value}", value.len())
}

/// 把日志序列化为 ADIF 文本（含本台信息与规范字段）。
fn export_adif(entries: &[LogEntry], station: &StationInfo) -> String {
  let mut s = String::from("Amateur radio logbook\n");
  s.push_str(&adif_field("ADIF_VER", "3.1.4"));
  s.push_str(&adif_field("PROGRAMID", "HamExamWeb"));
  if !station.callsign.trim().is_empty() {
    s.push_str(&adif_field("STATION_CALLSIGN", station.callsign.trim()));
  }
  if !station.operator.trim().is_empty() {
    s.push_str(&adif_field("OPERATOR", station.operator.trim()));
  }
  if !station.gridsquare.trim().is_empty() {
    s.push_str(&adif_field("MY_GRIDSQUARE", station.gridsquare.trim()));
  }
  if !station.rig.trim().is_empty() {
    s.push_str(&adif_field("MY_RIG", station.rig.trim()));
  }
  if !station.antenna.trim().is_empty() {
    s.push_str(&adif_field("MY_ANTENNA", station.antenna.trim()));
  }
  s.push_str("<EOH>\n");
  for e in entries {
    s.push_str(&adif_field("QSO_DATE", &e.date.replace('-', "")));
    s.push_str(&adif_field("TIME_ON", &e.time.replace(':', "")));
    s.push_str(&adif_field("CALL", &e.callsign));
    s.push_str(&adif_field("FREQ", e.freq.trim()));
    if let Ok(f) = e.freq.trim().parse::<f64>() {
      let band = band_of(f);
      if band != "其他" {
        s.push_str(&adif_field("BAND", &band.to_uppercase()));
      }
    }
    s.push_str(&adif_field("MODE", &e.mode));
    if !e.rst_sent.is_empty() {
      s.push_str(&adif_field("RST_SENT", &e.rst_sent));
    }
    if !e.rst_rcvd.is_empty() {
      s.push_str(&adif_field("RST_RCVD", &e.rst_rcvd));
    }
    if !e.gridsquare.is_empty() {
      s.push_str(&adif_field("GRIDSQUARE", &e.gridsquare));
    }
    if !e.name.is_empty() {
      s.push_str(&adif_field("NAME", &e.name));
    }
    if !e.qth.is_empty() {
      s.push_str(&adif_field("QTH", &e.qth));
    }
    if !e.remark.is_empty() {
      s.push_str(&adif_field("COMMENT", &e.remark));
    }
    s.push_str(&adif_field("QSL_SENT", if e.qsl_sent { "Y" } else { "N" }));
    s.push_str(&adif_field("QSL_RCVD", if e.qsl_rcvd { "Y" } else { "N" }));
    s.push_str("<EOR>\n");
  }
  s
}

/// CSV 字段转义。
fn csv_field(s: &str) -> String {
  if s.contains(',') || s.contains('"') || s.contains('\n') {
    format!("\"{}\"", s.replace('"', "\"\""))
  } else {
    s.to_owned()
  }
}

/// 把日志序列化为 CSV 文本。
fn export_csv(entries: &[LogEntry], station: &StationInfo) -> String {
  let mut s = String::from(
    "本台呼号,日期,时间,频率,模式,对方呼号,RST发送,RST接收,网格,姓名,QTH,备注,QSL寄出,QSL收到\n",
  );
  for e in entries {
    s.push_str(&format!(
      "{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
      csv_field(station.callsign.trim()),
      csv_field(&e.date),
      csv_field(&e.time),
      csv_field(&e.freq),
      csv_field(&e.mode),
      csv_field(&e.callsign),
      csv_field(&e.rst_sent),
      csv_field(&e.rst_rcvd),
      csv_field(&e.gridsquare),
      csv_field(&e.name),
      csv_field(&e.qth),
      csv_field(&e.remark),
      if e.qsl_sent { "Y" } else { "" },
      if e.qsl_rcvd { "Y" } else { "" },
    ));
  }
  s
}

/// 把 ADIF 记录转为本地日志条目。
fn from_adif(r: AdifRecord) -> LogEntry {
  let date = if r.qso_date.len() == 8
    && let (Some(y), Some(m), Some(d)) = (
      r.qso_date.get(0..4),
      r.qso_date.get(4..6),
      r.qso_date.get(6..8),
    ) {
    format!("{y}-{m}-{d}")
  } else {
    r.qso_date
  };
  let time = if r.time_on.len() == 4
    && let (Some(h), Some(m)) = (r.time_on.get(0..2), r.time_on.get(2..4))
  {
    format!("{h}:{m}")
  } else {
    r.time_on
  };
  LogEntry {
    // 占位 ID，由导入方基于现有记录重新分配，避免重复。
    id: 0,
    date,
    time,
    freq: r.freq,
    mode: r.mode.to_uppercase(),
    callsign: r.call,
    rst: String::new(),
    rst_sent: r.rst_sent,
    rst_rcvd: r.rst_rcvd,
    gridsquare: r.gridsquare,
    name: r.name,
    qth: r.qth,
    remark: r.comment,
    qsl_sent: false,
    qsl_rcvd: false,
  }
}
