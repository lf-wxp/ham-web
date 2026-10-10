//! 闯关（RPG）规则：经验与等级、关卡星级、答题伤害与生命、Boss 战评级、怪物击败。
//!
//! 这一层**只做规则**，不碰浏览器、存储与视图（见 AGENTS.md「不依赖浏览器的逻辑放 core」）。
//!
//! # 设计取舍：经验值从既有存档「推算」，不另存一份
//!
//! 学习数据（累计答对、模拟考试记录、每日挑战、错题本）早就在本地存着。
//! 若再单独存一个 `xp` 字段，就要处理「旧用户升级后经验归零」「两份数据对不上」「备份合并时怎么合」
//! 这些问题。所以经验值是 [`XpSources`] 的**纯函数**：同样的存档永远算出同样的等级，
//! 旧用户第一次打开就能看到自己的等级，备份导入后也自动一致，没有迁移，也没有丢数据的可能。
//!
//! 代价是「经验只增不减」靠的是来源本身只增不减（累计答对、考试次数都是单调的）。
//! 错题被移出（掌握）不会扣经验 —— 来源里没有「当前错题数」。
//!
//! 需要**持久化**的只有两样：关卡星级（[`StageStars`]，一次通关的历史最佳）与
//! 已击败的怪物集合（[`Bestiary`]）。两者都有 `Default`，缺省即「一无所有」，旧存档读不到就是空。

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::categories::{TOP_CATEGORIES, top_of};
use crate::exam::shuffle_in_place;
use crate::question::QuestionItem;

// ---------------------------------------------------------------------------
// 经验与等级
// ---------------------------------------------------------------------------

/// 答对一题的基础经验。
pub const XP_PER_CORRECT: u32 = 10;
/// 每日挑战完成一次的经验。
pub const XP_PER_CHALLENGE: u32 = 50;
/// 模拟考试：每次交卷的参与经验。
pub const XP_PER_EXAM: u32 = 30;
/// 模拟考试合格的额外经验。
pub const XP_EXAM_PASS: u32 = 120;
/// 模拟考试满分的额外经验。
pub const XP_EXAM_PERFECT: u32 = 200;
/// 连续打卡每天的加成（封顶 [`STREAK_BONUS_CAP`] 天）。
pub const XP_PER_STREAK_DAY: u32 = 5;
/// 连续打卡加成的天数上限，避免长期用户经验无限膨胀。
pub const STREAK_BONUS_CAP: u32 = 30;

/// 最高等级。
pub const MAX_LEVEL: u32 = 50;

/// 推算经验值所需的输入（由前端从本地存档构造）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct XpSources {
  /// 累计答对数（`StudyStats.total.correct`）。
  pub total_correct: u32,
  /// 每日挑战完成次数。
  pub challenge_count: u32,
  /// 模拟考试交卷次数。
  pub exam_count: u32,
  /// 模拟考试合格次数。
  pub exam_passed: u32,
  /// 模拟考试满分次数。
  pub exam_perfect: u32,
  /// 当前连续打卡天数。
  pub streak_days: u32,
}

/// 由存档推算总经验。
#[must_use]
pub fn total_xp(s: &XpSources) -> u32 {
  let streak_bonus = s.streak_days.min(STREAK_BONUS_CAP) * XP_PER_STREAK_DAY;
  s.total_correct
    .saturating_mul(XP_PER_CORRECT)
    .saturating_add(s.challenge_count.saturating_mul(XP_PER_CHALLENGE))
    .saturating_add(s.exam_count.saturating_mul(XP_PER_EXAM))
    .saturating_add(s.exam_passed.saturating_mul(XP_EXAM_PASS))
    .saturating_add(s.exam_perfect.saturating_mul(XP_EXAM_PERFECT))
    .saturating_add(streak_bonus)
}

/// 升到下一级所需的经验：`level` 为当前等级（1 起）。
///
/// 曲线 `80 + 40 * (level - 1)`：线性增长。刻意不用指数曲线 —— 备考类应用的用户
/// 做题量有限，指数曲线会让第 10 级之后「怎么做都不升级」，反而打击积极性。
/// 1→2 需要 80（8 题），10→11 需要 440，49→50 需要 2000。
#[must_use]
pub const fn xp_to_next(level: u32) -> u32 {
  80 + 40 * (level.saturating_sub(1))
}

/// 等级进度。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelInfo {
  /// 当前等级（1 起，最高 [`MAX_LEVEL`]）。
  pub level: u32,
  /// 本级已获得的经验。
  pub into_level: u32,
  /// 升级所需的经验；满级时为 0。
  pub needed: u32,
}

impl LevelInfo {
  /// 本级进度百分比（0–100）；满级恒为 100。
  #[must_use]
  pub fn percent(self) -> u32 {
    // 满级时 `needed` 为 0：除法返回 `None`，按 100% 展示。
    (self.into_level * 100)
      .checked_div(self.needed)
      .map_or(100, |p| p.min(100))
  }

  /// 是否已满级。
  #[must_use]
  pub const fn is_max(self) -> bool {
    self.level >= MAX_LEVEL
  }
}

