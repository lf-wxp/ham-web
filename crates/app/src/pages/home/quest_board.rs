//! 首页「今日值机」面板：每日挑战 / 排查到期干扰 / 推进通联路线，一行一个任务（灯 + 任务 + 进度）。

use ham_web_core::daily_challenge::{DailyResults, current_streak, longest_streak};
use ham_web_core::rpg::{XP_PER_CHALLENGE, XP_PER_CORRECT};
use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::i18n::{t, tf, tp};
use crate::rpg::load_stars;
use crate::study;
use crate::util::{local_today, now_ms, storage};

use super::quest_row::QuestRow;

/// 通联路线的信号格上限：10 站 × 每站 3 格。
const MAX_STARS: u32 = 30;

/// 今日任务栏。数据都是同步读本地存档，进入首页时算一次即可。
#[component]
pub(super) fn QuestBoard() -> impl IntoView {
  let today = local_today();
  // 整份存档都要：今日完成情况来自 `days`，连续天数来自整段历史。
  let results = storage::get_json::<DailyResults>("daily-challenge").unwrap_or_default();
  let daily = results.days.get(&today).copied();
  let streak = current_streak(&results.days, &today);
  let best = longest_streak(&results.days);
  let due = study::load_book().due_count(now_ms());
  let stars = load_stars().total();

  // 没完成的任务数：每日挑战没做、有到期怪物各算一项；「推进关卡」永远可做，不计入。
  let left = usize::from(daily.is_none()) + usize::from(due > 0);

  // 每日挑战只有一个入口（原来首页底部还挂着一张同内容的卡，已合并到这里）：
  // 今日完成情况 + 连续天数都在这一行的说明里，不再分两处说同一件事。
  let daily_hint = move || {
    let base = match daily {
      Some(r) => tf(
        "home.completed-today",
        &[&r.correct.to_string(), &r.total.to_string()],
      ),
      None => t("home.a-daily-timed-10"),
    };
    if streak > 0 {
      format!(
        "{base} · {}",
        tp(
          "home.days-in-a-row",
          streak as u32,
          &[&streak.to_string(), &best.to_string()]
        )
      )
    } else {
      base
    }
  };

  // 右侧进度只写真实数据：每日挑战是今天的「答对 / 总数」，到期干扰是待排查条数，路线是已收集的信号格。
  let daily_count = move || match daily {
    Some(r) => format!("{} / {}", r.correct, r.total),
    None => tf("rpg.reward-xp", &[&XP_PER_CHALLENGE.to_string()]),
  };
  let review_count = move || {
    if due > 0 {
      tp("rpg.quest-review-count", due, &[&due.to_string()])
    } else {
      t("rpg.done-tag")
    }
  };
  let stage_count = move || format!("{stars} / {}", MAX_STARS);

  view! {
    <SectionCard
      title=Signal::derive(move || t("rpg.quest-board"))
      meta=ViewFn::from(move || tp("rpg.quests-left", left, &[&left.to_string()]))
    >
      <ul class="px-4 py-1">
        <QuestRow
          href="/daily-challenge"
          title=Signal::derive(move || t("learning.daily-challenge"))
          hint=Signal::derive(daily_hint)
          count=Signal::derive(daily_count)
          done=daily.is_some()
        />
        <QuestRow
          href="/battle?mode=revenge"
          title=Signal::derive(move || t("rpg.revenge-all"))
          hint=Signal::derive(move || tf("rpg.reward-per-answer", &[&XP_PER_CORRECT.to_string()]))
          count=Signal::derive(review_count)
          done=due == 0
        />
        <QuestRow
          href="/map"
          title=Signal::derive(move || t("rpg.quest-stage"))
          hint=Signal::derive(move || tp("rpg.quest-stage-hint", stars, &[&stars.to_string()]))
          count=Signal::derive(stage_count)
          done=false
        />
      </ul>
    </SectionCard>
  }
}
