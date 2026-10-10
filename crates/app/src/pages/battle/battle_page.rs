use ham_web_core::rpg::{Outcome, stars_of};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_query_map;
use leptos_router::params::ParamsMap;

use crate::components::common::ExplanationCard;
use crate::components::common::PageContainer;
use crate::components::question_card::QuestionCard;
use crate::i18n::{t, tf};
use crate::pages::{bank_href, use_bank_query, use_no_site_footer};
use crate::shortcuts::{DigitDetail, Shortcuts, digit_answer, use_question_shortcuts};
use crate::ui::{Button, ButtonLink, Size, Variant};
use crate::util::set_title;

use super::battle_actions::BattleActions;
use super::battle_feedback::BattleFeedback;
use super::battle_result::{BattleResult, Settlement};
use super::battle_stage::BattleStage;
use super::deck::{self, DeckError, Enemy, Meta, Mode};
use super::run::Run;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
  Loading,
  Failed(DeckError),
  Fighting,
  Done,
}

/// 从查询串解析玩法：`?stage=<分类 key>` 闯关，`?mode=revenge[&key=][&topic=]` 复仇。
fn parse_mode(q: &ParamsMap) -> Option<Mode> {
  let non_empty = |name: &str| q.get(name).filter(|s| !s.is_empty());
  if let Some(key) = non_empty("stage") {
    return Some(Mode::Stage { key });
  }
  (q.get("mode").as_deref() == Some("revenge")).then(|| Mode::Revenge {
    key: non_empty("key"),
    topic: non_empty("topic"),
  })
}