/// 由总经验算出等级与本级进度。
#[must_use]
pub fn level_of(total_xp: u32) -> LevelInfo {
  let mut level = 1;
  let mut left = total_xp;
  while level < MAX_LEVEL {
    let need = xp_to_next(level);
    if left < need {
      return LevelInfo {
        level,
        into_level: left,
        needed: need,
      };
    }
    left -= need;
    level += 1;
  }
  LevelInfo {
    level: MAX_LEVEL,
    into_level: 0,
    needed: 0,
  }
}

/// 称号：按等级分档，给等级一个有「业余无线电味」的名字。返回词条 key 后缀（由视图层翻译）。
#[must_use]
pub const fn rank_key(level: u32) -> &'static str {
  match level {
    0..=4 => "listener",
    5..=9 => "novice",
    10..=19 => "operator",
    20..=34 => "dxer",
    _ => "elmer",
  }
}

// ---------------------------------------------------------------------------
// 回合战斗：答题 = 一回合
// ---------------------------------------------------------------------------

/// 玩家满血。
pub const PLAYER_MAX_HP: u32 = 5;

/// 一场战斗的状态（刷题关卡 / 复仇战斗共用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Battle {
  /// 玩家当前生命。
  pub hp: u32,
  /// 怪物（关卡）剩余生命：每题一点，答对扣一。
  pub enemy_hp: u32,
  /// 怪物满血（= 关卡题数）。
  pub enemy_max: u32,
  /// 当前连击数。
  pub combo: u32,
  /// 最高连击数。
  pub best_combo: u32,
  /// 已答对。
  pub correct: u32,
  /// 已答错。
  pub wrong: u32,
  /// 连击后是否有暴击（额外伤害）。
  ///
  /// 复仇队要求「每只怪物都出场一次」，暴击会让血条提前打空、后面的怪物永远轮不到，
  /// 所以复仇模式关闭暴击（见 [`Battle::without_crit`]）；闯关模式保留，作为连击的奖励。
  pub crit: bool,
}

/// 一回合的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Turn {
  /// 这回合对怪物造成的伤害。
  pub damage: u32,
  /// 这回合玩家受到的伤害。
  pub hurt: u32,
  /// 本回合获得的经验：答对恒为 [`XP_PER_CORRECT`]，与 [`total_xp`] 按累计答对数推算的口径一致。
  pub xp: u32,
}

/// 战斗结局。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
  /// 还在进行。
  Ongoing,
  /// 怪物被击败。
  Victory,
  /// 玩家生命归零。
  Defeat,
}

impl Battle {
  /// 开一场战斗：`questions` 为本关题数（同时也是怪物生命）。
  #[must_use]
  pub const fn new(questions: u32) -> Self {
    Self {
      hp: PLAYER_MAX_HP,
      enemy_hp: questions,
      enemy_max: questions,
      combo: 0,
      best_combo: 0,
      correct: 0,
      wrong: 0,
      crit: true,
    }
  }

  /// 无暴击的战斗：每题恰好一点伤害，题数 = 回合数（答对时）。
  #[must_use]
  pub const fn without_crit(questions: u32) -> Self {
    Self {
      crit: false,
      ..Self::new(questions)
    }
  }

  /// 打出一回合。
  ///
  /// 答对：怪物 -1，连击 +1，连击 ≥ 3 后每次额外 +1 伤害（暴击）但不超过怪物剩余生命；
  /// 答错：玩家 -1，连击清零。
  /// 战斗已经结束后再调用是空操作，返回全 0（界面在结算后不应再接受答题，这里兜底）。
  pub fn answer(&mut self, correct: bool) -> Turn {
    if self.outcome() != Outcome::Ongoing {
      return Turn {
        damage: 0,
        hurt: 0,
        xp: 0,
      };
    }
    if correct {
      self.correct += 1;
      self.combo += 1;
      self.best_combo = self.best_combo.max(self.combo);
      let crit = u32::from(self.crit && self.combo >= CRIT_COMBO);
      let damage = (1 + crit).min(self.enemy_hp);
      self.enemy_hp -= damage;
      Turn {
        damage,
        hurt: 0,
        // 暴击只加伤害、不加经验：经验由存档（累计答对数）推算，这里展示的数字必须与
        // 真正会涨的经验一致，否则结算写「+80」而等级条只涨了 60，用户会觉得被骗。
        xp: XP_PER_CORRECT,
      }
    } else {
      self.wrong += 1;
      self.combo = 0;
      self.hp = self.hp.saturating_sub(1);
      Turn {
        damage: 0,
        hurt: 1,
        xp: 0,
      }
    }
  }

  /// 当前结局。玩家与怪物同时归零时算玩家胜（最后一击答对时玩家不会同时扣血，实际不会发生，
  /// 但「先判胜」保证不会因顺序问题把赢了的局判成输）。
  #[must_use]
  pub const fn outcome(&self) -> Outcome {
    if self.enemy_hp == 0 {
      Outcome::Victory
    } else if self.hp == 0 {
      Outcome::Defeat
    } else {
      Outcome::Ongoing
    }
  }

  /// 玩家生命百分比（0–100），给血条用。
  #[must_use]
  pub fn hp_percent(&self) -> u32 {
    self.hp * 100 / PLAYER_MAX_HP
  }

