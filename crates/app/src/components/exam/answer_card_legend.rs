//! [`AnswerCardLegend`]：答题卡图例（像素风），弹窗与页面内的答题卡共用。

use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::{Led, LedState, LedTone};

/// 图例常驻（不只交卷后才有）：四种灯各说一句，交卷后再补对错两种小牌。
#[component]
pub(super) fn AnswerCardLegend(#[prop(into)] finished: Signal<bool>) -> impl IntoView {
  view! {
    <ul class="flex flex-wrap items-center gap-x-4 gap-y-1 pt-2 text-xs text-muted-foreground">
      <li class="inline-flex items-center gap-1.5">
        <Led tone=LedTone::Rx state=LedState::On small=true />
        {move || t("exam.answered")}
      </li>
      <li class="inline-flex items-center gap-1.5">
        <Led tone=LedTone::Warn state=LedState::On small=true />
        {move || t("exam.flagged")}
      </li>
      <li class="inline-flex items-center gap-1.5">
        <Led tone=LedTone::Info state=LedState::On small=true />
        {move || t("exam.current-question")}
      </li>
      <li class="inline-flex items-center gap-1.5">
        <Led tone=LedTone::Rx state=LedState::Off small=true />
        {move || t("exam.unanswered")}
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
  }
}
