use ham_web_core::{Bank, QuestionItem};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

use crate::data;
use crate::i18n::{t, tp};
use crate::ui::{
  ButtonLink, CARD_HEADER, Size, Variant, card_class, card_content_class, card_title_class,
};
use crate::util::storage;

/// 学习打卡状态。
#[derive(Serialize, Deserialize, Clone, Default)]
struct CheckinState {
  last_date: String,
  streak: usize,
}

const CHECKIN_KEY: &str = "daily-checkin";

/// 当天日期（YYYYMMDD，UTC）。
fn today_str() -> String {
  let d = js_sys::Date::new_0();
  format!(
    "{:04}{:02}{:02}",
    d.get_utc_full_year() as i32,
    d.get_utc_month() as i32 + 1,
    d.get_utc_date() as i32
  )
}

/// 昨天日期（YYYYMMDD，UTC）。
fn yesterday_str() -> String {
  let d = js_sys::Date::new_0();
  let prev = js_sys::Date::new(&JsValue::from_f64(d.get_time() - 86_400_000.0));
  format!(
    "{:04}{:02}{:02}",
    prev.get_utc_full_year() as i32,
    prev.get_utc_month() as i32 + 1,
    prev.get_utc_date() as i32
  )
}

/// 更新并返回连续打卡天数。
fn update_streak() -> usize {
  let today = today_str();
  let mut state: CheckinState = storage::get_json(CHECKIN_KEY).unwrap_or_default();
  if state.last_date != today {
    state.streak = if state.last_date == yesterday_str() {
      state.streak + 1
    } else {
      1
    };
    state.last_date = today;
  }
  storage::set_json(CHECKIN_KEY, &state);
  state.streak
}

/// 每日一题卡片。
#[component]
pub(super) fn DailyQuestion() -> impl IntoView {
  let question = RwSignal::new(None::<QuestionItem>);
  let streak = RwSignal::new(update_streak());

  spawn_local(async move {
    if let Ok(qs) = data::load_bank(None, Bank::A, false).await
      && !qs.is_empty()
    {
      let idx = today_str().parse::<usize>().unwrap_or(1) % qs.len();
      question.set(qs.get(idx).cloned());
    }
  });

  view! {
    <div data-slot="card" class=card_class("")>
      <div data-slot="card-header" class=CARD_HEADER>
        <div data-slot="card-title" class=card_title_class("flex items-center justify-between")>
          <span>{move || t("home.daily-question")}</span>
          <span class="text-sm font-normal text-muted-foreground">
            {move || {
              let n = streak.get() as u32;
              tp("home.day-streak", n, &[&n.to_string()])
            }}
          </span>
        </div>
      </div>
      <div data-slot="card-content" class=card_content_class("space-y-3")>
        {move || {
          question.get().map(|q| {
            view! {
              <div class="whitespace-pre-line text-sm">{q.question.clone()}</div>
              <div class="space-y-1">
                {q
                  .options
                  .iter()
                  .map(|o| {
                    view! {
                      <div class="text-sm text-muted-foreground">
                        <span class="font-mono">{o.key.clone()}</span> "　" {o.text.clone()}
                      </div>
                    }
                  })
                  .collect_view()}
              </div>
              <ButtonLink
                href="/practice"
                variant=Variant::Outline
                size=Size::Sm
              >
                {move || t("home.practice-this")}
              </ButtonLink>
            }
          })
        }}
      </div>
    </div>
  }
}