  /// 怪物生命百分比（0–100）。
  #[must_use]
  pub fn enemy_percent(&self) -> u32 {
    // 空关卡（0 题）没有怪物：`checked_div` 返回 `None`，按 0 处理。
    (self.enemy_hp * 100)
      .checked_div(self.enemy_max)
      .unwrap_or(0)
  }
}

/// 连击到几才开始暴击。
pub const CRIT_COMBO: u32 = 3;

// ---------------------------------------------------------------------------
// 关卡星级
// ---------------------------------------------------------------------------

/// 通关星级（1–3）。通关才有星；败北没有。
///
/// 按「剩余生命」评：满血 3 星，掉 1–2 点 2 星，其余 1 星。
/// 不按答对率评：关卡是「打到怪物死」，答错只是掉血，答对率在战斗里天然受生命约束。
#[must_use]
pub const fn stars_of(b: &Battle) -> u8 {
  if !matches!(b.outcome(), Outcome::Victory) {
    return 0;
  }
  match PLAYER_MAX_HP - b.hp {
    0 => 3,
    1 | 2 => 2,
    _ => 1,
  }
}

/// 关卡星级记录（历史最佳）。key = 关卡 id（见 [`stage_id`]）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageStars {
  #[serde(default)]
  pub best: BTreeMap<String, u8>,
}

impl StageStars {
  /// 记录一次通关；只保留历史最佳，返回是否刷新了纪录（含首次通关）。
  pub fn record(&mut self, stage: &str, stars: u8) -> bool {
    let stars = stars.min(3);
    if stars == 0 {
      return false;
    }
    let e = self.best.entry(stage.to_owned()).or_insert(0);
    if stars > *e {
      *e = stars;
      true
    } else {
      false
    }
  }

  /// 关卡星级（0 = 未通关）。
  #[must_use]
  pub fn stars(&self, stage: &str) -> u8 {
    self.best.get(stage).copied().unwrap_or(0)
  }

  /// 总星数。
  #[must_use]
  pub fn total(&self) -> u32 {
    self.best.values().map(|s| u32::from(*s)).sum()
  }
}

/// 关卡 id：`题库字母 + 分类 key`，如 `A:safety`。
#[must_use]
pub fn stage_id(bank: &str, top_key: &str) -> String {
  format!("{bank}:{top_key}")
}

/// 关卡状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageState {
  /// 前置关卡未通关。
  Locked,
  /// 可以挑战（含刚解锁、尚未通关）。
  Open,
  /// 已通关，带星级。
  Cleared(u8),
}

/// 按顺序解锁：第一关永远开放，之后每一关要求**前一关通关**。
///
/// `order` 是关卡 id 的线性顺序（地图路径）。不做分支解锁：分类只有几个，
/// 线性路径对「备考该先学什么」的引导最清晰，也最容易画成一条蜿蜒的地图。
#[must_use]
pub fn stage_states(order: &[String], stars: &StageStars) -> Vec<StageState> {
  let mut prev_cleared = true;
  order
    .iter()
    .map(|id| {
      let s = stars.stars(id);
      let state = if s > 0 {
        StageState::Cleared(s)
      } else if prev_cleared {
        StageState::Open
      } else {
        StageState::Locked
      };
      prev_cleared = s > 0;
      state
    })
    .collect()
}

// ---------------------------------------------------------------------------
// 关卡清单：一级分类 = 一关
// ---------------------------------------------------------------------------

/// 一关的题数（= 怪物生命）。
///
/// 10 题：一局约 3–5 分钟，够一次通勤；比每日挑战（10 题）同量级，用户已经熟悉这个节奏。
/// 题库里某分类不足 10 题时按实际题数来（见 [`stage_size`]）。
pub const STAGE_SIZE: usize = 10;

/// 地图上的一关。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stage {
  /// 一级分类 key（同时是关卡 id 的后半段）。
  pub key: &'static str,
  /// 关卡名（一级分类名）。
  pub name: &'static str,
  /// 关卡说明（一级分类描述）。
  pub desc: &'static str,
  /// 该题库里这一关可抽的题数。
  pub count: usize,
}

/// 由题库得出地图上的关卡，**顺序固定为 [`TOP_CATEGORIES`] 的顺序**。
///
/// 题量为 0 的分类不上图 —— 比如某个题库根本没出这一类，留一个打不开的空关会卡死后面所有关卡
/// （关卡按顺序解锁）。
#[must_use]
pub fn stages_of(questions: &[QuestionItem]) -> Vec<Stage> {
  TOP_CATEGORIES
    .iter()
    .filter_map(|top| {
      let count = questions
        .iter()
        .filter(|q| {
          q.p_code()
            .and_then(top_of)
            .is_some_and(|t| t.key == top.key)
        })
        .count();
      (count > 0).then_some(Stage {
        key: top.key,
        name: top.name,
        desc: top.desc,
        count,
      })
    })
    .collect()
}

/// 某一关可抽的题（`questions` 里的下标）。
#[must_use]
pub fn stage_pool(questions: &[QuestionItem], key: &str) -> Vec<usize> {
  questions
    .iter()
    .enumerate()
    .filter(|(_, q)| q.p_code().and_then(top_of).is_some_and(|t| t.key == key))
    .map(|(i, _)| i)
    .collect()
}

/// 这一关实际出几题：题库够就是 [`STAGE_SIZE`]，不够就全出。
#[must_use]
pub fn stage_size(pool_len: usize) -> usize {
  pool_len.min(STAGE_SIZE)
}

