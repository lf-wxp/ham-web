//! 牌组：这一局要打的题、对手是谁、打完结算到哪一关。

use ham_web_core::categories::top_of;
use ham_web_core::mistake_book::MistakeRecord;
use ham_web_core::rpg::{
  STAGE_SIZE, StageState, next_stage, pick_stage, stage_id, stage_pool, stage_states, stages_of,
};
use ham_web_core::{Bank, QuestionItem};

use crate::data;
use crate::rpg::{load_stars, monster_sprite};
use crate::study;
use crate::util::{now_ms, random};

/// 这一局的玩法。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Mode {
  /// 闯关：`key` 是一级分类 key。
  Stage { key: String },
  /// 复仇：`key` 指定单只怪物（错题 key），`topic` 把范围限定在某个专题。
  Revenge {
    key: Option<String>,
    topic: Option<String>,
  },
}

/// 对手的称呼；到视图里再翻译，战斗中途切换语言时名字也能跟着变。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Enemy {
  /// 关卡守卫：用一级分类名。
  Stage(&'static str),
  /// 一队复仇的怪物。
  Pack,
  /// 单只怪物。
  Single,
}

/// 牌组之外的对局信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Meta {
  pub(super) enemy: Enemy,
  pub(super) sprite: &'static str,
  /// 闯关模式下写星级用的关卡 id；复仇模式没有。
  pub(super) stage_id: Option<String>,
  /// 闯关模式下的下一关 `(key, 名称)`。
  pub(super) next: Option<(String, &'static str)>,
}

/// 构建牌组失败的原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DeckError {
  /// URL 里没有战斗目标。
  NoTarget,
  /// 题库加载失败。
  Load,
  /// 地图上没有这一关（题库里没有这个分类，或 key 写错）。
  NotFound,
  /// 前一关还没通关；带上前一关的名字，好提示该去哪。
  Locked(&'static str),
  /// 图鉴里没有可打的怪物。
  Empty,
}

/// 一局的牌组。
pub(super) struct Deck {
  pub(super) questions: Vec<QuestionItem>,
  pub(super) meta: Meta,
}

/// 按玩法构建牌组。
pub(super) async fn build(
  mode: &Mode,
  version: Option<&str>,
  bank: Bank,
) -> Result<Deck, DeckError> {
  match mode {
    Mode::Stage { key } => build_stage(key, version, bank).await,
    Mode::Revenge { key, topic } => build_revenge(key.as_deref(), topic.as_deref()),
  }
}

async fn build_stage(key: &str, version: Option<&str>, bank: Bank) -> Result<Deck, DeckError> {
  let all = data::load_bank(version, bank, true)
    .await
    .map_err(|_| DeckError::Load)?;
  let stages = stages_of(&all);
  let at = stages
    .iter()
    .position(|s| s.key == key)
    .ok_or(DeckError::NotFound)?;

  // 直接输入 URL 也得守住解锁顺序，否则「逐关解锁」只是地图上的装饰。
  let order: Vec<String> = stages
    .iter()
    .map(|s| stage_id(bank.as_str(), s.key))
    .collect();
  if stage_states(&order, &load_stars()).get(at) == Some(&StageState::Locked) {
    // 第一关永远开放，所以锁定的关一定有前一关。
    let prev = at.checked_sub(1).and_then(|i| stages.get(i));
    return Err(DeckError::Locked(prev.map_or("", |s| s.name)));
  }

  let stage = stages[at];
  let pool = stage_pool(&all, key);
  let mut rng = random;
  let questions: Vec<QuestionItem> = pick_stage(&pool, &mut rng)
    .into_iter()
    .filter_map(|i| all.get(i).cloned())
    .collect();
  if questions.is_empty() {
    return Err(DeckError::NotFound);
  }
  Ok(Deck {
    questions,
    meta: Meta {
      enemy: Enemy::Stage(stage.name),
      sprite: monster_sprite(key),
      stage_id: Some(stage_id(bank.as_str(), key)),
      next: next_stage(&stages, key).map(|s| (s.key.to_owned(), s.name)),
    },
  })
}

fn build_revenge(key: Option<&str>, topic: Option<&str>) -> Result<Deck, DeckError> {
  let book = study::load_book();
  let now = now_ms();
  let top_key = |r: &MistakeRecord| r.question.p_code().and_then(top_of).map(|t| t.key);

  let mut records: Vec<MistakeRecord> = match key {
    Some(k) => book
      .records
      .iter()
      .filter(|r| r.key == k)
      .cloned()
      .collect(),
    None => {
      let mut v: Vec<MistakeRecord> = book
        .records
        .iter()
        .filter(|r| topic.is_none_or(|t| crate::rpg::topic_matches(top_key(r), t)))
        .cloned()
        .collect();
      // 到期的排前面（`false < true`，所以取反），同样到期状态再按到期时间：
      // 没有到期怪物时也能凑出一队，不让用户点了「复仇」却发现空空如也。
      v.sort_by_key(|r| (!r.is_due(now), r.due_ms));
      v.truncate(STAGE_SIZE);
      v
    }
  };
  let first = records.first().ok_or(DeckError::Empty)?;
  let sprite = monster_sprite(top_key(first).unwrap_or_default());
  let enemy = if records.len() == 1 {
    Enemy::Single
  } else {
    Enemy::Pack
  };
  Ok(Deck {
    questions: records.drain(..).map(|r| r.question).collect(),
    meta: Meta {
      enemy,
      sprite,
      stage_id: None,
      next: None,
    },
  })
}
