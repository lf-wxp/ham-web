use ham_web_core::answer_card::{AnswerCardFilter, Answers, Flags, Jump};
use leptos::prelude::*;

use crate::data::Questions;
use crate::i18n::t;
use crate::ui::{Button, Separator, Sheet, Size, Variant, button_class};

use super::answer_card_cell::AnswerCardCell;
use super::answer_card_legend::AnswerCardLegend;
use super::answer_card_model::AnswerCardModel;

#[component]
pub fn AnswerCardSheet(
  open: RwSignal<bool>,
  #[prop(into)] questions: Signal<Questions>,
  #[prop(into)] answers: Signal<Answers>,
  #[prop(into)] flags: Signal<Flags>,
  #[prop(into)] finished: Signal<bool>,
  #[prop(into)] filter: Signal<AnswerCardFilter>,
  on_change_filter: Callback<AnswerCardFilter>,
  on_jump: Callback<usize>,
  #[prop(into)] current_index: Signal<usize>,
  /// 机考仿真：不能回看 / 跳题。格子与跳转按钮如实置灰，并在顶部说明原因。
  #[prop(optional, into)]
  jump_locked: Signal<bool>,
) -> impl IntoView {
  if crate::kit::is_modern_at_mount() {
    return view! {
      <crate::kit_modern::ModernAnswerCard
        open=open
        questions=questions
        answers=answers
        flags=flags
        finished=finished
        filter=filter
        on_change_filter=on_change_filter
        on_jump=on_jump
        current_index=current_index
        jump_locked=jump_locked
      />
    }
    .into_any();
  }

  // 取数与判定在 `AnswerCardModel` / core 的 `answer_card`，与现代版共用同一份。
  let model = AnswerCardModel {
    questions,
    answers,
    flags,
    finished,
    filter,
    current_index,
  };
  let jump = move |kind: Jump| {
    if let Some(i) = model.jump_target(kind) {
      on_jump.run(i);
      open.set(false);
    }
  };

  // 刻意的例外：筛选按钮靠切换 `variant` 表示当前筛选，而 `Button` 的 `variant` 是静态
  // prop（见 `docs/ui/pixel.md` 的「常见坑」第 4 条）。同文件的关闭按钮已用组件。
  //
  // `label` 收语义 key（`exam.all` 这类），不是中文原文：以前传的是「全部」，
  // 靠 `catalog::resolve` 的中文反查才命中，属于约定外的一层间接。
  let filter_button = move |f: AnswerCardFilter, label: &'static str| {
    view! {
      <button
        type="button"
        aria-pressed=move || (filter.get() == f).to_string()
        class=move || {
          button_class(if filter.get() == f { Variant::Default } else { Variant::Outline }, Size::Sm, "")
        }
        on:click=move |_| on_change_filter.run(f)
      >
        {move || t(label)}
      </button>
    }
  };

  let grid = move || {
    let locked = jump_locked.get();
    model
      .cells()
      .into_iter()
      .map(|cell| {
        let index = cell.index;
        view! {
          <AnswerCardCell
            number=cell.number()
            answered=cell.answered
            current=cell.current
            flagged=cell.flagged
            verdict=cell.verdict
            locked=locked
            on_pick=Callback::new(move |()| {
              on_jump.run(index);
              open.set(false);
            })
          />
        }
      })
      .collect_view()
  };

  view! {
    <Sheet open=open>
      <div data-slot="sheet-header" class="flex flex-col gap-1.5 p-4">
        <h2 data-slot="sheet-title" class="text-foreground">{move || t("exam.answer-card")}</h2>
      </div>
      <div class="px-4 space-y-3 overflow-y-auto overscroll-contain pt-1">
        <div class="flex items-center justify-between gap-3 text-sm">
          <div class="text-muted-foreground">
            {move || t("exam.answered-summary")} " " {move || model.answered_count()} " / " {move || model.total()} "｜" {move || t("exam.flag")} " " {move || model.flagged_count()}
          </div>
          <div class="flex items-center gap-2">
            {filter_button(AnswerCardFilter::All, "exam.all")}
            {filter_button(AnswerCardFilter::Unanswered, "exam.unanswered-filter")}
            {filter_button(AnswerCardFilter::Flagged, "exam.flag")}
          </div>
        </div>
        <Separator />
        {move || {
          jump_locked
            .get()
            .then(|| view! { <p class="text-sm text-hp-text" role="note">{move || t("exam.answer-card-locked")}</p> })
        }}
        <div class="flex items-center justify-between flex-wrap gap-2">
          <div class="text-sm text-muted-foreground">{move || t("exam.tap-a-number-to")}</div>
          <div class="flex items-center gap-2">
            <Button
              variant=Variant::Outline
              size=Size::Sm
              disabled=jump_locked
              on_click=Callback::new(move |_| jump(Jump::FirstUnanswered))
            >
              {move || t("exam.first-unanswered")}
            </Button>
            <Button
              variant=Variant::Outline
              size=Size::Sm
              disabled=jump_locked
              on_click=Callback::new(move |_| jump(Jump::NextUnanswered))
            >
              {move || t("exam.next-unanswered")}
            </Button>
            <Button
              variant=Variant::Outline
              size=Size::Sm
              disabled=jump_locked
              on_click=Callback::new(move |_| jump(Jump::NextFlagged))
            >
              {move || t("exam.next-flagged")}
            </Button>
          </div>
        </div>
        <div class="grid grid-cols-6 gap-2 sm:grid-cols-8">{grid}</div>
        <AnswerCardLegend finished=finished />
      </div>
    </Sheet>
  }
  .into_any()
}
