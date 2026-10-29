use crate::i18n::{t, tf, tp};
use ham_web_core::Bank;
use ham_web_core::categories::{sub_category, top_pages};
use ham_web_core::explanation_index::ExplanationIndex;
use ham_web_core::mistake_book::{MistakeRecord, WRONG_CAUSES};
use leptos::prelude::*;

use crate::components::common::ExplanationCard;
use crate::data;
use crate::ui::ChipToggle;

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// 一道错题视图。
///
/// `fresh` 是题库当前版本的解析索引：快照里的解析是答错当时的，展示时经它换成最新的，
/// 索引到达只换卡片里的文字，已展开的卡片不会收起。
#[component]
pub(super) fn MistakeCard(
  record: MistakeRecord,
  now_ms: i64,
  fresh: ReadSignal<ExplanationIndex>,
  on_remove: Callback<String>,
) -> impl IntoView {
  let q = &record.question;
  let j = q.j_code().map_or_else(|| "—".to_owned(), str::to_owned);
  let sub = q.p_code().and_then(sub_category);
  let topic = sub.map_or("未分类", |s| s.name);
  // 考点对应的一级分类专题页（取首个），用于点击直达补知识点。
  let topic_href = sub
    .and_then(|s| top_pages(s.top).first())
    .map(|&(href, _)| href);
  // 该一级分类的正确率（用于展示薄弱程度）。
  let topic_rate = sub.and_then(|s| {
    crate::study::load_stats()
      .total
      .categories
      .get(s.top)
      .and_then(|t| t.rate())
      .map(|r| (r * 100.0).round() as u32)
  });
  // 同类题再练：按二级分类（考点）筛选该题库下的同类题目。
  let same_href = sub.map(|s| format!("/browse?bank={}&sub={}", Bank::of_id(q.id_str()), s.name));
  let due = if record.is_due(now_ms) {
    t("learning.due-for-review")
  } else {
    let days = ((record.due_ms - now_ms) as f64 / DAY_MS as f64).ceil() as i64;
    tp("learning.review-in-days", days, &[&days.to_string()])
  };
  // 掌握度：难度系数越小，间隔增长越慢、越难掌握。
  let difficulty = if record.ease < 1.8 {
    (t("exam.hard"), "text-hp-text")
  } else if record.ease < 2.3 {
    (t("exam.medium"), "text-gold-text")
  } else {
    (t("exam.easy"), "text-win-text")
  };
  let my_answer = if record.my_answer.is_empty() {
    t("learning.marked-don-t-know")
  } else {
    record.my_answer.join("、")
  };
  let key = record.key.clone();
  let explain = data::snapshot_explanation(fresh, record.question.clone());
  let cause = RwSignal::new(record.cause.clone());
  let set_cause = {
    let key = key.clone();
    move |c: &'static str| {
      cause.set(if c.is_empty() {
        None
      } else {
        Some(c.to_owned())
      });
      crate::study::set_mistake_cause(&key, c);
    }
  };
  view! {
    <div class="pxl-card border bg-card p-4">
      <div class="mb-2 flex items-baseline gap-2">
        <span class="shrink-0 bg-muted px-1.5 py-0.5 font-mono text-xs">{j}</span>
        <p class="whitespace-pre-line text-sm leading-snug">{q.question.clone()}</p>
      </div>
      <div class="space-y-1">
        {q
          .options
          .iter()
          .map(|o| {
            view! {
              <div class="text-xs text-muted-foreground">
                <span class="font-mono">{o.key.clone()}</span> "　" {o.text.clone()}
              </div>
            }
          })
          .collect_view()}
      </div>
      <div class="mt-3 flex flex-wrap items-center gap-x-5 gap-y-1 text-xs">
        <span class="text-hp-text">
          {move || t("learning.my-answer")} <span class="font-mono">{my_answer}</span>
        </span>
        <span class="text-win-text">
          {move || t("exam.correct-answer-prefix")} <span class="font-mono">{q.answer_keys.join("、")}</span>
        </span>
        <span class="text-muted-foreground">
          {move || t("learning.topic")}
          {match topic_href {
            Some(href) => view! {
              <a href=href class="text-foreground underline underline-offset-4">{move || t(topic)}</a>
            }
            .into_any(),
            None => view! { <span class="text-foreground">{move || t(topic)}</span> }.into_any(),
          }}
          {topic_rate.map(|r| view! { <span class="ml-1 tabular-nums">{tf("learning.accuracy-in-parens", &[&r.to_string()])}</span> })}
          {same_href.map(|href| view! {
            <a href=href class="ml-1 text-primary underline underline-offset-4">
              {move || t("exam.practise-similar-link")}
            </a>
          })}
        </span>
      </div>
      <div class="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
        <span>{move || t("learning.wrong")} <span class="text-foreground tabular-nums">{record.wrong_count}</span> {move || t("learning.times")}</span>
        <span>{tf("learning.in-a-row", &[&record.streak.to_string(), &record.target_streak().to_string()])}</span>
        <span>
          {move || t("exam.difficulty")} <span class=difficulty.1>{difficulty.0}</span>
        </span>
        <span>{tp("exam.day-interval", record.interval_days.round() as i64, &[&format!("{:.0}", record.interval_days)])}</span>
        <span>{due}</span>
        <button
          type="button"
          class="ml-auto px-2 py-0.5 transition-colors hover:bg-accent hover:text-foreground"
          on:click=move |_| on_remove.run(key.clone())
        >
          {move || t("learning.remove")}
        </button>
      </div>
      <div class="mt-2 flex flex-wrap items-center gap-1.5 border-t pt-2 text-xs">
        <span class="text-muted-foreground">{move || t("exam.why-wrong")}</span>
        {WRONG_CAUSES
          .iter()
          .map(|&(ck, cn)| {
            let sc = set_cause.clone();
            view! {
              <ChipToggle
                active=Signal::derive(move || cause.get().as_deref() == Some(ck))
                on_change=Callback::new(move |on: bool| sc(if on { ck } else { "" }))
              >
                {view! { {move || t(cn)} }.into_any()}
              </ChipToggle>
            }
          })
          .collect_view()}
      </div>
      // 默认折叠：错题多时列表很长，解析只在想看时展开。
      <div class="mt-3">
        <ExplanationCard explanation=explain />
      </div>
    </div>
  }
}
