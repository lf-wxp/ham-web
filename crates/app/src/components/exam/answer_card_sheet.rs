use std::collections::HashMap;

use leptos::prelude::*;

use crate::data::Questions;
use crate::i18n::t;
use crate::ui::{Button, Separator, Sheet, Size, Variant, button_class};

use super::answer_card_cell::AnswerCardCell;

/// 答题卡筛选。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AnswerCardFilter {
  #[default]
  All,
  Unanswered,
  Flagged,
}

impl AnswerCardFilter {
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::All => "all",
      Self::Unanswered => "unanswered",
      Self::Flagged => "flagged",
    }
  }

  pub fn parse(s: &str) -> Option<Self> {
    match s {
      "all" => Some(Self::All),
      "unanswered" => Some(Self::Unanswered),
      "flagged" => Some(Self::Flagged),
      _ => None,
    }
  }
}

#[component]
pub fn AnswerCardSheet(
  open: RwSignal<bool>,
  #[prop(into)] questions: Signal<Questions>,
  #[prop(into)] answers: Signal<HashMap<String, Vec<String>>>,
  #[prop(into)] flags: Signal<HashMap<String, bool>>,
  #[prop(into)] finished: Signal<bool>,
  #[prop(into)] filter: Signal<AnswerCardFilter>,
  on_change_filter: Callback<AnswerCardFilter>,
  on_jump: Callback<usize>,
  #[prop(into)] current_index: Signal<usize>,
  /// 机考仿真：不能回看 / 跳题。格子与跳转按钮如实置灰，并在顶部说明原因。
  #[prop(optional, into)]
  jump_locked: Signal<bool>,
) -> impl IntoView {
  let is_answered = move |key: &str| answers.with(|a| a.get(key).is_some_and(|v| !v.is_empty()));
  let is_flagged = move |key: &str| flags.with(|f| f.get(key).copied().unwrap_or(false));
  let answered_count = move || answers.with(|a| a.values().filter(|v| !v.is_empty()).count());
  let flagged_count = move || flags.with(|f| f.values().filter(|v| **v).count());

  let find = move |from: Option<usize>, pred: &dyn Fn(&str) -> bool| -> Option<usize> {
    let start = from.map_or(0, |f| f + 1);
    questions.with(|qs| (start..qs.len()).find(|&i| pred(&qs[i].answer_key(i))))
  };
  let jump = move |target: Option<usize>| {
    if let Some(i) = target {
      on_jump.run(i);
      open.set(false);
    }
  };

  // 刻意的例外：筛选按钮靠切换 `variant` 表示当前筛选，而 `Button` 的 `variant` 是静态
  // prop（见 `docs/ui-components.md` 的「常见坑」第 4 条）。同文件的关闭按钮已用组件。
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
    let fin = finished.get();
    let filt = filter.get();
    let locked = jump_locked.get();
    let current = current_index.get();
    questions.with(|qs| {
      qs.iter()
        .enumerate()
        .filter_map(|(i, q)| {
          let key = q.answer_key(i);
          let user = answers.with(|a| a.get(&key).cloned().unwrap_or_default());
          let answered = !user.is_empty();
          let flagged = is_flagged(&key);
          let visible = match filt {
            AnswerCardFilter::All => true,
            AnswerCardFilter::Unanswered => !answered,
            AnswerCardFilter::Flagged => flagged,
          };
          if !visible {
            return None;
          }
          Some(view! {
            <AnswerCardCell
              number=i + 1
              answered=answered
              current=i == current
              flagged=flagged
              verdict=fin.then(|| q.is_answer_correct(&user))
              locked=locked
              on_pick=Callback::new(move |()| {
                on_jump.run(i);
                open.set(false);
              })
            />
          })
        })
        .collect_view()
    })
  };

  view! {
    <Sheet open=open>
      <div data-slot="sheet-header" class="flex flex-col gap-1.5 p-4">
        <h2 data-slot="sheet-title" class="text-foreground">{move || t("exam.answer-card")}</h2>
      </div>
      <div class="px-4 space-y-3 overflow-y-auto overscroll-contain pt-1">
        <div class="flex items-center justify-between gap-3 text-sm">
          <div class="text-muted-foreground">
            {move || t("exam.answered-summary")} " " {answered_count} " / " {move || questions.with(|q| q.len())} "｜" {move || t("exam.flag")} " " {flagged_count}
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
              on_click=Callback::new(move |_| jump(find(None, &|k| !is_answered(k))))
            >
              {move || t("exam.first-unanswered")}
            </Button>
            <Button
              variant=Variant::Outline
              size=Size::Sm
              disabled=jump_locked
              on_click=Callback::new(move |_| jump(find(Some(current_index.get_untracked()), &|k| !is_answered(k))))
            >
              {move || t("exam.next-unanswered")}
            </Button>
            <Button
              variant=Variant::Outline
              size=Size::Sm
              disabled=jump_locked
              on_click=Callback::new(move |_| jump(find(Some(current_index.get_untracked()), &|k| is_flagged(k))))
            >
              {move || t("exam.next-flagged")}
            </Button>
          </div>
        </div>
        <div class="grid grid-cols-6 gap-2 sm:grid-cols-8">{grid}</div>
        // 图例常驻：以前只有交卷后才出现，作答中那个小黄块的含义要靠猜。
        <ul class="flex flex-wrap items-center gap-x-4 gap-y-1 pt-2 text-xs text-muted-foreground">
          <li class="inline-flex items-center gap-1">
            <span class="inline-block size-4 border-2 border-ink bg-primary"></span>
            {move || t("exam.answered")}
          </li>
          <li class="inline-flex items-center gap-1">
            <span class="inline-block size-4 border-2 border-ink bg-card"></span>
            {move || t("exam.unanswered")}
          </li>
          <li class="inline-flex items-center gap-1">
            <span class="inline-block size-4 border-2 border-b-4 border-ink border-b-gold bg-card"></span>
            {move || t("exam.current-question")}
          </li>
          <li class="inline-flex items-center gap-1">
            <span class="pxl-label inline-flex size-4 items-center justify-center border-2 border-ink bg-accent text-xs text-accent-foreground">"!"</span>
            {move || t("exam.flagged")}
          </li>
          {move || {
            finished
              .get()
              .then(|| {
                view! {
                  <li class="inline-flex items-center gap-1">
                    <span class="inline-flex size-4 items-center justify-center border-2 border-win bg-card text-xs text-win-text">"✓"</span>
                    {move || t("exam.correct-label")}
                  </li>
                  <li class="inline-flex items-center gap-1">
                    <span class="inline-flex size-4 items-center justify-center border-2 border-destructive bg-card text-xs text-hp-text">"✗"</span>
                    {move || t("exam.wrong-label")}
                  </li>
                }
              })
          }}
        </ul>
      </div>
    </Sheet>
  }
}