/// 回合战：答对出拳、答错挨打。
#[component]
pub fn BattlePage() -> impl IntoView {
  set_title("rpg.battle");
  use_no_site_footer();
  let (version, bank) = use_bank_query();
  let query = use_query_map();
  let mode = Memo::new(move |_| query.with(parse_mode));

  let run = Run::new();
  let phase = RwSignal::new(Phase::Loading);
  let meta = RwSignal::new(None::<Meta>);
  let settlement = RwSignal::new(None::<Settlement>);
  // 「再战一次」递增它，让下面的加载 Effect 重跑并重新抽题。
  let attempt = RwSignal::new(0u32);
  let generation = StoredValue::new(0u32);

  Effect::new(move |_| {
    let (m, b, v) = (mode.get(), bank.get(), version.get());
    attempt.track();
    generation.update_value(|g| *g += 1);
    let current = generation.get_value();
    settlement.set(None);
    let Some(m) = m else {
      phase.set(Phase::Failed(DeckError::NoTarget));
      return;
    };
    phase.set(Phase::Loading);
    spawn_local(async move {
      let result = deck::build(&m, v.as_deref(), b).await;
      // 加载期间用户可能已换了目标或又点了一次重试：只认最新一次。
      if generation.try_get_value() != Some(current) {
        return;
      }
      match result {
        Ok(d) => {
          let crit = d.meta.stage_id.is_some();
          meta.set(Some(d.meta));
          // 复仇队不开暴击：每只怪物都要出场一次，暴击会让血条提前打空。
          run.start(d.questions, crit);
          phase.set(Phase::Fighting);
        }
        Err(e) => phase.set(Phase::Failed(e)),
      }
    });
  });

  // 结算：写星级、对比等级。星级只在通关且刷新纪录时落盘，败北不记任何东西。
  let finish = Callback::new(move |()| {
    let battle = run.battle.get_untracked();
    let victory = battle.outcome() == Outcome::Victory;
    let m = meta.get_untracked();
    let stage = m.as_ref().and_then(|m| m.stage_id.clone());
    let revenge = stage.is_none();
    let stars = (!revenge).then(|| stars_of(&battle));
    let new_record = match (&stage, stars) {
      (Some(id), Some(s)) if victory && s > 0 => crate::rpg::record_stage(id, s),
      _ => false,
    };
    let before = run.before.get_untracked().map_or(1, |p| p.level.level);
    let after = crate::rpg::load_profile().level.level;
    settlement.set(Some(Settlement {
      victory,
      stars,
      new_record,
      xp: run.xp.get_untracked(),
      best_combo: battle.best_combo,
      correct: battle.correct,
      wrong: battle.wrong,
      defeated: run.defeated.get_untracked(),
      levels: (before, after.max(before)),
      next: m.and_then(|m| m.next),
      revenge,
    }));
    phase.set(Phase::Done);
  });
  let on_attack = Callback::new(move |()| run.attack());
  let on_continue = Callback::new(move |()| {
    if run.advance() {
      finish.run(());
    }
  });
  let on_retry = Callback::new(move |()| attempt.update(|n| *n += 1));

  use_question_shortcuts(Shortcuts {
    on_prev: Callback::new(|()| {}),
    on_next: Callback::new(move |()| {
      if phase.get_untracked() == Phase::Fighting {
        on_continue.run(());
      }
    }),
    on_digit: Callback::new(move |(n, d): (usize, DigitDetail)| {
      if phase.get_untracked() != Phase::Fighting || run.feedback.with_untracked(Option::is_some) {
        return;
      }
      let Some(q) = run.current_untracked() else {
        return;
      };
      let keys: Vec<String> = q.options.iter().map(|o| o.key.clone()).collect();
      if let Some(next) = digit_answer(
        &keys,
        q.is_multiple(),
        &run.answer.get_untracked(),
        n,
        d.strict,
      ) {
        run.answer.set(next);
      }
    }),
    // 复用「Enter」这个入口：出招，或在反馈出来后继续。
    enter_search: Some((
      Signal::derive(move || phase.get() == Phase::Fighting),
      Callback::new(move |()| {
        if run.feedback.with_untracked(Option::is_some) {
          on_continue.run(());
        } else {
          on_attack.run(());
        }
      }),
    )),
    on_help: None,
  });

  let enemy_name = Signal::derive(move || match meta.with(|m| m.as_ref().map(|m| m.enemy)) {
    Some(Enemy::Stage(name)) => t(name),
    Some(Enemy::Pack) => t("rpg.enemy-pack"),
    Some(Enemy::Single) => t("rpg.enemy-single"),
    None => String::new(),
  });
  let revenge = Memo::new(move |_| matches!(mode.get(), Some(Mode::Revenge { .. })));
  let back_href = Signal::derive(move || {
    if revenge.get() {
      "/bestiary".to_owned()
    } else {
      bank_href("/map", version.get().as_deref(), bank.get())
    }
  });

  let stage = move || {
    let sprite = meta.with(|m| m.as_ref().map_or("mon_noise", |m| m.sprite));
    view! { <BattleStage battle=run.battle enemy_name=enemy_name enemy_sprite=sprite hit=run.hit /> }
  };

  let fighting = move || {
    let revealed = Signal::derive(move || run.feedback.with(Option::is_some));
    view! {
      {stage()}
      {move || {
        run.current()
          .map(|(i, q)| {
            let expl = q.clone();
            view! {
              <QuestionCard
                index=i
                total=run.queue.with(Vec::len)
                question=q
                selected=run.answer
                on_change=Callback::new(move |a: Vec<String>| {
                  if !revealed.get_untracked() {
                    run.answer.set(a);
                  }
                })
                show_answer=revealed
                read_only=revealed
              />
              {move || {
                run.feedback
                  .get()
                  .filter(|fb| !fb.correct)
                  .map(|_| view! { <ExplanationCard question=expl.clone() /> })
              }}
            }
          })
      }}
      <BattleFeedback feedback=run.feedback revenge=revenge.get_untracked() />
      <BattleActions run=run on_attack=on_attack on_continue=on_continue retreat_href=back_href />
    }
  };

  let failed = move |e: DeckError| {
    let message = match e {
      DeckError::NoTarget => t("rpg.no-target"),
      DeckError::Load => t("rpg.load-failed"),
      DeckError::NotFound => t("rpg.stage-not-found"),
      DeckError::Locked(prev) => tf("rpg.stage-locked", &[&t(prev)]),
      DeckError::Empty => t("rpg.no-monsters"),
    };
    view! {
      <section class="pxl-window space-y-4 p-6" role="alert">
        <p>{message}</p>
        <div class="flex flex-wrap gap-3">
          {(e == DeckError::Load)
            .then(|| {
              view! {
                <Button variant=Variant::Default size=Size::Default on_click=on_retry>
                  {move || t("rpg.retry")}
                </Button>
              }
            })}
          <ButtonLink href=back_href variant=Variant::Outline size=Size::Default>
            {move || if revenge.get() { t("rpg.to-bestiary") } else { t("rpg.back-to-map") }}
          </ButtonLink>
        </div>
      </section>
    }
  };

  let done = move || {
    settlement.get().map(|s| {
      view! {
        {stage()}
        <BattleResult
          settlement=s
          bank=bank.get_untracked()
          version=version.get_untracked()
          on_retry=on_retry
        />
      }
    })
  };

  view! {
    <h1 class="sr-only">{move || t("rpg.battle")}</h1>
    <PageContainer class="space-y-4 py-4 pb-24">
      {move || match phase.get() {
        Phase::Loading => {
          view! { <div class="pxl-window p-6" aria-live="polite">{t("rpg.battle-loading")}</div> }
            .into_any()
        }
        Phase::Failed(e) => failed(e).into_any(),
        Phase::Fighting => fighting().into_any(),
        Phase::Done => done().into_any(),
      }}
    </PageContainer>
  }
}
