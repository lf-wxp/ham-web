//! 基地的「今日任务」：每日挑战 / 复仇到期怪物 / 推进关卡。

use ham_web_core::daily_challenge::DailyResults;
use ham_web_core::rpg::{XP_PER_CHALLENGE, XP_PER_CORRECT};
use leptos::prelude::*;

use crate::i18n::{t, tf, tp};
use crate::rpg::load_stars;
use crate::study;
use crate::util::{local_today, now_ms, storage};

use super::quest_row::QuestRow;

/// 今日任务栏。数据都是同步读本地存档，进入首页时算一次即可。
#[component]
pub(super) fn QuestBoard() -> impl IntoView {
  let today = local_today();
  let daily = storage::get_json::<DailyResults>("daily-challenge")
    .unwrap_or_default()
    .days
    .get(&today)
    .copied();
  let due = study::load_book().due_count(now_ms());
  let stars = load_stars().total();

  // 没完成的任务数：每日挑战没做、有到期怪物各算一项；「推进关卡」永远可做，不计入。
  let left = usize::from(daily.is_none()) + usize::from(due > 0);

  let daily_hint = match daily {
    Some(r) => tf(
      "home.completed-today",
      &[&r.correct.to_string(), &r.total.to_string()],
    ),
    None => t("home.a-daily-timed-10"),
  };

  view! {
    <section class="space-y-3" aria-labelledby="quest-board-title">
      <div class="flex flex-wrap items-baseline justify-between gap-2">
        <h2 id="quest-board-title" class="pxl-title text-sm">{move || t("rpg.quest-board")}</h2>
        <span class="text-xs text-muted-foreground">
          {move || tp("rpg.quests-left", left, &[&left.to_string()])}
        </span>
      </div>
      <ul class="grid gap-3 md:grid-cols-3">
        <QuestRow
          href="/daily-challenge"
          sprite="node_coin"
          title=Signal::derive(move || t("learning.daily-challenge"))
          hint=Signal::derive(move || daily_hint.clone())
          reward=Signal::derive(move || tf("rpg.reward-xp", &[&XP_PER_CHALLENGE.to_string()]))
          done=daily.is_some()
        />
        <QuestRow
          href="/battle?mode=revenge"
          sprite="node_heart"
          title=Signal::derive(move || t("rpg.revenge-all"))
          hint=Signal::derive(move || {
            if due > 0 {
              tp("rpg.quest-review-count", due, &[&due.to_string()])
            } else {
              t("rpg.quest-review-done")
            }
          })
          reward=Signal::derive(move || tf("rpg.reward-per-answer", &[&XP_PER_CORRECT.to_string()]))
          done=due == 0
        />
        <QuestRow
          href="/map"
          sprite="node_flag"
          title=Signal::derive(move || t("rpg.quest-stage"))
          hint=Signal::derive(move || tp("rpg.quest-stage-hint", stars, &[&stars.to_string()]))
          reward=Signal::derive(move || tf("rpg.reward-per-answer", &[&XP_PER_CORRECT.to_string()]))
          done=false
        />
      </ul>
    </section>
  }
}
