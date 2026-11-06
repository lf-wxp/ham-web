//! 首页每日打卡的持久化结构（`localStorage` 的 `daily-checkin`）。
//!
//! 类型放在 core 是因为账号同步的服务端要把它存进数据库表，前后端必须是同一份定义。

use serde::{Deserialize, Serialize};

/// 学习打卡状态。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckinState {
  /// 最近一次打卡的日期（`YYYYMMDD`，UTC）。
  pub last_date: String,
  /// 连续打卡天数。
  pub streak: usize,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_stored_shape_is_pinned() {
    let state = CheckinState {
      last_date: "20261105".into(),
      streak: 7,
    };
    let json = r#"{"last_date":"20261105","streak":7}"#;
    assert_eq!(serde_json::to_string(&state).unwrap(), json);
    assert_eq!(serde_json::from_str::<CheckinState>(json).unwrap(), state);
  }
}
