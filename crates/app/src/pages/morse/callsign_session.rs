use ham_web_core::cw_runner::{Exchange, QsoResult, RunnerStats};

use crate::util::storage;

/// 本地存储键。
pub(super) const KEY: &str = "morse-runner";
/// 每轮通联数。
pub(super) const SESSION_QSOS: u32 = 10;

/// 已完成的一次通联。
#[derive(Clone)]
pub(super) struct Logged {
  pub(super) ex: Exchange,
  pub(super) result: QsoResult,
  pub(super) call: String,
  pub(super) serial: String,
  pub(super) wpm: u32,
  pub(super) replays: u32,
}

/// 进行中的一轮。
#[derive(Clone)]
pub(super) struct Session {
  pub(super) current: Exchange,
  /// 叠听模式下的干扰台（发在目标台之前）。
  pub(super) decoy: Option<Exchange>,
  pub(super) replays: u32,
  /// 跳过的通联数（不计分，但计入总进度）。
  pub(super) skipped: u32,
  pub(super) log: Vec<Logged>,
}

impl Session {
  /// 已进行（含跳过）的通联数。
  pub(super) fn attempted(&self) -> u32 {
    self.log.len() as u32 + self.skipped
  }

  pub(super) fn score(&self) -> u32 {
    self.log.iter().map(|l| l.result.points).sum()
  }

  pub(super) fn finished(&self) -> bool {
    self.attempted() >= SESSION_QSOS
  }
}

pub(super) fn save(s: &RunnerStats) {
  storage::set_json(KEY, s);
}
