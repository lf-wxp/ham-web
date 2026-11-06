//! 摩尔斯页面持久化的四份小文档：解码练习统计（`morse-stats`）、解码练习设置（`morse-trainer`）、
//! 发报统计（`morse-send`）、播放设置（`morse-settings`）。
//!
//! 类型放在 core 而不是页面里：账号同步的服务端要把它们存进数据库表，
//! 前后端必须是同一份定义，页面加字段服务端才不会悄悄丢掉。
//!
//! 映射一律用 `BTreeMap`：键序确定，序列化的结果每次相同。`HashMap` 的键序随机，
//! 同一份数据每次写出的文本都可能不同，同步时指纹就对不上。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::morse_trainer::{Mode, Scope, Target};

/// 解码练习统计。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MorseStats {
  pub correct: usize,
  pub wrong: usize,
  /// 每个字符（答案文本）的累计答错次数，用于易错字符优先出题。
  pub mistakes: BTreeMap<String, usize>,
  /// 当前连续答对（不持久化，仅运行时）。
  #[serde(skip)]
  pub streak: usize,
  /// 历史最长连续答对。
  #[serde(default)]
  pub best_streak: usize,
  /// 答对过的最高速度（WPM）。
  #[serde(default)]
  pub top_wpm: f64,
  /// 今日日期（`YYYY-MM-DD`）。
  #[serde(default)]
  pub today: String,
  /// 今日正确。
  #[serde(default)]
  pub today_correct: usize,
  /// 今日错误。
  #[serde(default)]
  pub today_wrong: usize,
  /// 每日快照（日期 → 正确 / 错误），用于近 7 天趋势。
  #[serde(default)]
  pub daily: BTreeMap<String, (usize, usize)>,
}

/// 解码练习的设置。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TrainerSettings {
  #[serde(default = "default_mode")]
  pub mode: Mode,
  #[serde(default = "default_scope")]
  pub scope: Scope,
  #[serde(default = "default_target")]
  pub target: Target,
  #[serde(default = "default_wpm")]
  pub wpm: f64,
  #[serde(default = "default_wpm")]
  pub eff_wpm: f64,
  /// 自适应速度：答对加速、答错减速。
  #[serde(default)]
  pub adaptive: bool,
}

const fn default_mode() -> Mode {
  Mode::Listen
}

const fn default_scope() -> Scope {
  Scope::Letters
}

const fn default_target() -> Target {
  Target::Single
}

const fn default_wpm() -> f64 {
  20.0
}

impl Default for TrainerSettings {
  fn default() -> Self {
    Self {
      mode: default_mode(),
      scope: default_scope(),
      target: default_target(),
      wpm: default_wpm(),
      eff_wpm: default_wpm(),
      adaptive: false,
    }
  }
}

/// 发报统计。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendStats {
  pub correct: usize,
  pub wrong: usize,
  pub best_streak: usize,
}

/// 播放设置：音调与音量。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MorseSettings {
  #[serde(default = "default_tone")]
  pub tone_hz: u32,
  #[serde(default = "default_volume")]
  pub volume: f32,
}

const fn default_tone() -> u32 {
  700
}

const fn default_volume() -> f32 {
  1.0
}

impl Default for MorseSettings {
  fn default() -> Self {
    Self {
      tone_hz: default_tone(),
      volume: default_volume(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_stored_shapes_are_pinned() {
    // 写进 localStorage、也进云端备份的格式：字段名和顺序都不能悄悄变。
    let stats = MorseStats {
      correct: 5,
      wrong: 2,
      mistakes: [("K".to_owned(), 3), ("A".to_owned(), 1)].into(),
      streak: 9,
      best_streak: 4,
      top_wpm: 22.5,
      today: "2026-11-05".into(),
      today_correct: 1,
      today_wrong: 0,
      daily: [("2026-11-04".to_owned(), (3, 1))].into(),
    };
    let json = r#"{"correct":5,"wrong":2,"mistakes":{"A":1,"K":3},"best_streak":4,"top_wpm":22.5,"today":"2026-11-05","today_correct":1,"today_wrong":0,"daily":{"2026-11-04":[3,1]}}"#;
    assert_eq!(serde_json::to_string(&stats).unwrap(), json);
    // 当前连续答对只在运行时存在，读回来是 0。
    let back: MorseStats = serde_json::from_str(json).unwrap();
    assert_eq!(back.streak, 0);
    assert_eq!(back.mistakes, stats.mistakes);

    let trainer = TrainerSettings::default();
    let json = r#"{"mode":"Listen","scope":"Letters","target":"Single","wpm":20.0,"eff_wpm":20.0,"adaptive":false}"#;
    assert_eq!(serde_json::to_string(&trainer).unwrap(), json);
    assert_eq!(
      serde_json::from_str::<TrainerSettings>(json).unwrap(),
      trainer
    );

    let send = SendStats {
      correct: 1,
      wrong: 2,
      best_streak: 3,
    };
    let json = r#"{"correct":1,"wrong":2,"best_streak":3}"#;
    assert_eq!(serde_json::to_string(&send).unwrap(), json);

    let json = serde_json::to_string(&MorseSettings::default()).unwrap();
    assert_eq!(json, r#"{"tone_hz":700,"volume":1.0}"#);
  }

  #[test]
  fn missing_fields_fall_back_to_the_defaults_the_pages_used_before() {
    let t: TrainerSettings = serde_json::from_str("{}").unwrap();
    assert_eq!(t, TrainerSettings::default());
    let s: MorseSettings = serde_json::from_str("{}").unwrap();
    assert_eq!(s, MorseSettings::default());
  }
}
