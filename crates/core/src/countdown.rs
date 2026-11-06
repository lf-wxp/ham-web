//! 倒计时与提醒的持久化结构（`localStorage` 的 `countdowns`）。
//!
//! 类型放在 core 而不是页面里：账号同步的服务端要把它存进数据库表，
//! 前后端必须是同一份定义，页面加字段服务端才不会悄悄丢掉。

use serde::{Deserialize, Serialize};

/// 一个倒计时事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountdownEvent {
  /// 本机自增编号，列表内唯一。
  pub id: u64,
  pub title: String,
  /// 目标时间（毫秒时间戳）。
  pub target_ms: i64,
}

/// 全部倒计时。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountdownList {
  pub events: Vec<CountdownEvent>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_stored_shape_is_pinned() {
    // 这是写进 localStorage、也进云端备份的格式：字段名和顺序都不能悄悄变。
    let list = CountdownList {
      events: vec![CountdownEvent {
        id: 1,
        title: "考试".into(),
        target_ms: 1_700_000_000_000,
      }],
    };
    let json = r#"{"events":[{"id":1,"title":"考试","target_ms":1700000000000}]}"#;
    assert_eq!(serde_json::to_string(&list).unwrap(), json);
    assert_eq!(serde_json::from_str::<CountdownList>(json).unwrap(), list);
  }
}
