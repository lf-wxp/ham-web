//! [`AnswerCardPanel`]：铺在考试页里的答题卡（像素风、窄屏；canvas「存储频道 · 答题卡」）。
//!
//! 窄屏不再弹出答题卡：格子直接铺在题卡下面，一眼看全 40 格的状态，点格子跳题。
//! 筛选与「下一个未答」这类跳转留在桌面档的弹窗里（`AnswerCardSheet`）。

use ham_web_core::answer_card::{AnswerCardFilter, Answers, Flags};
use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::data::Questions;
use crate::i18n::t;

use super::answer_card_cell::AnswerCardCell;
use super::answer_card_legend::AnswerCardLegend;
use super::answer_card_model::AnswerCardModel;

#[component]
pub fn AnswerCardPanel(
  #[prop(into)] questions: Signal<Questions>,
  #[prop(into)] answers: Signal<Answers>,
  #[prop(into)] flags: Signal<Flags>,
  #[prop(into)] finished: Signal<bool>,
  #[prop(into)] current_index: Signal<usize>,
  on_jump: Callback<usize>,
  /// 机考仿真：不能回看 / 跳题，格子如实置灰。
  #[prop(optional, into)]
  jump_locked: Signal<bool>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let model = AnswerCardModel {
    questions,
    answers,
    flags,
    finished,
    filter: Signal::stored(AnswerCardFilter::All),
    current_index,
  };
  view! {
    <SectionCard
      title=Signal::derive(move || t("exam.channel-memory"))
      meta=ViewFn::from(move || format!("{} / {}", model.answered_count(), model.total()))
      class=class
    >
      <div class="space-y-2 p-3">
        <div class="grid grid-cols-8 gap-1">
          {move || {
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
                    on_pick=Callback::new(move |()| on_jump.run(index))
                  />
                }
              })
              .collect_view()
          }}
        </div>
        <AnswerCardLegend finished=finished />
      </div>
    </SectionCard>
  }
}
