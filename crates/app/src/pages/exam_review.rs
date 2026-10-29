//! 考后复盘：最近一次模拟考试的逐题对错、分类正确率与错题入口。
//!
//! 交卷弹窗关掉后，原本无法再回看「哪道题错了、正确答案是什么」，这里补上。

use ham_web_core::explanation_index::{ExplanationIndex, banks_for};
use leptos::prelude::*;

use crate::components::common::{EmptyState, ExplanationCard, PageContainer, PageHeader};
use crate::data;
use crate::i18n::{t, tf, tp};
use crate::icons::IconKind;
use crate::store;
use crate::ui::{ButtonLink, ChipToggle, Size, Variant, card_title_class};
use crate::util::set_title;

/// 考后复盘页。
#[component]
pub fn ExamReviewPage() -> impl IntoView {
  set_title("exam.post-exam-review");

  let review = store::load_exam_review();
  // 题库直接展示快照里存的字符串：`bank_class` 需要 `Bank`，而解析失败时回退展示原文更直观。
  let bank = review.as_ref().map(|r| r.bank.clone());

  // 题库当前版本的解析索引：复盘快照里的解析是交卷当时的，展示时经它换成题库现在的。
  // 只拉复盘里的题涉及的题库，没有复盘记录就不拉；到之前沿用快照，到了只换卡片里的文字。
  let fresh = RwSignal::new(ExplanationIndex::default());
  if let Some(r) = &review {
    data::spawn_explanation_index(
      banks_for(r.items.iter().map(|i| Some(i.id.as_str()))),
      fresh,
    );
  }

  let body = match review {
    None => view! {
      <EmptyState
        icon=IconKind::ClipboardList
        title=Signal::derive(move || t("exam.no-exam-to-review"))
        actions=ViewFn::from(move || {
          view! {
            <ButtonLink
              href="/exam"
              variant=Variant::Default
              size=Size::Default
              class="mt-2 inline-flex"
            >
              {move || t("exam.take-a-mock-exam")}
            </ButtonLink>
          }
        })
      />
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
          <section class="pxl-card border bg-card p-4">
            <div class="flex flex-wrap items-baseline gap-x-5 gap-y-1 text-sm">
              // 空格留在模板里而不写进文案：带尾空格的中文 key 在翻译时极易漏掉，
              // 一旦漏掉就永远匹配不上（`check-i18n` 也查不出来）。
              <span>{move || t("exam.bank")} " " <b>{bank.clone().unwrap_or_default()}</b></span>
              <span>{move || t("exam.correct-summary")} " " <b class="tabular-nums">{correct}</b> " / " {total}</span>
              <span>{move || t("exam.answered-summary")} " " <b class="tabular-nums">{answered}</b> " / " {total}</span>
              <span class="text-muted-foreground">
                {move || tf("common.submitted-at", &[&finished.clone()])}
              </span>
            </div>
          </section>

          <section class="pxl-card border bg-card p-4">
            <h2 class=card_title_class("mb-2")>{move || t("exam.performance-by-category-most")}</h2>
            <ul class="space-y-1.5">
              {cats
                .into_iter()
                .map(|c| {
                  let wrong = c.total - c.correct;
                  let pct = (c.correct * 100).checked_div(c.total).unwrap_or(0);
                  view! {
                    <li class="flex items-center gap-2 text-sm">
                      <span class="w-20 shrink-0 font-mono text-xs text-muted-foreground">{c.code.clone()}</span>
                      <div class="h-3 flex-1 overflow-hidden bg-muted">
                        <div class="h-full bg-win/70" style=format!("width: {pct}%")></div>
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
            <h2 class=card_title_class("")>{move || t("exam.per-question-review")}</h2>
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
                  "border-win/40"
                } else {
                  "border-destructive/40"
                };
                // 快照里的 `id` 是 stable_id，正好用来就地收藏 —— 复盘时最容易
                // 产生的动作就是「把这题收起来回头再看」。
                let item_id = item.id.clone();
                let marked = RwSignal::new(store::is_bookmarked(&item_id));
                // 复盘快照里的解析是交卷当时的版本，展示时读题库现在的；索引没来之前、或题库里
                // 认不出这道题时用快照的。
                let explanation = data::review_explanation(fresh.read_only(), item.clone());
                view! {
                  <div class=format!("pxl-card border bg-card p-4 {border}")>
                    <div class="mb-2 flex items-start gap-2">
                      <span class="mt-0.5 shrink-0 bg-muted px-1.5 py-0.5 font-mono text-xs">{i + 1}</span>
                      <p class="flex-1 text-sm leading-snug">{item.question.clone()}</p>
                      <span class=format!("shrink-0 text-xs {}", if ok { "text-win-text" } else { "text-hp-text" })>
                        {if ok { t("exam.correct-label") } else { t("exam.wrong-label") }}
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
                      <ChipToggle
                        active=marked
                        on_change=Callback::new(move |_| marked.set(store::toggle_bookmark(&item_id)))
                        class="ml-auto"
                      >
                        {view! {
                          {move || if marked.get() { t("exam.bookmarked") } else { t("exam.bookmark") }}
                        }
                          .into_any()}
                      </ChipToggle>
                    </div>
                    // 回顾页一次铺开几十道题，解析默认折叠，想看哪题再展开。
                    <div class="mt-3">
                      <ExplanationCard explanation=explanation />
                    </div>
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
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader title=move || t("exam.post-exam-review") subtitle=move || t("exam.per-question-results-correct") />
      <PageContainer class="space-y-4">
        <p class="text-sm text-muted-foreground">
          {move || t("exam.review-your-latest-mock")}
        </p>
        {body}
      </PageContainer>
    </div>
  }
}
