use ham_web_core::contest::{CONTESTS, ContestDef, contest};
use serde::{Deserialize, Serialize};

use crate::pages::log::{LogEntry, utc_now_time, utc_today};

/// 当前竞赛设置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Session {
  pub(super) contest_id: String,
  /// 本场开始时间（UTC，`YYYY-MM-DD HH:MM`），之前的同名竞赛通联不计入。
  pub(super) start: String,
  /// 我发出的固定交换（CQ 分区 / 功率），序号类交换自动递增。
  pub(super) my_exch: String,
  pub(super) freq: String,
  pub(super) mode: String,
  #[serde(default)]
  pub(super) power: String,
}

impl Default for Session {
  fn default() -> Self {
    Self {
      contest_id: CONTESTS[0].id.to_owned(),
      start: format!("{} {}", utc_today(), utc_now_time()),
      my_exch: String::new(),
      freq: "14.025".to_owned(),
      mode: "CW".to_owned(),
      power: "LOW".to_owned(),
    }
  }
}

impl Session {
  pub(super) fn def(&self) -> &'static ContestDef {
    contest(&self.contest_id).unwrap_or(&CONTESTS[0])
  }

  pub(super) fn mode(&self) -> String {
    match self.def().mode {
      Some(m) => m.to_owned(),
      None => self.mode.clone(),
    }
  }

  pub(super) fn includes(&self, e: &LogEntry) -> bool {
    e.contest_id == self.contest_id && format!("{} {}", e.date, e.time) >= self.start
  }
}
