//! 现代风答题卡抽屉：右侧滑出、筛选胶囊、方格网格、图例。
//!
//! 与像素版 `components/exam/answer_card_sheet.rs` 同一份入参，且**共用同一个取数模型**
//! （`AnswerCardModel`，判定在 core 的 `answer_card`，有单测）：筛选、跳转、计数、格子的
//! 无障碍名都不在这里写第二遍。这里只把「方格 + 描边」换成圆角磁贴与色点。

use ham_web_core::answer_card::{AnswerCardFilter, Answers, Flags, Jump};
use leptos::portal::Portal;
use leptos::prelude::*;

use crate::components::exam::{AnswerCardModel, cell_label};
use crate::data::Questions;
use crate::focus::use_escape_close;
use crate::i18n::{t, tf};

use super::backdrop::Backdrop;
use super::{MIcon, ModernIcon};
use crate::presence::use_presence;

#[component]
pub fn ModernAnswerCard(
  open: RwSignal<bool>,
  #[prop(into)] questions: Signal<Questions>,
  #[prop(into)] answers: Signal<Answers>,
  #[prop(into)] flags: Signal<Flags>,
  #[prop(into)] finished: Signal<bool>,
  #[prop(into)] filter: Signal<AnswerCardFilter>,
  on_change_filter: Callback<AnswerCardFilter>,
  on_jump: Callback<usize>,
  #[prop(into)] current_index: Signal<usize>,
  #[prop(optional, into)] jump_locked: Signal<bool>,
) -> impl IntoView {
  let model = AnswerCardModel {
    questions,
    answers,
    flags,
    finished,
    filter,
    current_index,
  };
  let presence = use_presence(open);
  let backdrop = Backdrop::new(open);
  // 抽屉也是模态：焦点、滚动锁、Esc 三件套一个都不能少（`check-structure` 的 C7.8 守着）。
  crate::focus::use_modal_focus(open);
  crate::overlay::use_scroll_lock(open);
  use_escape_close(open);

  let jump = move |kind: Jump| {
    if let Some(i) = model.jump_target(kind) {
      on_jump.run(i);
      open.set(false);
    }
  };

  let grid = move || {
    let locked = jump_locked.get();
    model
      .cells()
      .into_iter()
      .map(|cell| {
        let index = cell.index;
        let mut class = String::from("md-cell");
        if cell.answered {
          class.push_str(" md-cell-answered");
        }
        if cell.current {
          class.push_str(" md-cell-current");
        }
        if cell.flagged {
          class.push_str(" md-cell-flagged");
        }
        match cell.verdict {
          Some(true) => class.push_str(" md-cell-right"),
          Some(false) => class.push_str(" md-cell-wrong"),
          None => {}
        }
        if locked {
          class.push_str(" md-cell-locked");
        }
        view! {
          <button
            type="button"
            class=class
            aria-current=if cell.current { "true" } else { "false" }
            // 名字里带上标记 / 对错：对错在格子上只有颜色，读屏全靠这个名字（与像素版同一个函数）。
            aria-label=cell_label(
              tf("exam.question-number", &[&cell.number().to_string()]),
              cell.flagged,
              cell.verdict,
            )
            disabled=locked
            on:click=move |_| {
              on_jump.run(index);
              open.set(false);
            }
          >
            <span class="md-cell-no">{cell.number()}</span>
            // 角标是装饰：状态已写进按钮的名字，`!` 不能再混进可见文本（否则名字不再包含可见文本）。
            {cell.flagged.then(|| view! { <span class="md-cell-flag" aria-hidden="true">"!"</span> })}
          </button>
        }
      })
      .collect_view()
  };

  view! {
    {move || {
      presence.visible.get().then(|| {
        view! {
          <Portal>
            <div
              class="md-sheet"
              data-state=presence.state()
              inert=presence.inert()
              aria-hidden=presence.aria_hidden()
              on:pointerdown=backdrop.on_pointerdown()
              on:click=backdrop.on_click()
            >
              <div
                class="md-sheet-panel md-sheet-wide"
                data-modal-root=""
                tabindex="-1"
                role="dialog"
                aria-modal="true"
                aria-label=move || t("exam.answer-card")
              >
                <div class="md-sheet-head">
                  <div class="flex flex-col gap-0.5">
                    <span class="md-h2">{move || t("exam.answer-card")}</span>
                    <span class="md-muted md-sm">
                      {move || t("exam.answered-summary")} " " {move || model.answered_count()} " / "
                      {move || model.total()} " · " {move || t("exam.flag")} " "
                      {move || model.flagged_count()}
                    </span>
                  </div>
                  <button
                    type="button"
                    class="md-icon-btn"
                    aria-label=move || t("shell.close-menu")
                    on:click=move |_| open.set(false)
                  >
                    <ModernIcon kind=MIcon::Close />
                  </button>
                </div>

                <div class="md-sheet-body">
                  <div class="md-chip-row">
                    <button
                      type="button"
                      class="md-chip"
                      aria-pressed=move || (filter.get() == AnswerCardFilter::All).to_string()
                      on:click=move |_| on_change_filter.run(AnswerCardFilter::All)
                    >
                      {move || t("exam.all")}
                    </button>
                    <button
                      type="button"
                      class="md-chip"
                      aria-pressed=move || (filter.get() == AnswerCardFilter::Unanswered).to_string()
                      on:click=move |_| on_change_filter.run(AnswerCardFilter::Unanswered)
                    >
                      {move || t("exam.unanswered-filter")}
                    </button>
                    <button
                      type="button"
                      class="md-chip"
                      aria-pressed=move || (filter.get() == AnswerCardFilter::Flagged).to_string()
                      on:click=move |_| on_change_filter.run(AnswerCardFilter::Flagged)
                    >
                      {move || t("exam.flag")}
                    </button>
                  </div>

                  {move || {
                    jump_locked
                      .get()
                      .then(|| view! { <p class="md-warn" role="note">{move || t("exam.answer-card-locked")}</p> })
                  }}

                  <div class="flex flex-wrap items-center gap-2">
                    <button
                      type="button"
                      class="md-chip"
                      disabled=move || jump_locked.get()
                      on:click=move |_| jump(Jump::FirstUnanswered)
                    >
                      {move || t("exam.first-unanswered")}
                    </button>
                    <button
                      type="button"
                      class="md-chip"
                      disabled=move || jump_locked.get()
                      on:click=move |_| jump(Jump::NextUnanswered)
                    >
                      {move || t("exam.next-unanswered")}
                    </button>
                    <button
                      type="button"
                      class="md-chip"
                      disabled=move || jump_locked.get()
                      on:click=move |_| jump(Jump::NextFlagged)
                    >
                      {move || t("exam.next-flagged")}
                    </button>
                  </div>

                  <div class="md-cell-grid">{grid}</div>

                  <ul class="md-legend">
                    <li><span class="md-dot md-dot-answered"></span>{move || t("exam.answered")}</li>
                    <li><span class="md-dot"></span>{move || t("exam.unanswered")}</li>
                    <li><span class="md-dot md-dot-current"></span>{move || t("exam.current-question")}</li>
                    <li><span class="md-dot md-dot-flag">"!"</span>{move || t("exam.flagged")}</li>
                    {move || {
                      finished
                        .get()
                        .then(|| {
                          view! {
                            <li><span class="md-dot md-dot-right">"✓"</span>{move || t("exam.correct-label")}</li>
                            <li><span class="md-dot md-dot-wrong">"✗"</span>{move || t("exam.wrong-label")}</li>
                          }
                        })
                    }}
                  </ul>
                </div>
              </div>
            </div>
          </Portal>
        }
      })
    }}
  }
}
