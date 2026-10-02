//! 单题作答统计：按内容指纹 key 累计每道题的作答次数、正确次数与最近作答时间，
//! 供薄弱项组卷加权与「这道题我练过几次」展示使用。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// 一道题的累计作答统计。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionStat {
  /// 累计作答次数。
  pub attempts: u32,
  /// 其中答对次数。
  pub correct: u32,
  /// 最近一次作答时间（毫秒时间戳）。
  pub last_ms: i64,
  /// 累计作答耗时（毫秒），用于「会但慢」分析与薄弱项加权。
  #[serde(default)]
  pub duration_ms: u64,
}

/// 全部题目的单题统计（key = 内容指纹短哈希，见 [`crate::mistake_book::question_key`]）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionStats {
  pub items: BTreeMap<String, QuestionStat>,
}

impl QuestionStats {
  /// 记录一次作答；`duration_ms` 为本次作答耗时。
  pub fn record(&mut self, key: &str, correct: bool, now_ms: i64, duration_ms: u64) {
    let e = self.items.entry(key.to_owned()).or_default();
    e.attempts += 1;
    e.correct += u32::from(correct);
    e.last_ms = now_ms;
    e.duration_ms += duration_ms;
  }

  /// 某题的正确率（0–1）；作答不足 `min_attempts` 次时为 `None`，避免小样本噪声。
  #[must_use]
  pub fn rate(&self, key: &str, min_attempts: u32) -> Option<f64> {
    let s = self.items.get(key)?;
    (s.attempts >= min_attempts).then(|| f64::from(s.correct) / f64::from(s.attempts))
  }

  /// 某题的平均作答耗时（毫秒）；未作答过为 `None`。
  #[must_use]
  pub fn avg_ms(&self, key: &str) -> Option<u64> {
    let s = self.items.get(key)?;
    (s.attempts > 0).then(|| s.duration_ms / u64::from(s.attempts))
  }

  /// 某题的作答统计（未作答过为 `None`）。
  #[must_use]
  pub fn get(&self, key: &str) -> Option<QuestionStat> {
    self.items.get(key).copied()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn records_and_rates() {
    let mut s = QuestionStats::default();
    s.record("a", false, 1, 1000);
    s.record("a", true, 2, 2000);
    s.record("a", true, 3, 3000);
    s.record("b", true, 3, 500);
    assert_eq!(
      s.get("a"),
      Some(QuestionStat {
        attempts: 3,
        correct: 2,
        last_ms: 3,
        duration_ms: 6000
      })
    );
    assert_eq!(s.avg_ms("a"), Some(2000));
    assert_eq!(s.avg_ms("missing"), None);
    // 作答不足阈值时返回 None。
    assert_eq!(s.rate("b", 2), None);
    assert_eq!(s.rate("a", 2), Some(2.0 / 3.0));
    assert_eq!(s.rate("missing", 1), None);
  }
}
