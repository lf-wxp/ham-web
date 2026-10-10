//! 浏览器本地（`localStorage`）持久化的进度结构。
//!
//! 字段名与 key 格式与旧版（Next.js）完全一致，升级后用户已有进度可继续使用。

use serde::{Deserialize, Serialize};

use crate::bank::Bank;
use crate::question::QuestionItem;

/// 练习恢复提示最少已答题数。
pub const RESUME_MIN_ANSWERED: usize = 3;
/// 练习进度有效期：7 天。
pub const RESUME_EXPIRY_MS: i64 = 7 * 24 * 60 * 60 * 1000;

fn answered_count(answers: &[Option<Vec<String>>]) -> usize {
  answers.iter().filter(|a| a.is_some()).count()
}

/// 练习进度。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PracticeSavedState {
  /// 结构版本（1 或 2）。
  pub version: u8,
  /// 题库。
  pub bank: Bank,
  /// 题库版本 ID。
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub version_id: Option<String>,
  /// 保存时间（毫秒时间戳）。
  pub timestamp: i64,
  /// 当前位置。
  pub index: usize,
  /// 题序：`sequential` / `random`。
  pub order: String,
  /// 是否显示答案。
  pub show_answer: bool,
  /// 是否显示解析。
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub show_explanation: Option<bool>,
  /// 当前题序相对原始题库的下标。
  pub order_indices: Vec<usize>,
  /// 按位置保存的作答。
  pub answers_by_position: Vec<Option<Vec<String>>>,
  /// 原始题库题目总数。
  pub total: usize,
}

impl PracticeSavedState {
  /// 存储 key。
  #[must_use]
  pub fn storage_key(bank: Bank, version_id: Option<&str>) -> String {
    match version_id {
      Some(v) => format!("practice:{v}:{bank}"),
      None => format!("practice:{bank}"),
    }
  }

  /// 结构版本是否受支持。
  #[must_use]
  pub const fn is_supported(&self) -> bool {
    self.version >= 1 && self.version <= 2
  }

  /// 是否值得提示恢复进度。
  #[must_use]
  pub fn should_resume(&self, now_ms: i64) -> bool {
    if now_ms - self.timestamp > RESUME_EXPIRY_MS {
      return false;
    }
    if answered_count(&self.answers_by_position) < RESUME_MIN_ANSWERED {
      return false;
    }
    self.index > 0 && self.index + 1 < self.total
  }
}

/// 考试进度。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamSavedState {
  /// 结构版本（1 或 2）。
  pub version: u8,
  /// 题库。
  pub bank: Bank,
  /// 题库版本 ID。
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub version_id: Option<String>,
  /// 保存时间。
  pub timestamp: i64,
  /// 考试结束的绝对时间（毫秒时间戳）。
  pub end_at_ms: i64,
  /// 当前位置。
  pub index: usize,
  /// 按位置保存的作答。
  pub answers_by_position: Vec<Option<Vec<String>>>,
  /// 按位置保存的标记。
  pub flags_by_position: Vec<bool>,
  /// 抽中的题目数。
  pub total: usize,
  /// 抽中题目的稳定标识（`id` 或 `J`），用于重建题目集合。
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub question_ids: Option<Vec<Option<String>>>,
  /// 题目快照（兜底）。
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub questions_snapshot: Option<Vec<QuestionItem>>,
}

impl ExamSavedState {
  /// 存储 key。
  #[must_use]
  pub fn storage_key(bank: Bank, version_id: Option<&str>) -> String {
    match version_id {
      Some(v) => format!("exam:savedState:{v}:{bank}"),
      None => format!("exam:savedState:{bank}"),
    }
  }

  /// 是否为可用（版本受支持且未超时）的进度。
  #[must_use]
  pub const fn is_valid(&self, now_ms: i64) -> bool {
    self.version >= 1 && self.version <= 2 && self.end_at_ms > now_ms
  }

  /// 已答题数。
  #[must_use]
  pub fn answered(&self) -> usize {
    answered_count(&self.answers_by_position)
  }

  /// 是否值得提示恢复考试。
  #[must_use]
  pub fn should_resume(&self, now_ms: i64) -> bool {
    if self.end_at_ms <= now_ms {
      return false;
    }
    if self.answered() < 1 && self.index == 0 {
      return false;
    }
    self.total > 0
  }

