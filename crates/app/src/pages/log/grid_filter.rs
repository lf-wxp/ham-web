use serde::{Deserialize, Serialize};

/// QSL 状态筛选维度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(super) enum QslStatus {
  /// 已确认（QSL_RCVD）。
  Confirmed,
  /// 已寄出未确认（QSL_SENT 且未 QSL_RCVD）。
  SentPending,
  /// 未寄出。
  #[default]
  NotSent,
}

/// 网格地图筛选状态（用于 localStorage 持久化）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct GridFilters {
  pub(super) band: Option<String>,
  pub(super) mode: Option<String>,
  pub(super) dxcc: Option<String>,
  pub(super) year: Option<String>,
  pub(super) callsign: String,
  pub(super) qsl: Option<QslStatus>,
  #[serde(default)]
  pub(super) paths: bool,
}
