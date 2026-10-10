//! 闯关档案：把本地存档读成 HUD / 地图 / 图鉴要用的数据，并读写星级与怪物图鉴。
//!
//! 规则都在 `ham_web_core::rpg`（纯函数、带测试）。这里只做浏览器相关的事：
//! 读 `localStorage`、拼 [`XpSources`]、写回持久化字段。
//!
//! 经验值**不单独存**：每次由学习存档推算（见 `rpg` 模块文档的取舍），所以这里没有
//! 「增加经验」的写入口 —— 答对一题会照常写进 `study-stats`，经验自然就涨了。

use ham_web_core::daily_challenge::DailyResults;
use ham_web_core::rpg::{Bestiary, LevelInfo, StageStars, XpSources, level_of, rank_key, total_xp};
use ham_web_core::saved_state::keys;
use ham_web_core::{Bank, ExamRule};
use serde::Deserialize;

use crate::i18n::{t, tf};
use crate::util::storage;

/// 打卡状态（精简，与 `achievements.rs` 读的是同一份 `daily-checkin`）。
#[derive(Deserialize, Default)]
struct Checkin {
  #[serde(default)]
  streak: u32,
}

/// 玩家档案快照。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
  /// 总经验。
  pub xp: u32,
  /// 等级与本级进度。
  pub level: LevelInfo,
  /// 连续打卡天数。
  pub streak: u32,
}

/// 从本地存档读取玩家档案。
///
/// 每次调用都会读几份 JSON：HUD 在路由切换与答题后才刷新，不放在渲染热路径里反复算。
pub fn load_profile() -> Profile {
  let study = crate::study::load_stats();
  let checkin: Checkin = storage::get_json("daily-checkin").unwrap_or_default();
  let challenge = storage::get_json::<DailyResults>("daily-challenge").map_or(0, |r| r.days.len());

  let history = crate::exam_history::load();
  // 弱项组卷（`weak`）题目偏难，不计入合格 / 满分（与成就判定、备考评估口径一致）。
  let counted: Vec<_> = history.iter().filter(|r| !r.weak).collect();
  let exam_passed = counted
    .iter()
    .filter(|r| r.correct >= ExamRule::of(Bank::from_param(Some(&r.bank))).pass)
    .count();
  let exam_perfect = counted
    .iter()
    .filter(|r| r.total > 0 && r.correct == r.total)
    .count();

  let sources = XpSources {
    total_correct: study.total.correct,
    challenge_count: u32::try_from(challenge).unwrap_or(u32::MAX),
    exam_count: u32::try_from(history.len()).unwrap_or(u32::MAX),
    exam_passed: u32::try_from(exam_passed).unwrap_or(u32::MAX),
    exam_perfect: u32::try_from(exam_perfect).unwrap_or(u32::MAX),
    streak_days: checkin.streak,
  };
  let xp = total_xp(&sources);
  Profile {
    xp,
    level: level_of(xp),
    streak: checkin.streak,
  }
}

/// 等级称号（按 [`rank_key`] 分档）。
pub fn rank_label(level: u32) -> String {
  match rank_key(level) {
    "listener" => t("shell.rank-listener"),
    "novice" => t("shell.rank-novice"),
    "operator" => t("shell.rank-operator"),
    "dxer" => t("shell.rank-dxer"),
    _ => t("shell.rank-elmer"),
  }
}

/// 「本级经验 / 升级所需」的读数，如 `120 / 440`；满级返回 `MAX`。
#[must_use]
pub fn xp_progress(level: LevelInfo) -> String {
  if level.is_max() {
    t("shell.hud-max-level")
  } else {
    tf(
      "shell.hud-xp-progress",
      &[&level.into_level.to_string(), &level.needed.to_string()],
    )
  }
}

/// 读取关卡星级。
pub fn load_stars() -> StageStars {
  storage::get_json(keys::RPG_STARS).unwrap_or_default()
}

/// 记录一次通关并写回；返回是否刷新了该关的历史最佳。
pub fn record_stage(stage: &str, stars: u8) -> bool {
  let mut all = load_stars();
  let improved = all.record(stage, stars);
  if improved {
    storage::set_json(keys::RPG_STARS, &all);
  }
  improved
}

/// 读取怪物图鉴。
pub fn load_bestiary() -> Bestiary {
  storage::get_json(keys::RPG_BESTIARY).unwrap_or_default()
}

/// 记录击败一只怪物（错题 key）并写回；返回是否首次击败。
pub fn defeat_monster(key: &str) -> bool {
  let mut dex = load_bestiary();
  let first = dex.defeat(key);
  if first {
    storage::set_json(keys::RPG_BESTIARY, &dex);
  }
  first
}

/// 图鉴专题筛选里代表「没有分类码」的哨兵值。
///
/// 不能用空串：图鉴的「全部专题」筛选项就是空串，两者撞在一起会让「其它」既选中不了、
/// 又筛不出未分类的题（点「其它」时 `on_change("")` 等价于「全部」）。
pub const OTHER_TOPIC: &str = "__other__";

/// 错题的专题是否命中筛选值；[`OTHER_TOPIC`] 命中「没有分类码」的题。
#[must_use]
pub fn topic_matches(top_key: Option<&str>, filter: &str) -> bool {
  if filter == OTHER_TOPIC {
    top_key.is_none()
  } else {
    top_key == Some(filter)
  }
}

/// 怪物精灵名，下标对应 [`ham_web_core::rpg::monster_kind`]。
///
/// 数量与 `MONSTER_KINDS` 对齐；见下面的测试。
const MONSTER_SPRITES: [&str; 5] = ["mon_static", "mon_swr", "mon_noise", "mon_spur", "mon_fade"];

/// 某专题错题对应的怪物精灵（同一专题永远是同一种怪）。
#[must_use]
pub fn monster_sprite(top_key: &str) -> &'static str {
  let kind = usize::from(ham_web_core::rpg::monster_kind(top_key));
  MONSTER_SPRITES[kind % MONSTER_SPRITES.len()]
}

#[cfg(test)]
mod tests {
  use super::{MONSTER_SPRITES, monster_sprite};
  use crate::icons::sprite_exists;
  use ham_web_core::rpg::MONSTER_KINDS;

  #[test]
  fn monster_sprites_match_core_kinds_and_exist() {
    assert_eq!(MONSTER_SPRITES.len(), MONSTER_KINDS as usize);
    for name in MONSTER_SPRITES {
      assert!(sprite_exists(name), "怪物精灵 {name} 不存在");
    }
  }

  #[test]
  fn every_top_category_maps_to_a_real_monster() {
    for top in ham_web_core::categories::TOP_CATEGORIES {
      assert!(sprite_exists(monster_sprite(top.key)));
    }
  }

  #[test]
  fn other_topic_matches_only_uncategorized() {
    use super::{OTHER_TOPIC, topic_matches};
    // 「其它」是独立哨兵，不能靠空串表达 —— 图鉴的「全部专题」才是空串。
    assert!(topic_matches(None, OTHER_TOPIC));
    assert!(!topic_matches(Some("法规"), OTHER_TOPIC));
    assert!(topic_matches(Some("法规"), "法规"));
    assert!(
      !topic_matches(None, "法规"),
      "没分类码的题不该被算进具体专题"
    );
    assert!(topic_matches(Some("其他"), "其他"), "哨兵不占用真实分类名");
  }
}