  /// 根据保存的题目标识从完整题库重建抽中的题目。
  #[must_use]
  pub fn reconstruct(&self, all: &[QuestionItem]) -> Vec<QuestionItem> {
    if let Some(ids) = self.question_ids.as_ref().filter(|ids| !ids.is_empty()) {
      let restored: Vec<QuestionItem> = ids
        .iter()
        .filter_map(|id| {
          let id = id.as_deref()?;
          all
            .iter()
            .find(|q| q.stable_id().as_deref() == Some(id))
            .cloned()
        })
        .collect();
      if restored.len() == self.total {
        return restored;
      }
    }
    if let Some(snapshot) = self.questions_snapshot.as_ref().filter(|s| !s.is_empty()) {
      return snapshot.iter().take(self.total).cloned().collect();
    }
    all
      .iter()
      .take(self.total.min(all.len()))
      .cloned()
      .collect()
  }
}

/// 其他本地存储 key。
pub mod keys {
  use crate::bank::Bank;

  /// 练习上次使用的题序。
  pub const PRACTICE_LAST_MODE: &str = "practice:lastMode";
  /// 主题。
  pub const THEME: &str = "theme";
  /// 像素动效偏好（`on` / `off`，缺省为 `on`）。
  pub const PIXEL_MOTION: &str = "ui:pixelMotion";
  /// 易读字体偏好（`on` / `off`，缺省为 `off`）：正文切回抗锯齿字体。
  pub const READABLE_FONT: &str = "ui:readableFont";
  /// 配色方案 id（[`crate::color_scheme`]；缺省或不认识时用默认方案）。
  pub const COLOR_SCHEME: &str = "ui:colorScheme";
  /// 闯关星级（[`crate::rpg::StageStars`]）。
  pub const RPG_STARS: &str = "rpg-stars";
  /// 怪物图鉴（[`crate::rpg::Bestiary`]）。
  pub const RPG_BESTIARY: &str = "rpg-bestiary";
  /// 界面语言（`zh` / `en`）。
  pub const LOCALE: &str = "locale";
  /// 练习快捷键说明是否已展示。
  pub const HELP_SEEN_PRACTICE: &str = "ui:shortcutsHelpSeen:practice";
  /// 考试快捷键说明是否已展示。
  pub const HELP_SEEN_EXAM: &str = "ui:shortcutsHelpSeen:exam";

  /// 本题库不再提示恢复。
  #[must_use]
  pub fn no_resume(bank: Bank, version_id: Option<&str>) -> String {
    match version_id {
      Some(v) => format!("practice:noResumePrompt:{v}:{bank}"),
      None => format!("practice:noResumePrompt:{bank}"),
    }
  }

  /// 考试答题卡筛选偏好。
  #[must_use]
  pub fn answer_card_filter(bank: Bank) -> String {
    format!("exam:answerCardFilter:{bank}")
  }

  /// 考试交卷后是否显示解析。
  #[must_use]
  pub fn exam_show_explanation(bank: Bank) -> String {
    format!("exam:showExplanation:{bank}")
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn practice(index: usize, answered: usize, total: usize, timestamp: i64) -> PracticeSavedState {
    let mut answers = vec![None; total];
    for a in answers.iter_mut().take(answered) {
      *a = Some(vec!["A".to_owned()]);
    }
    PracticeSavedState {
      version: 2,
      bank: Bank::A,
      version_id: Some("2025-10".into()),
      timestamp,
      index,
      order: "sequential".into(),
      show_answer: true,
      show_explanation: Some(true),
      order_indices: (0..total).collect(),
      answers_by_position: answers,
      total,
    }
  }

  #[test]
  fn practice_resume_rules() {
    assert!(practice(5, 3, 10, 0).should_resume(1000));
    assert!(!practice(5, 2, 10, 0).should_resume(1000));
    assert!(!practice(0, 3, 10, 0).should_resume(1000));
    assert!(!practice(9, 3, 10, 0).should_resume(1000));
    assert!(!practice(5, 3, 10, 0).should_resume(RESUME_EXPIRY_MS + 1));
  }

  #[test]
  fn practice_state_roundtrip_uses_camel_case() {
    let json = serde_json::to_string(&practice(1, 1, 2, 5)).expect("serialize");
    assert!(json.contains("\"versionId\":\"2025-10\""));
    assert!(json.contains("\"answersByPosition\":[[\"A\"],null]"));
    assert_eq!(
      PracticeSavedState::storage_key(Bank::B, Some("v")),
      "practice:v:B"
    );
  }
}
