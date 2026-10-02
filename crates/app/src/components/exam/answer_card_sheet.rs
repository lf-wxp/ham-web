use std::collections::HashMap;

use leptos::prelude::*;

use crate::components::common::PreviewableImage;
use crate::data::Questions;
use crate::i18n::{t, tf};
use crate::ui::{BadgeVariant, Separator, Sheet, Size, Variant, badge_class, button_class};

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

  let filter_button = move |f: AnswerCardFilter, label: &'static str| {
    view! {
      <button
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
          let correct = q.is_answer_correct(&user);
          let btn_class = if answered {
            "relative h-9 w-full rounded-md border text-sm font-medium transition-colors bg-primary text-primary-foreground"
          } else {
            "relative h-9 w-full rounded-md border text-sm font-medium transition-colors bg-muted text-foreground"
          };
          let image = q.image().map(|src| {
            let label = q.j_code().map_or_else(|| t("题目附图"), |j| tf("题号 {} 题图", &[(j)]));
            view! { <PreviewableImage src=src.to_owned() alt=label.clone() title=label small_trigger=true /> }
          });
          Some(view! {
            <div class="relative">
              <button
                class=btn_class
                on:click=move |_| {
                  on_jump.run(i);
                  open.set(false);
                }
              >
                <span>{i + 1}</span>
                {flagged
                  .then(|| {
                    view! { <span class="absolute -top-1 -right-1 inline-block size-3 rounded-full bg-yellow-400"></span> }
                  })}
                {fin
                  .then(|| {
                    let extra = if correct {
                      "absolute -bottom-1 -right-1 px-1 py-0 text-[10px] bg-green-600 text-white"
                    } else {
                      "absolute -bottom-1 -right-1 px-1 py-0 text-[10px] bg-red-600 text-white"
                    };
                    view! {
                      <span data-slot="badge" class=badge_class(BadgeVariant::Secondary, extra)>
                        {if correct { "✓" } else { "✗" }}
                      </span>
                    }
                  })}
              </button>
              {image}
            </div>
          })
        })
        .collect_view()
    })
  };

  view! {
    <Sheet open=open>
      <div data-slot="sheet-header" class="flex flex-col gap-1.5 p-4">
        <h2 data-slot="sheet-title" class="text-foreground font-semibold">{move || t("答题卡")}</h2>
      </div>
      <div class="px-4 space-y-3 overflow-y-auto">
        <div class="flex items-center justify-between gap-3 text-sm">
          <div class="text-muted-foreground">
            {move || t("已答")} " " {answered_count} " / " {move || questions.with(|q| q.len())} "｜" {move || t("标记")} " " {flagged_count}
          </div>
          <div class="flex items-center gap-2">
            {filter_button(AnswerCardFilter::All, "全部")}
            {filter_button(AnswerCardFilter::Unanswered, "未答")}
            {filter_button(AnswerCardFilter::Flagged, "标记")}
          </div>
        </div>
        <Separator />
        <div class="flex items-center justify-between flex-wrap gap-2">
          <div class="text-sm text-muted-foreground">{move || t("点击题号跳转")}</div>
          <div class="flex items-center gap-2">
            <button
              class=button_class(Variant::Outline, Size::Sm, "")
              on:click=move |_| jump(find(None, &|k| !is_answered(k)))
            >
              {move || t("首个未答")}
            </button>
            <button
              class=button_class(Variant::Outline, Size::Sm, "")
              on:click=move |_| jump(find(Some(current_index.get_untracked()), &|k| !is_answered(k)))
            >
              {move || t("下一个未答")}
            </button>
            <button
              class=button_class(Variant::Outline, Size::Sm, "")
              on:click=move |_| jump(find(Some(current_index.get_untracked()), &|k| is_flagged(k)))
            >
              {move || t("下一个标记")}
            </button>
          </div>
        </div>
        <div class="grid grid-cols-6 gap-2 sm:grid-cols-8">{grid}</div>
        {move || {
          finished
            .get()
            .then(|| {
              view! {
                <div class="pt-2 text-xs text-muted-foreground flex items-center gap-3">
                  <span class="inline-flex items-center gap-1">
                    <span class="inline-block size-2 rounded-full bg-green-600"></span>
                    {move || t("正确")}
                  </span>
                  <span class="inline-flex items-center gap-1">
                    <span class="inline-block size-2 rounded-full bg-red-600"></span>
                    {move || t("错误")}
                  </span>
                  <span class="inline-flex items-center gap-1">
                    <span class="inline-block size-2 rounded-full bg-yellow-400"></span>
                    {move || t("已标记")}
                  </span>
                </div>
              }
            })
        }}
      </div>
    </Sheet>
  }
}
