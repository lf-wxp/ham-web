//! 考后复盘：最近一次模拟考试的逐题对错、分类正确率与错题入口。
//!
//! 交卷弹窗关掉后，原本无法再回看「哪道题错了、正确答案是什么」，这里补上。

use leptos::prelude::*;

use crate::components::common::PageHeader;
use crate::i18n::{t, tf, tp};
use crate::store;
use crate::ui::{ButtonLink, Size, Variant};
use crate::util::set_title;

/// 考后复盘页。
#[component]
pub fn ExamReviewPage() -> impl IntoView {
  set_title("exam.post-exam-review");

  let review = store::load_exam_review();
  // 题库直接展示快照里存的字符串：`bank_class` 需要 `Bank`，而解析失败时回退展示原文更直观。
  let bank = review.as_ref().map(|r| r.bank.clone());

  let body = match review {
    None => view! {
      <div class="rounded-xl border bg-card px-4 py-10 text-center">
        <div class="text-sm text-muted-foreground">{move || t("exam.no-exam-to-review")}</div>
        <ButtonLink
          href="/exam"
          variant=Variant::Default
          size=Size::Default
          class="mt-4 inline-flex"
        >
          {move || t("exam.take-a-mock-exam")}
        </ButtonLink>
      </div>
    }
    .into_any(),
    Some(r) => {
      let total = r.total();
      let correct = r.correct_count();
      let answered = r.answered();
      let finished = crate::util::local_day(r.finished_at_ms as f64);
      let cats = r.by_category();
      let items = r.items.clone();

      view! {
        <div class="space-y-4">
          <section class="rounded-xl border bg-card p-4">
            <div class="flex flex-wrap items-baseline gap-x-5 gap-y-1 text-sm">
              // 空格留在模板里而不写进文案：带尾空格的中文 key 在翻译时极易漏掉，
              // 一旦漏掉就永远匹配不上（`check-i18n` 也查不出来）。
              <span>{move || t("exam.bank")} " " <b>{bank.clone().unwrap_or_default()}</b></span>
              <span>{move || t("exam.correct-4")} " " <b class="tabular-nums">{correct}</b> " / " {total}</span>
              <span>{move || t("exam.answered-2")} " " <b class="tabular-nums">{answered}</b> " / " {total}</span>
              <span class="text-muted-foreground">
                {move || tf("common.submitted-at", &[&finished.clone()])}
              </span>
            </div>
          </section>

          <section class="rounded-xl border bg-card p-4">
            <h2 class="mb-2 text-sm font-semibold">{move || t("exam.performance-by-category-most")}</h2>
            <ul class="space-y-1.5">
              {cats
                .into_iter()
                .map(|c| {
                  let wrong = c.total - c.correct;
                  let pct = (c.correct * 100).checked_div(c.total).unwrap_or(0);
                  view! {
                    <li class="flex items-center gap-2 text-sm">
                      <span class="w-20 shrink-0 font-mono text-xs text-muted-foreground">{c.code.clone()}</span>
                      <div class="h-3 flex-1 overflow-hidden rounded bg-muted">
                        <div class="h-full bg-emerald-500" style=format!("width: {pct}%")></div>
                      </div>
                      <span class="w-28 shrink-0 text-right text-xs tabular-nums text-muted-foreground">
                        {tp(
                          "common.wrong",
                          wrong as u32,
                          &[&c.correct.to_string(), &c.total.to_string(), &wrong.to_string()],
                        )}
                      </span>
                    </li>
                  }
                })
                .collect_view()}
            </ul>
          </section>

          <section class="space-y-3">
            <h2 class="text-sm font-semibold">{move || t("exam.per-question-review")}</h2>
            {items
              .into_iter()
              .enumerate()
              .map(|(i, item)| {
                let ok = item.is_correct();
                let blank = item.is_blank();
                let given = if blank {
                  t("exam.unanswered")
                } else {
                  item.given.join("、")
                };
                let border = if ok {
                  "border-emerald-500/40"
                } else {
                  "border-red-500/40"
                };
                // 快照里的 `id` 是 stable_id，正好用来就地收藏 —— 复盘时最容易
                // 产生的动作就是「把这题收起来回头再看」。
                let item_id = item.id.clone();
                let marked = RwSignal::new(store::is_bookmarked(&item_id));
                view! {
                  <div class=format!("rounded-xl border bg-card p-4 {border}")>
                    <div class="mb-2 flex items-start gap-2">
                      <span class="mt-0.5 shrink-0 rounded bg-muted px-1.5 py-0.5 font-mono text-xs">{i + 1}</span>
                      <p class="flex-1 text-sm font-medium leading-snug">{item.question.clone()}</p>
                      <span class=format!("shrink-0 text-xs {}", if ok { "text-emerald-600 dark:text-emerald-400" } else { "text-red-600 dark:text-red-400" })>
                        {if ok { t("exam.correct-3") } else { t("exam.wrong-2") }}
                      </span>
                    </div>
                    <div class="space-y-1">
                      {item
                        .options
                        .iter()
                        .map(|o| {
                          view! { <div class="text-xs text-muted-foreground">{o.clone()}</div> }
                        })
                        .collect_view()}
                    </div>
                    <div class="mt-2 flex flex-wrap items-center gap-x-4 text-xs">
                      <span>{move || t("exam.correct-answer")} " " <b class="font-mono">{item.answer.join("、")}</b></span>
                      <span>{move || t("exam.your-answer-2")} " " <b class="font-mono">{given.clone()}</b></span>
                      <button
                        type="button"
                        class="ml-auto rounded-full border px-2 py-0.5 text-[11px] text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                        on:click=move |_| marked.set(store::toggle_bookmark(&item_id))
                      >
                        {move || if marked.get() { t("exam.bookmarked") } else { t("exam.bookmark") }}
                      </button>
                    </div>
                    {(!item.explanation.is_empty())
                      .then(|| {
                        view! {
                          <p class="mt-2 border-t pt-2 text-xs text-muted-foreground">{item.explanation.clone()}</p>
                        }
                      })}
                  </div>
                }
              })
              .collect_view()}
          </section>
        </div>
      }
      .into_any()
    }
  };

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader title=t("exam.post-exam-review") subtitle=t("exam.per-question-results-correct") />
      <div class="mx-auto max-w-5xl space-y-4 px-4 py-5">
        <p class="text-sm text-muted-foreground">
          {move || t("exam.review-your-latest-mock")}
        </p>
        {body}
      </div>
    </div>
  }
}
