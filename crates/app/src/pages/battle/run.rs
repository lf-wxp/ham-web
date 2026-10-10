//! 一局战斗的状态：题目队列、当前回合、血量与特效。
//!
//! 全部是 `RwSignal`，所以 [`Run`] 是 `Copy` 的，可以直接塞进各个闭包与子组件。

use ham_web_core::QuestionItem;
use ham_web_core::mistake_book::RecordOutcome;
use ham_web_core::rpg::{Battle, Outcome, Turn};
use leptos::prelude::*;

use crate::rpg::Profile;

/// 一回合的反馈（答完题后展示，直到玩家点「继续」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Feedback {
  pub(super) correct: bool,
  pub(super) turn: Turn,
  /// 这次作答对错题本的影响（复仇模式用它讲「怪物还剩几击」）。
  pub(super) outcome: Option<RecordOutcome>,
  /// 这回合之后战斗是否已分出胜负。
  pub(super) finished: bool,
}

/// 特效种类。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum HitKind {
  #[default]
  None,
  /// 我方命中：对怪物造成的伤害与获得的经验。
  Strike { damage: u32, xp: u32 },
  /// 我方受击。
  Hurt,
}

/// 特效事件。`seq` 每回合递增：视图靠它判断「又发生了一次」，
/// 重新创建 DOM 节点来重放 CSS 动画（比手动摘加类名可靠）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct HitFx {
  pub(super) seq: u32,
  pub(super) kind: HitKind,
}

#[derive(Clone, Copy)]
pub(super) struct Run {
  /// 题目队列：答错的题会追加到末尾，稍后再打一次。
  pub(super) queue: RwSignal<Vec<QuestionItem>>,
  pub(super) index: RwSignal<usize>,
  pub(super) answer: RwSignal<Vec<String>>,
  pub(super) feedback: RwSignal<Option<Feedback>>,
  pub(super) battle: RwSignal<Battle>,
  pub(super) hit: RwSignal<HitFx>,
  /// 本局获得的经验（只用于结算展示；真正的经验由学习存档推算）。
  pub(super) xp: RwSignal<u32>,
  /// 本局击败的怪物数（错题被「掌握」）。
  pub(super) defeated: RwSignal<u32>,
  /// 开战前的玩家档案，结算时与开战后对比出「升级了没有」。
  pub(super) before: RwSignal<Option<Profile>>,
}

impl Run {
  pub(super) fn new() -> Self {
    Self {
      queue: RwSignal::new(Vec::new()),
      index: RwSignal::new(0),
      answer: RwSignal::new(Vec::new()),
      feedback: RwSignal::new(None),
      battle: RwSignal::new(Battle::new(0)),
      hit: RwSignal::new(HitFx::default()),
      xp: RwSignal::new(0),
      defeated: RwSignal::new(0),
      before: RwSignal::new(None),
    }
  }

  /// 开一局。`crit` 为假时没有暴击（复仇队必须让每只怪物都出场，见 `Battle::crit`）。
  pub(super) fn start(self, questions: Vec<QuestionItem>, crit: bool) {
    let n = u32::try_from(questions.len()).unwrap_or(u32::MAX);
    self.battle.set(if crit {
      Battle::new(n)
    } else {
      Battle::without_crit(n)
    });
    self.queue.set(questions);
    self.index.set(0);
    self.answer.set(Vec::new());
    self.feedback.set(None);
    self.hit.set(HitFx::default());
    self.xp.set(0);
    self.defeated.set(0);
    self.before.set(Some(crate::rpg::load_profile()));
    crate::study::note_question_start();
  }

  /// 当前题（`(回合序号, 题目)`）。会订阅 `index` / `queue`。
  pub(super) fn current(self) -> Option<(usize, QuestionItem)> {
    let i = self.index.get();
    self.queue.with(|q| q.get(i).cloned()).map(|q| (i, q))
  }

  /// 当前题，不订阅信号（给事件回调用）。
  pub(super) fn current_untracked(self) -> Option<QuestionItem> {
    let i = self.index.get_untracked();
    self.queue.with_untracked(|q| q.get(i).cloned())
  }

  /// 出招：判分、记录到错题本 / 统计，并把结果结算到战斗里。
  pub(super) fn attack(self) {
    if self.feedback.get_untracked().is_some() {
      return;
    }
    let answer = self.answer.get_untracked();
    if answer.is_empty() {
      return;
    }
    let i = self.index.get_untracked();
    let Some(q) = self.queue.with_untracked(|qs| qs.get(i).cloned()) else {
      return;
    };
    let correct = q.is_answer_correct(&answer);
    let outcome = crate::study::record_answer(&q, &answer);

    let mut battle = self.battle.get_untracked();
    let turn = battle.answer(correct);
    let finished = battle.outcome() != Outcome::Ongoing;
    self.battle.set(battle);
    self.xp.update(|x| *x += turn.xp);
    if outcome == Some(RecordOutcome::Mastered) {
      self.defeated.update(|d| *d += 1);
    }
    // 答错的题放回队尾：怪物没死，这一题还得再打一次。
    // 队列里剩下的题数始终不少于怪物剩余生命（每答对一题至少扣 1 点），所以不会出现
    // 「题打完了怪物还活着」的死局。
    if !correct && !finished {
      self.queue.update(|qs| qs.push(q));
    }
    self.hit.update(|h| {
      h.seq += 1;
      h.kind = if correct {
        HitKind::Strike {
          damage: turn.damage,
          xp: turn.xp,
        }
      } else {
        HitKind::Hurt
      };
    });
    self.feedback.set(Some(Feedback {
      correct,
      turn,
      outcome,
      finished,
    }));
  }

  /// 进入下一回合；返回 `true` 表示战斗已结束，应该去结算。
  pub(super) fn advance(self) -> bool {
    let Some(fb) = self.feedback.get_untracked() else {
      return false;
    };
    let next = self.index.get_untracked() + 1;
    // 防御：队列耗尽也当结束处理，避免任何意外下卡在一道不存在的题上。
    if fb.finished || next >= self.queue.with_untracked(Vec::len) {
      return true;
    }
    self.index.set(next);
    self.answer.set(Vec::new());
    self.feedback.set(None);
    crate::study::note_question_start();
    false
  }
}
