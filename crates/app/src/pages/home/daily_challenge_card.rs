//! 首页「每日挑战」：今日状态与连续挑战天数。

use ham_web_core::daily_challenge::{DailyResults, current_streak, longest_streak};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::{
  CARD_HEADER, Size, Variant, button_class, card_class, card_content_class, card_title_class,
};
use crate::util::{local_today, storage};

const KEY: &str = "daily-challenge";

/// 首页每日挑战入口卡片：显示今日是否完成与连续挑战天数。
#[component]
pub(super) fn DailyChallengeCard() -> impl IntoView {
  let results = storage::get_json::<DailyResults>(KEY).unwrap_or_default();
  let today = local_today();
  let done = results.days.get(&today).copied();
  let streak = current_streak(&results.days, &today);
  let best = longest_streak(&results.days);

  view! {
    <div data-slot="card" class=card_class("")>
      <div data-slot="card-header" class=CARD_HEADER>
        <div data-slot="card-title" class=card_title_class("flex items-center justify-between")>
          <span>{move || t("每日挑战")}</span>
          {(streak > 0).then(|| view! {
            <span class="text-sm font-normal text-muted-foreground">
              {move || tf("连续 {} 天 · 最长 {} 天", &[&streak.to_string(), &best.to_string()])}
            </span>
          })}
        </div>
      </div>
      <div data-slot="card-content" class=card_content_class("flex flex-wrap items-center gap-3")>
        <p class="mr-auto text-sm text-muted-foreground">
          {match done {
            Some(r) => tf(
              "今日已完成：答对 {} / {}。",
              &[&r.correct.to_string(), &r.total.to_string()],
            ),
            None => t("每天一组 10 题限时闯关，同一天同一组题，交卷计入打卡。"),
          }}
        </p>
        <a href="/daily-challenge" class=button_class(Variant::Default, Size::Sm, "")>
          {if done.is_some() { t("再挑战一次") } else { t("开始挑战") }}
        </a>
      </div>
    </div>
  }
}
