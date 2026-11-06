//! 日志网格地图的筛选状态（`localStorage` 的 `grid-map-filters`）。
//!
//! 类型放在 core 是因为账号同步的服务端要把它存进数据库表，前后端必须是同一份定义。

use serde::{Deserialize, Serialize};

/// QSL 状态筛选维度。
///
/// 序列化直接用变体名（没有 `rename`），这是已经写进用户存储的格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum QslFilter {
  /// 已确认（QSL_RCVD）。
  Confirmed,
  /// 已寄出未确认（QSL_SENT 且未 QSL_RCVD）。
  SentPending,
  /// 未寄出。
  #[default]
  NotSent,
}

impl QslFilter {
  /// 全部取值。
  pub const ALL: [Self; 3] = [Self::Confirmed, Self::SentPending, Self::NotSent];

  /// 序列化里的写法（数据库列用同一个写法）。
  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Confirmed => "Confirmed",
      Self::SentPending => "SentPending",
      Self::NotSent => "NotSent",
    }
  }
}

/// 网格地图筛选状态。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridFilters {
  pub band: Option<String>,
  pub mode: Option<String>,
  pub dxcc: Option<String>,
  pub year: Option<String>,
  pub callsign: String,
  pub qsl: Option<QslFilter>,
  /// 是否显示通联路径。
  #[serde(default)]
  pub paths: bool,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_stored_shape_is_pinned() {
    let f = GridFilters {
      band: Some("20m".into()),
      mode: None,
      dxcc: None,
      year: Some("2026".into()),
      callsign: "BG".into(),
      qsl: Some(QslFilter::SentPending),
      paths: true,
    };
    let json = r#"{"band":"20m","mode":null,"dxcc":null,"year":"2026","callsign":"BG","qsl":"SentPending","paths":true}"#;
    assert_eq!(serde_json::to_string(&f).unwrap(), json);
    assert_eq!(serde_json::from_str::<GridFilters>(json).unwrap(), f);
  }

  #[test]
  fn the_column_spelling_matches_what_serde_writes() {
    // 数据库的 CHECK 与 `as_str` 同一个写法；它必须与序列化一致，否则落库时才发现对不上。
    for q in QslFilter::ALL {
      assert_eq!(
        serde_json::to_string(&q).unwrap(),
        format!("\"{}\"", q.as_str())
      );
    }
  }
}
