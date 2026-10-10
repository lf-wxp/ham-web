//! 每日挑战：每天一组 10 题限时闯关，同一天抽到同一组题，交卷计分并计入打卡。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use ham_web_core::daily_challenge::{
  CHALLENGE_COUNT, CHALLENGE_MINUTES, DailyResult, DailyResults, current_streak, longest_streak,
  pick_daily,
};
use ham_web_core::{ExamScore, QuestionItem};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;

use crate::components::common::{PageContainer, PageHeader};
use crate::components::question_card::QuestionCard;
use crate::data;
use crate::i18n::{t, tf, tp};
use crate::pages::{use_bank_query, use_no_site_footer};
use crate::ui::{Button, Size, Variant};
use crate::util::{local_today, now_ms, set_title, storage};

const KEY: &str = "daily-challenge";

fn load_results() -> DailyResults {
  storage::get_json(KEY).unwrap_or_default()
}

fn save_results(r: &DailyResults) {
  storage::set_json(KEY, r);
}

#[component]
pub fn DailyChallengePage() -> impl IntoView {
  set_title("learning.daily-challenge");
  use_no_site_footer();
  let (version, bank) = use_bank_query();
  let navigate = use_navigate();

  let today = local_today();
  // `StoredValue` 是 Copy，便于在多个闭包中共享今日日期。
  let today_sv = StoredValue::new(today.clone());
  let questions = RwSignal::new(Arc::<Vec<QuestionItem>>::new(Vec::new()));
  let answers = RwSignal::new(HashMap::<String, Vec<String>>::new());
  let loading = RwSignal::new(true);
  let started = RwSignal::new(false);
  let finished = RwSignal::new(false);
  let deadline = RwSignal::new(0i64);
  let remaining = RwSignal::new(i64::from(CHALLENGE_MINUTES) * 60);
  let results = RwSignal::new(load_results());

  // 加载题库并按今日日期确定性抽题。
  Effect::new(move |_| {
    let (v, b, day) = (version.get(), bank.get(), today_sv.get_value());
    questions.set(Arc::new(Vec::new()));
    answers.set(HashMap::new());
    started.set(false);
    finished.set(false);
    loading.set(true);
    spawn_local(async move {
      if let Ok(qs) = data::load_bank(v.as_deref(), b, true).await {
        let idx = pick_daily(qs.len(), &day, CHALLENGE_COUNT);
        let picked: Vec<QuestionItem> =
          idx.into_iter().filter_map(|i| qs.get(i).cloned()).collect();
        questions.set(Arc::new(picked));
      }
      loading.set(false);
    });
  });

  let submit = Callback::new(move |_| {
    let qs = questions.get_untracked();
    let ans = answers.get_untracked();
    let score = ExamScore::calculate(&qs, |q, pos| ans.get(&q.answer_key(pos)).map(Vec::as_slice));
    // 计入错题本、分类统计与每日打卡（重做也按真实作答累计）。
    let items: Vec<(&QuestionItem, &[String])> = qs
      .iter()
      .enumerate()
      .filter_map(|(pos, q)| {
        let key = q.answer_key(pos);
        ans.get(&key).map(|a| (q, a.as_slice()))
      })
      .collect();
    // 实际作答用时 = 现在 - 开始时间（开始时间由 deadline 反推）。
    let started_at = deadline.get_untracked() - i64::from(CHALLENGE_MINUTES) * 60_000;
    let elapsed = (now_ms() - started_at).max(0) as u64;
    let per = if items.is_empty() {
      0
    } else {
      elapsed / items.len() as u64
    };
    crate::study::record_many(items.into_iter().map(|(q, a)| (q, a, per)));
    results.update(|r| {
      r.record(
        &today_sv.get_value(),
        DailyResult {
          correct: score.correct,
          total: score.total,
        },
      );
    });
    save_results(&results.get_untracked());
    started.set(false);
    finished.set(true);
  });

  // 计时器：每秒刷新剩余时间，超时自动交卷。
  Effect::new(move |_| {
    if !started.get() {
      return;
    }
    let Ok(interval) = set_interval_with_handle(
      move || {
        let left = ((deadline.get_untracked() - now_ms()).max(0) + 999) / 1000;
        remaining.set(left);
        if left <= 0 {
          submit.run(());
        }
      },
      Duration::from_secs(1),
    ) else {
      return;
    };
    on_cleanup(move || interval.clear());
  });

  let today_result = move || results.with(|r| r.days.get(&today_sv.get_value()).copied());
  let cur_streak = move || results.with(|r| current_streak(&r.days, &today_sv.get_value()));
  let best_streak = move || results.with(|r| longest_streak(&r.days));
  let mmss = move || {
    let s = remaining.get();
    format!("{:02}:{:02}", s / 60, s % 60)
  };

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("learning.daily-challenge")
        subtitle=tf("learning.questions-a-day-in", &[&CHALLENGE_COUNT.to_string(), &CHALLENGE_MINUTES.to_string()])
        actions=ViewFn::from(move || {
          view! {
            <div class="flex overflow-hidden rounded-lg border text-xs">
              {["A", "B", "C"]
                .into_iter()
                .map(|b| {
                  let bb = ham_web_core::Bank::from_param(Some(b));
                  let nav = navigate.clone();
                  view! {
                    <button
                      type="button"
                      class=move || if bank.get() == bb { "bg-primary px-3 py-1.5 font-medium text-primary-foreground" } else { "px-3 py-1.5 transition-colors hover:bg-accent" }
                      on:click=move |_| {
                        nav(&format!("/daily-challenge?bank={b}"), leptos_router::NavigateOptions::default());
                      }
                    >
                      {b} " 类"
                    </button>
                  }
                })
                .collect_view()}
            </div>
          }
        })
      />

      <PageContainer class="space-y-4">
        {move || {
          if loading.get() {
            return view! { <div class="rounded-xl border bg-card px-4 py-12 text-center text-sm text-muted-foreground">{move || t("learning.loading-today-s-questions")}</div> }.into_any();
          }
          if !started.get() && !finished.get() {
            let done = today_result();
            return view! {
              <section class="rounded-xl border bg-card px-4 py-10 text-center">
                <div class="text-lg font-semibold">{move || t("learning.today-s-challenge")}</div>
                <div class="mt-2 text-sm text-muted-foreground">
                  {tf("learning.questions-minute-limit-counts", &[&CHALLENGE_COUNT.to_string(), &CHALLENGE_MINUTES.to_string()])}
                </div>
                {done.map(|r| view! {
                  <div class="mt-3 text-sm text-muted-foreground">
                    {tp(
                      "learning.done-today-correct",
                      r.total as u32,
                      &[&r.correct.to_string(), &r.total.to_string(), &r.percent().to_string()],
                    )}
                  </div>
                })}
                {(cur_streak() > 0).then(|| view! {
                  <div class="mt-2 text-xs text-muted-foreground">
                    {tp("learning.challenge-streak-days-best", cur_streak(), &[&cur_streak().to_string(), &best_streak().to_string()])}
                  </div>
                })}
                <Button
                  variant=Variant::Default
                  size=Size::Default
                  class="mt-5"
                  on_click=Callback::new(move |_| {
                                    answers.set(HashMap::new());
                                    finished.set(false);
                                    started.set(true);
                                    deadline.set(now_ms() + i64::from(CHALLENGE_MINUTES) * 60_000);
                                    remaining.set(i64::from(CHALLENGE_MINUTES) * 60);
                                  })
                >
                  {if done.is_some() { t("learning.try-again") } else { t("learning.start-challenge") }}
                </Button>
              </section>
            }.into_any();
          }
          if finished.get() {
            let done = today_result();
            return view! {
              <section class="rounded-xl border bg-card px-4 py-10 text-center">
                <div class="text-lg font-semibold">{move || t("learning.challenge-complete")}</div>
                {done.map(|r| view! {
                  <div class="mt-2 text-sm text-muted-foreground">
                    {tp(
                      "learning.correct-2",
                      r.total as u32,
                      &[&r.correct.to_string(), &r.total.to_string(), &r.percent().to_string()],
                    )}
                  </div>
                })}
                {(cur_streak() > 0).then(|| view! {
                  <div class="mt-2 text-xs text-muted-foreground">
                    {tp("learning.challenge-streak-days-best", cur_streak(), &[&cur_streak().to_string(), &best_streak().to_string()])}
                  </div>
                })}
                <Button
                  variant=Variant::Default
                  size=Size::Default
                  class="mt-5"
                  on_click=Callback::new(move |_| { finished.set(false); })
                >
                  {move || t("learning.view-questions")}
                </Button>
              </section>
            }.into_any();
          }
          // 进行中：题目 + 计时 + 交卷
          let qs = questions.get();
          let total = qs.len();
          let list = qs
            .iter()
            .enumerate()
            .map(|(i, q)| {
              let key = q.answer_key(i);
              let sel_key = key.clone();
              let selected = Signal::derive(move || answers.with(|a| a.get(&sel_key).cloned().unwrap_or_default()));
              let on_change = Callback::new(move |v: Vec<String>| {
                answers.update(|a| {
                  if v.is_empty() { a.remove(&key); } else { a.insert(key.clone(), v); }
                });
              });
              view! {
                <QuestionCard
                  index=i
                  total=total
                  question=q.clone()
                  selected=selected
                  on_change=on_change
                  show_answer=Signal::derive(|| false)
                />
              }
            })
            .collect_view();
          view! {
            <div class="sticky top-0 z-10 rounded-xl border bg-card p-3">
              <div class="flex items-center gap-3">
                <span class="text-xs text-muted-foreground">{move || t("learning.time-left")}</span>
                <span class="font-mono text-lg font-semibold tabular-nums">{mmss}</span>
                <Button
                  variant=Variant::Default
                  size=Size::Sm
                  class="ml-auto"
                  on_click=Callback::new(move |_| submit.run(()))
                >
                  {move || t("exam.submit")}
                </Button>
              </div>
            </div>
            <div class="space-y-4">{list}</div>
          }.into_any()
        }}
      </PageContainer>
    </div>
  }
}