/// 从候选池里随机抽出本局的题（洗牌后取前 [`stage_size`] 个）。
///
/// 为什么不优先「没做过 / 做错过」的题：这是闯关，不是针对性复习；
/// 针对性复习有图鉴（错题）和练习页（只练没做过）。这里保持随机，每次重打都是新的一局。
#[must_use]
pub fn pick_stage(pool: &[usize], rng: &mut impl FnMut() -> f64) -> Vec<usize> {
  let mut picked = pool.to_vec();
  shuffle_in_place(&mut picked, rng);
  picked.truncate(stage_size(pool.len()));
  picked
}

/// 紧接在 `key` 之后的关卡 key；已是最后一关时为 `None`。
#[must_use]
pub fn next_stage<'a>(stages: &'a [Stage], key: &str) -> Option<&'a Stage> {
  let i = stages.iter().position(|s| s.key == key)?;
  stages.get(i + 1)
}

// ---------------------------------------------------------------------------
// Boss 战：模拟考试
// ---------------------------------------------------------------------------

/// 考试评级。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Grade {
  C,
  B,
  A,
  S,
}

impl Grade {
  /// 评级字母。
  #[must_use]
  pub const fn letter(self) -> &'static str {
    match self {
      Self::S => "S",
      Self::A => "A",
      Self::B => "B",
      Self::C => "C",
    }
  }
}

/// Boss 战结算。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BossResult {
  /// 是否通关（达到合格线）。
  pub passed: bool,
  /// 评级。
  pub grade: Grade,
  /// 本次获得的经验。
  pub xp: u32,
  /// Boss 最终剩余血量百分比（还差多少没打掉；通关时为 0）。
  pub boss_left_percent: u32,
}

/// 结算一场模拟考试。
///
/// Boss 的血量 = 合格线：答对 `pass` 题就击败。评级看**超出合格线多少**占「合格线之上的空间」：
/// - 不及格 → C
/// - 及格但余量 < 1/3 → B
/// - 余量 ≥ 1/3 → A
/// - 满分 → S
///
/// 按「余量占比」而不是固定分数线，三个题库（40 / 60 / 90 题，合格线 30 / 45 / 70）才公平。
#[must_use]
pub fn boss_result(correct: usize, total: usize, pass: usize) -> BossResult {
  let passed = correct >= pass;
  let perfect = total > 0 && correct >= total;
  let headroom = total.saturating_sub(pass).max(1);
  let over = correct.saturating_sub(pass);
  let grade = if !passed {
    Grade::C
  } else if perfect {
    Grade::S
  } else if over.saturating_mul(3) >= headroom {
    Grade::A
  } else {
    Grade::B
  };
  let mut xp = XP_PER_EXAM;
  if passed {
    xp += XP_EXAM_PASS;
  }
  if perfect {
    xp += XP_EXAM_PERFECT;
  }
  let boss_left_percent = boss_hp_percent(correct, pass);
  BossResult {
    passed,
    grade,
    xp,
    boss_left_percent,
  }
}

/// 考试进行中的 Boss 血量百分比：答对越多越低；到合格线即为 0。
#[must_use]
pub fn boss_hp_percent(correct: usize, pass: usize) -> u32 {
  // 合格线为 0 = 不需要答对任何题就算过：Boss 一开始就没有血。
  // 不能用 `.max(1)` 糊弄除零 —— 那会让「0 题合格线」算出满血 Boss。
  if pass == 0 || correct >= pass {
    return 0;
  }
  // `pass` 与 `correct` 都是题数（几十到几百），乘 100 不会溢出 usize；
  // 结果 < 100，必然能放进 u32。
  u32::try_from((pass - correct) * 100 / pass).unwrap_or(100)
}

// ---------------------------------------------------------------------------
// 怪物图鉴：错题
// ---------------------------------------------------------------------------

/// 怪物种类（对应 `icons::pixel::sprite_data` 里的怪物精灵，由视图层映射）。
///
/// 按错题所属一级分类稳定地分配一种外形：同一专题的错题长得一样，
/// 用户能一眼看出「这一片都是静电怪」，而不是每次刷新都换样子。
#[must_use]
pub fn monster_kind(top_key: &str) -> u8 {
  // 简单稳定的字符串哈希（FNV-1a）：不能用 `DefaultHasher`，它不保证跨版本稳定。
  let mut h: u32 = 0x811c_9dc5;
  for b in top_key.bytes() {
    h ^= u32::from(b);
    h = h.wrapping_mul(0x0100_0193);
  }
  u8::try_from(h % MONSTER_KINDS).unwrap_or(0)
}

/// 怪物外形种类数（与视图层的精灵表对齐）。
pub const MONSTER_KINDS: u32 = 5;

/// 已击败的怪物（错题 key 集合）。
///
/// 错题「掌握」后会被移出错题本，图鉴要记住它们曾存在过 ——
/// 否则「击败进度」会随错题清空而归零，失去收集的意义。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bestiary {
  #[serde(default)]
  pub defeated: BTreeSet<String>,
}

impl Bestiary {
  /// 记录击败一只怪物，返回是否首次击败。
  pub fn defeat(&mut self, key: &str) -> bool {
    self.defeated.insert(key.to_owned())
  }

