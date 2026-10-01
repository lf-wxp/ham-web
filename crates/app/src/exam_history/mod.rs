//! 模拟考试成绩历史：本地存储与趋势展示。

mod exam_trend;
mod history_chart;

pub use exam_trend::ExamTrend;
pub use history_chart::HistoryChart;

use ham_web_core::exam_history::ExamRecord;
use ham_web_core::{Bank, ExamScore};

use crate::util::{now_ms, storage};

const KEY: &str = "exam-history";
const MAX: usize = 50;

/// 保存一次考试成绩（最多保留最近 50 次）；`weak` 为薄弱项组卷。
pub fn save(bank: Bank, score: ExamScore, weak: bool) {
  let mut history = load();
  history.push(ExamRecord {
    bank: bank.as_str().to_owned(),
    correct: score.correct,
    total: score.total,
    timestamp: now_ms(),
    weak,
  });
  if history.len() > MAX {
    let drain = history.len() - MAX;
    history.drain(..drain);
  }
  storage::set_json(KEY, &history);
}

/// 全部考试记录。
pub fn load() -> Vec<ExamRecord> {
  storage::get_json(KEY).unwrap_or_default()
}
