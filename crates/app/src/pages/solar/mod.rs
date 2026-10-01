//! 传播预测科普：太阳活动指数、传播条件分级，并接入实时数据。
//!
//! 优先通过服务端 `/api/solar`（HamQSL，含 A/K 指数与各波段条件）；
//! 不可用时降级为直接请求 NOAA SWPC。

mod alerts_card;
mod conditions_table;
mod flux_trend;
mod metric_card;
mod solar_page;

pub use solar_page::SolarPage;

use serde::Deserialize;

/// 服务端 `/api/solar` 返回结构（HamQSL 数据）。
#[derive(Deserialize, Clone)]
struct SolarApi {
  updated: Option<String>,
  solar_flux: Option<i32>,
  a_index: Option<i32>,
  k_index: Option<i32>,
  sunspots: Option<i32>,
  xray: Option<String>,
  conditions: Vec<BandCond>,
}

/// 各波段传播条件。
#[derive(Deserialize, Clone)]
struct BandCond {
  band: String,
  time: String,
  condition: String,
}

/// NOAA Kp 指数记录（降级数据源）。
#[derive(Deserialize)]
struct KpEntry {
  time_tag: String,
  kp_index: Option<i32>,
  estimated_kp: Option<f64>,
}

/// NOAA 太阳活动记录（降级数据源与历史趋势）。
#[derive(Deserialize)]
struct SolarEntry {
  #[serde(rename = "time-tag")]
  time_tag: String,
  ssn: Option<f64>,
  #[serde(rename = "f10.7")]
  f107: Option<f64>,
}

/// NOAA SWPC 行星 Kp 指数（每分钟）。
const KP_URL: &str = "https://services.swpc.noaa.gov/json/planetary_k_index_1m.json";
/// NOAA SWPC 太阳黑子数与太阳通量（月度）。
const SOLAR_URL: &str =
  "https://services.swpc.noaa.gov/json/solar-cycle/observed-solar-cycle-indices.json";

/// 把 ISO 时间 `2026-09-30T02:48:00` 显示为 `2026-09-30 02:48`。
fn fmt_iso_time(s: &str) -> String {
  if s.len() >= 16 {
    s[..16].replace('T', " ")
  } else {
    s.to_owned()
  }
}

/// 条件文字对应的颜色。
fn condition_color(condition: &str) -> &'static str {
  match condition {
    "Good" => "text-emerald-700 dark:text-emerald-400",
    "Fair" => "text-amber-700 dark:text-amber-400",
    "Poor" => "text-red-700 dark:text-red-400",
    _ => "text-foreground",
  }
}

/// 服务端 `/api/alerts` 返回的一条空间天气警报。
#[derive(Deserialize, Clone)]
struct Alert {
  product_id: String,
  issue_time: String,
  level: String,
  message: String,
}

/// 警报级别文字颜色。
fn level_color(level: &str) -> &'static str {
  match level.chars().next() {
    Some('G') => "text-red-700 dark:text-red-400",
    Some('S') => "text-purple-700 dark:text-purple-300",
    Some('R') => "text-orange-700 dark:text-orange-400",
    _ => "text-foreground",
  }
}
