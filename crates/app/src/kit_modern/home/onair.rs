//! 今日开台清单：每日挑战 / 错题复习 / 推进关卡，三条「频道」。
//!
//! 替换原来的滚动专题带（`md-ticker`）：那条带子是纯装饰、不可点、对读屏隐藏，
//! 占的却是一屏里最贵的位置。首页要回答的第一件事是「现在做什么」，
//! 这里改成三行可点清单，数据与像素版 `quest_board` 同源（本地存档，进入页面读一次）。

use ham_web_core::daily_challenge::{DailyResults, current_streak, longest_streak};
use ham_web_core::rpg::{XP_PER_CHALLENGE, XP_PER_CORRECT};
use leptos::prelude::*;

use crate::i18n::{t, tf, tp};
use crate::rpg::load_stars;
use crate::study;
use crate::util::{local_today, now_ms, storage};

use super::onair_row::{OnAirRow, OnAirState};

#[component]
pub(super) fn OnAir() -> impl IntoView {
  let today = local_today();
  let results = storage::get_json::<DailyResults>("daily-challenge").unwrap_or_default();
  let daily = results.days.get(&today).copied();
  let streak = current_streak(&results.days, &today);
  let best = longest_streak(&results.days);
  let due = study::load_book().due_count(now_ms());
  let stars = load_stars().total();

  // 没完成的任务数：每日挑战没做、有到期错题各算一项；「推进关卡」永远可做，不计入。
  let left = usize::from(daily.is_none()) + usize::from(due > 0);
  // 状态先落到变量：`view!` 的属性位置不接 `if … {}` 表达式（宏会在花括号处断句）。
  let daily_state = if daily.is_some() {
    OnAirState::Done
  } else {
    OnAirState::Todo
  };
  let review_state = if due > 0 {
    OnAirState::Todo
  } else {
    OnAirState::Done
  };

  let daily_hint = Signal::derive(move || {
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
          streak,
          &[&streak.to_string(), &best.to_string()]
        )
      )
    } else {
      base
    }
  });

  view! {
    <section aria-labelledby="home-onair-title">
      <div class="md-sect-head">
        <h2 id="home-onair-title" class="md-h2">
          {move || t("home.onair-title")}
        </h2>
        <span class="md-sect-note">
          {move || {
            if left == 0 {
              t("home.onair-all-done")
            } else {
              tp("home.onair-left", left, &[&left.to_string()])
            }
          }}
        </span>
      </div>

      // 用 `<ol>` 而不是 `<ul>`：这里三条是「今天按顺序做完」的清单，编号有意义。
      <ol class="md-onair">
        <OnAirRow
          channel="01"
          href="/daily-challenge"
          title=Signal::derive(move || t("learning.daily-challenge"))
          hint=daily_hint
          meta=Signal::derive(move || tf("rpg.reward-xp", &[&XP_PER_CHALLENGE.to_string()]))
          state_label=Signal::derive(move || {
            if daily.is_some() {
              t("home.onair-state-done")
            } else {
              t("home.onair-state-todo")
            }
          })
          state=daily_state
        />
        <OnAirRow
          channel="02"
          href="/battle?mode=revenge"
          title=Signal::derive(move || t("rpg.revenge-all"))
          hint=Signal::derive(move || {
            if due > 0 {
              tp("rpg.quest-review-count", due, &[&due.to_string()])
            } else {
              t("rpg.quest-review-done")
            }
          })
          meta=Signal::derive(move || {
            tf("rpg.reward-per-answer", &[&XP_PER_CORRECT.to_string()])
          })
          state_label=Signal::derive(move || {
            if due > 0 {
              t("home.onair-state-todo")
            } else {
              t("home.onair-state-done")
            }
          })
          state=review_state
        />
        <OnAirRow
          channel="03"
          href="/map"
          title=Signal::derive(move || t("rpg.quest-stage"))
          hint=Signal::derive(move || tp("rpg.quest-stage-hint", stars, &[&stars.to_string()]))
          meta=Signal::derive(move || {
            tf("rpg.reward-per-answer", &[&XP_PER_CORRECT.to_string()])
          })
          state_label=Signal::derive(move || t("home.onair-state-open"))
          state=OnAirState::Open
        />
      </ol>
    </section>
  }
}