  /// 已击败总数。
  #[must_use]
  pub fn count(&self) -> usize {
    self.defeated.len()
  }

  /// 是否击败过。
  #[must_use]
  pub fn has(&self, key: &str) -> bool {
    self.defeated.contains(key)
  }
}

/// 图鉴进度：已击败 / （已击败 + 仍在错题本里的）。
///
/// 错题本里还没击败的是「活着的怪物」；已击败的不再占错题本。两者相加是用户遇到过的怪物总数。
#[must_use]
pub fn bestiary_percent(defeated: usize, alive: usize) -> u32 {
  let total = defeated + alive;
  // 没遇到过怪物：`checked_div` 返回 `None`，进度为 0。
  (defeated * 100)
    .checked_div(total)
    .map_or(0, |p| u32::try_from(p).unwrap_or(100))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn xp_is_a_pure_function_of_the_save() {
    let s = XpSources {
      total_correct: 100,
      challenge_count: 2,
      exam_count: 1,
      exam_passed: 1,
      exam_perfect: 0,
      streak_days: 3,
    };
    // 1000 + 100 + 30 + 120 + 0 + 15
    assert_eq!(total_xp(&s), 1265);
    assert_eq!(total_xp(&XpSources::default()), 0);
  }

  #[test]
  fn streak_bonus_is_capped() {
    let a = XpSources {
      streak_days: STREAK_BONUS_CAP,
      ..XpSources::default()
    };
    let b = XpSources {
      streak_days: 9999,
      ..XpSources::default()
    };
    assert_eq!(total_xp(&a), total_xp(&b));
  }

  #[test]
  fn xp_does_not_overflow() {
    let s = XpSources {
      total_correct: u32::MAX,
      challenge_count: u32::MAX,
      exam_count: u32::MAX,
      exam_passed: u32::MAX,
      exam_perfect: u32::MAX,
      streak_days: u32::MAX,
    };
    assert_eq!(total_xp(&s), u32::MAX);
  }

  #[test]
  fn level_curve_boundaries() {
    assert_eq!(level_of(0).level, 1);
    assert_eq!(level_of(79).level, 1);
    assert_eq!(level_of(79).into_level, 79);
    // 1→2 需要 80
    assert_eq!(level_of(80).level, 2);
    assert_eq!(level_of(80).into_level, 0);
    // 2→3 需要 120：累计 200 才到 3 级
    assert_eq!(level_of(199).level, 2);
    assert_eq!(level_of(200).level, 3);
  }

  #[test]
  fn level_is_monotonic_and_capped() {
    let mut last = 1;
    for xp in (0..200_000).step_by(37) {
      let l = level_of(xp).level;
      assert!(l >= last, "等级不能随经验增加而下降：xp={xp}");
      assert!(l <= MAX_LEVEL);
      last = l;
    }
    let max = level_of(u32::MAX);
    assert!(max.is_max());
    assert_eq!(max.percent(), 100);
    assert_eq!(max.needed, 0);
  }

  #[test]
  fn level_progress_percent() {
    assert_eq!(level_of(0).percent(), 0);
    assert_eq!(level_of(40).percent(), 50);
    // 刚升级：进度归零
    assert_eq!(level_of(80).percent(), 0);
  }

  #[test]
  fn ranks_cover_every_level() {
    assert_eq!(rank_key(1), "listener");
    assert_eq!(rank_key(5), "novice");
    assert_eq!(rank_key(10), "operator");
    assert_eq!(rank_key(20), "dxer");
    assert_eq!(rank_key(MAX_LEVEL), "elmer");
  }

  #[test]
  fn correct_answer_hurts_the_enemy() {
    let mut b = Battle::new(10);
    let t = b.answer(true);
    assert_eq!((t.damage, t.hurt), (1, 0));
    assert_eq!(b.enemy_hp, 9);
    assert_eq!(b.hp, PLAYER_MAX_HP);
    assert_eq!(b.outcome(), Outcome::Ongoing);
  }

  #[test]
  fn wrong_answer_hurts_the_player_and_breaks_combo() {
    let mut b = Battle::new(10);
    b.answer(true);
    b.answer(true);
    assert_eq!(b.combo, 2);
    let t = b.answer(false);
    assert_eq!((t.damage, t.hurt, t.xp), (0, 1, 0));
    assert_eq!(b.hp, PLAYER_MAX_HP - 1);
    assert_eq!(b.combo, 0);
    assert_eq!(b.best_combo, 2, "最高连击不会因断连而清零");
  }

  #[test]
  fn without_crit_every_answer_is_exactly_one_damage() {
    // 复仇队 4 只怪物：连续答对 4 题必须正好打满 4 回合，不能被暴击提前结束。
    let mut b = Battle::without_crit(4);
    let damage: Vec<u32> = (0..4).map(|_| b.answer(true).damage).collect();
    assert_eq!(damage, vec![1, 1, 1, 1]);
    assert_eq!(b.outcome(), Outcome::Victory);
    assert_eq!(b.correct, 4, "每一题都应出场");
    // 对照：同样 4 题，有暴击时第 3 回合就结束了，第 4 题没有机会出场。
    let mut c = Battle::new(4);
    let mut turns = 0;
    while c.outcome() == Outcome::Ongoing {
      c.answer(true);
      turns += 1;
    }
    assert_eq!(turns, 3);
  }

  #[test]
  fn crit_adds_damage_but_not_xp() {
    // 展示的经验必须等于存档推算出的经验（累计答对数 × 每题经验）。
    let mut b = Battle::new(20);
    let xp: Vec<u32> = (0..5).map(|_| b.answer(true).xp).collect();
    assert!(
      xp.iter().all(|x| *x == XP_PER_CORRECT),
      "暴击不能多给经验：{xp:?}"
    );
  }

  #[test]
  fn combo_triggers_crit_damage() {
    let mut b = Battle::new(20);
    let dmg: Vec<u32> = (0..5).map(|_| b.answer(true).damage).collect();
    // 第 3 次起暴击
    assert_eq!(dmg, vec![1, 1, 2, 2, 2]);
  }

  #[test]
  fn crit_never_overkills() {
    // 3 点生命：1 + 1 + 暴击（本可打 2，但只剩 1）→ 刚好击杀，不会出现「负血」。
    let mut b = Battle::new(3);
    assert_eq!(b.answer(true).damage, 1);
    assert_eq!(b.answer(true).damage, 1);
    let last = b.answer(true);
    assert_eq!(last.damage, 1, "暴击伤害被怪物剩余生命截断");
    assert_eq!(b.enemy_hp, 0);
    assert_eq!(b.outcome(), Outcome::Victory);
  }

  #[test]
  fn defeat_when_hp_runs_out() {
    let mut b = Battle::new(10);
    for _ in 0..PLAYER_MAX_HP {
      b.answer(false);
    }
    assert_eq!(b.hp, 0);
    assert_eq!(b.outcome(), Outcome::Defeat);
  }

  #[test]
  fn finished_battle_ignores_further_answers() {
    let mut b = Battle::new(1);
    b.answer(true);
    assert_eq!(b.outcome(), Outcome::Victory);
    let before = b;
    let t = b.answer(false);
    assert_eq!(
      (t.damage, t.hurt, t.xp),
      (0, 0, 0),
      "结算后再答不应扣血，否则会把赢了的局改成输"
    );
    assert_eq!(b, before);
  }

  #[test]
  fn victory_takes_priority_over_defeat() {
    let b = Battle {
      hp: 0,
      enemy_hp: 0,
      ..Battle::new(5)
    };
    assert_eq!(b.outcome(), Outcome::Victory);
  }

  #[test]
  fn empty_stage_is_an_instant_victory() {
    // 防御：分类下没有题时 Battle::new(0) 不应 panic 或除零。
    let b = Battle::new(0);
    assert_eq!(b.outcome(), Outcome::Victory);
    assert_eq!(b.enemy_percent(), 0);
  }

  #[test]
  fn hp_percentages() {
    let mut b = Battle::new(4);
    assert_eq!((b.hp_percent(), b.enemy_percent()), (100, 100));
    b.answer(false);
    b.answer(true);
    assert_eq!(b.hp_percent(), 80);
    assert_eq!(b.enemy_percent(), 75);
  }

  #[test]
  fn stars_depend_on_remaining_hp() {
    let mut flawless = Battle::new(3);
    for _ in 0..3 {
      flawless.answer(true);
    }
    assert_eq!(stars_of(&flawless), 3);

    let mut hurt = Battle::new(4);
    hurt.answer(false);
    for _ in 0..4 {
      hurt.answer(true);
    }
    assert_eq!(stars_of(&hurt), 2);

    let mut scraped = Battle::new(10);
    for _ in 0..(PLAYER_MAX_HP - 1) {
      scraped.answer(false);
    }
    for _ in 0..10 {
      scraped.answer(true);
    }
    assert_eq!(scraped.hp, 1);
    assert_eq!(stars_of(&scraped), 1);
  }

  #[test]
  fn no_stars_without_victory() {
    let mut b = Battle::new(10);
    for _ in 0..PLAYER_MAX_HP {
      b.answer(false);
    }
    assert_eq!(stars_of(&b), 0);
    assert_eq!(stars_of(&Battle::new(10)), 0, "进行中的战斗也没有星");
  }

  #[test]
  fn stage_stars_keep_the_best() {
    let mut s = StageStars::default();
    assert!(s.record("A:safety", 2), "首次通关算刷新");
    assert!(!s.record("A:safety", 1), "低于历史最佳不覆盖");
    assert_eq!(s.stars("A:safety"), 2);
    assert!(s.record("A:safety", 3));
    assert_eq!(s.stars("A:safety"), 3);
    assert!(!s.record("A:safety", 0), "败北不记录");
    assert!(!s.record("A:other", 0));
    assert_eq!(s.stars("A:other"), 0);
    s.record("A:other", 9);
    assert_eq!(s.stars("A:other"), 3, "星级封顶 3");
    assert_eq!(s.total(), 6);
  }

  #[test]
  fn stages_unlock_in_order() {
    let order: Vec<String> = ["A:a", "A:b", "A:c", "A:d"]
      .iter()
      .map(ToString::to_string)
      .collect();
    let mut s = StageStars::default();
    use StageState::{Cleared, Locked, Open};
    assert_eq!(stage_states(&order, &s), vec![Open, Locked, Locked, Locked]);
    s.record("A:a", 2);
    assert_eq!(
      stage_states(&order, &s),
      vec![Cleared(2), Open, Locked, Locked]
    );
    s.record("A:b", 3);
    assert_eq!(
      stage_states(&order, &s),
      vec![Cleared(2), Cleared(3), Open, Locked]
    );
  }

  #[test]
  fn skipping_ahead_does_not_unlock_the_gap() {
    // 数据异常（比如从备份导入）：C 有星但 B 没有。B 仍应是「可挑战」，D 保持锁定。
    let order: Vec<String> = ["A:a", "A:b", "A:c", "A:d"]
      .iter()
      .map(ToString::to_string)
      .collect();
    let mut s = StageStars::default();
    s.record("A:a", 1);
    s.record("A:c", 3);
    use StageState::{Cleared, Locked, Open};
    assert_eq!(
      stage_states(&order, &s),
      vec![Cleared(1), Open, Cleared(3), Open]
    );
    let _ = Locked;
  }

  #[test]
  fn stage_ids_are_namespaced_by_bank() {
    assert_eq!(stage_id("A", "safety"), "A:safety");
    assert_ne!(stage_id("A", "safety"), stage_id("B", "safety"));
  }

  #[test]
  fn boss_grade_follows_headroom_not_absolute_score() {
    // A 类：40 题，合格线 30，余量 10
    assert_eq!(boss_result(29, 40, 30).grade, Grade::C);
    assert!(!boss_result(29, 40, 30).passed);
    assert_eq!(boss_result(30, 40, 30).grade, Grade::B);
    assert_eq!(boss_result(32, 40, 30).grade, Grade::B, "余量 2/10 < 1/3");
    assert_eq!(boss_result(34, 40, 30).grade, Grade::A, "余量 4/10 ≥ 1/3");
    assert_eq!(boss_result(39, 40, 30).grade, Grade::A);
    assert_eq!(boss_result(40, 40, 30).grade, Grade::S);
  }

  #[test]
  fn boss_grade_is_fair_across_banks() {
    // 同样「刚好及格」在三个题库里都是 B，「满分」都是 S。
    for (total, pass) in [(40, 30), (60, 45), (90, 70)] {
      assert_eq!(boss_result(pass, total, pass).grade, Grade::B);
      assert_eq!(boss_result(total, total, pass).grade, Grade::S);
      assert_eq!(boss_result(0, total, pass).grade, Grade::C);
    }
  }

  #[test]
  fn boss_xp_stacks_with_pass_and_perfect() {
    assert_eq!(boss_result(10, 40, 30).xp, XP_PER_EXAM);
    assert_eq!(boss_result(30, 40, 30).xp, XP_PER_EXAM + XP_EXAM_PASS);
    assert_eq!(
      boss_result(40, 40, 30).xp,
      XP_PER_EXAM + XP_EXAM_PASS + XP_EXAM_PERFECT
    );
  }

  #[test]
  fn boss_hp_drops_as_you_answer() {
    assert_eq!(boss_hp_percent(0, 30), 100);
    assert_eq!(boss_hp_percent(15, 30), 50);
    assert_eq!(boss_hp_percent(30, 30), 0);
    assert_eq!(boss_hp_percent(35, 30), 0, "超过合格线不会变成负血");
    assert_eq!(boss_hp_percent(0, 0), 0, "合格线为 0 不应除零");
  }

  #[test]
  fn boss_result_survives_degenerate_input() {
    // 空卷子 / 合格线为 0 不应 panic。
    let r = boss_result(0, 0, 0);
    assert!(r.passed);
    assert_eq!(boss_result(0, 0, 0).boss_left_percent, 0);
    let _ = boss_result(usize::MAX, usize::MAX, usize::MAX);
    // 余量很大、答对数也很大：乘 3 不能溢出。
    let r = boss_result(usize::MAX, usize::MAX, 1);
    assert_eq!(r.grade, Grade::S);
    let _ = boss_result(usize::MAX - 1, usize::MAX, 1);
  }

  #[test]
  fn boss_left_percent_for_failed_run() {
    assert_eq!(boss_result(15, 40, 30).boss_left_percent, 50);
    assert_eq!(boss_result(30, 40, 30).boss_left_percent, 0);
  }

  #[test]
  fn monster_kind_is_stable_and_in_range() {
    for key in [
      "safety",
      "regulations",
      "antennas",
      "propagation",
      "operating",
    ] {
      let k = monster_kind(key);
      assert!(u32::from(k) < MONSTER_KINDS);
      assert_eq!(k, monster_kind(key), "同一专题每次都是同一种怪");
    }
    // 固定值：防止有人换了哈希而让全站怪物外形悄悄改变（改哈希就要同步改这里的数字）。
    assert_eq!(monster_kind("safety"), 2);
    // 空串不做任何一轮混合，直接是初值取模。
    assert_eq!(monster_kind(""), (0x811c_9dc5_u32 % MONSTER_KINDS) as u8);
  }

  #[test]
  fn bestiary_remembers_defeated_monsters() {
    let mut b = Bestiary::default();
    assert!(b.defeat("A-1"));
    assert!(!b.defeat("A-1"), "重复击败不重复计数");
    assert!(b.has("A-1"));
    assert!(!b.has("A-2"));
    assert_eq!(b.count(), 1);
  }

  #[test]
  fn bestiary_progress_combines_alive_and_defeated() {
    assert_eq!(bestiary_percent(0, 0), 0, "没遇到过怪物不应除零");
    assert_eq!(bestiary_percent(3, 1), 75);
    assert_eq!(bestiary_percent(5, 0), 100);
    assert_eq!(bestiary_percent(0, 4), 0);
  }

  #[test]
  fn persisted_state_tolerates_old_or_partial_saves() {
    // 旧存档里没有这些键：反序列化出空值，而不是报错。
    let stars: StageStars = serde_json::from_str("{}").unwrap();
    assert_eq!(stars.total(), 0);
    let dex: Bestiary = serde_json::from_str("{}").unwrap();
    assert_eq!(dex.count(), 0);
    // 往返
    let mut s = StageStars::default();
    s.record("A:a", 3);
    let back: StageStars = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    assert_eq!(back, s);
  }

  fn question(n: usize, p: &str) -> QuestionItem {
    use crate::question::{Codes, QuestionOption, QuestionType};
    QuestionItem {
      id: Some(format!("A-{n}")),
      codes: Codes {
        j: Some(format!("LY{n:04}")),
        p: Some(p.into()),
      },
      question: format!("题目 {n}"),
      options: vec![QuestionOption {
        key: "A".into(),
        text: "x".into(),
      }],
      answer_keys: vec!["A".into()],
      kind: QuestionType::Single,
      pages: None,
      image_url: None,
      explanation: None,
    }
  }

  /// 法规 1.1.1、频率下的某个码；取分类表里真实存在的码，避免写死后分类表改动让测试悄悄失效。
  fn code_of(top_key: &str) -> &'static str {
    crate::categories::SUB_CATEGORIES
      .iter()
      .find(|s| s.top == top_key)
      .map(|s| s.code)
      .expect("分类表里每个一级分类都应有二级码")
  }

  #[test]
  fn stages_follow_category_order_and_skip_empty_ones() {
    let law = code_of("法规");
    let freq = code_of("频率");
    // 故意先放「频率」再放「法规」：关卡顺序必须由分类表决定，不能被题目顺序带偏。
    let qs = vec![question(1, freq), question(2, law), question(3, law)];
    let stages = stages_of(&qs);
    let keys: Vec<&str> = stages.iter().map(|s| s.key).collect();
    assert_eq!(keys, vec!["法规", "频率"]);
    assert_eq!(stages[0].count, 2);
    assert_eq!(stages[1].count, 1);
  }

  #[test]
  fn questions_without_a_category_code_are_not_on_the_map() {
    let mut q = question(1, code_of("法规"));
    q.codes.p = None;
    assert!(stages_of(&[q]).is_empty());
    let mut q = question(2, code_of("法规"));
    q.codes.p = Some("999.9.9".into());
    assert!(stages_of(&[q]).is_empty(), "未知分类码不能凭空造出一关");
  }

  #[test]
  fn stage_pool_selects_only_that_categorys_questions() {
    let law = code_of("法规");
    let freq = code_of("频率");
    let qs = vec![question(1, law), question(2, freq), question(3, law)];
    assert_eq!(stage_pool(&qs, "法规"), vec![0, 2]);
    assert_eq!(stage_pool(&qs, "频率"), vec![1]);
    assert!(stage_pool(&qs, "不存在").is_empty());
  }

  #[test]
  fn stage_size_is_capped_but_never_inflated() {
    assert_eq!(stage_size(0), 0);
    assert_eq!(stage_size(4), 4);
    assert_eq!(stage_size(STAGE_SIZE), STAGE_SIZE);
    assert_eq!(stage_size(500), STAGE_SIZE);
  }

  #[test]
  fn pick_stage_draws_distinct_questions_from_the_pool() {
    let pool: Vec<usize> = (100..140).collect();
    let mut n = 0u32;
    let mut rng = move || {
      n = n.wrapping_mul(1_103_515_245).wrapping_add(12_345);
      f64::from(n % 1000) / 1000.0
    };
    let picked = pick_stage(&pool, &mut rng);
    assert_eq!(picked.len(), STAGE_SIZE);
    assert!(picked.iter().all(|i| pool.contains(i)));
    let mut sorted = picked.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), picked.len(), "同一局不能抽到重复的题");
  }

  #[test]
  fn pick_stage_on_a_small_pool_uses_everything() {
    let mut rng = || 0.5;
    let mut picked = pick_stage(&[7, 8, 9], &mut rng);
    picked.sort_unstable();
    assert_eq!(picked, vec![7, 8, 9]);
    assert!(pick_stage(&[], &mut rng).is_empty());
  }

  #[test]
  fn next_stage_walks_the_map_and_stops_at_the_end() {
    let stages = [
      Stage {
        key: "a",
        name: "A",
        desc: "",
        count: 1,
      },
      Stage {
        key: "b",
        name: "B",
        desc: "",
        count: 1,
      },
    ];
    assert_eq!(next_stage(&stages, "a").map(|s| s.key), Some("b"));
    assert!(next_stage(&stages, "b").is_none());
    assert!(next_stage(&stages, "zzz").is_none());
  }
}
