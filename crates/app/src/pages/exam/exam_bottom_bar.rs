//! 考试页底部操作栏：作答 / 标记统计、上一题 / 下一题、标记、答题卡与交卷。

use leptos::prelude::*;

use crate::components::common::BottomBar;
use crate::i18n::t;
use crate::ui::{Button, Size, Variant};

const PRESS: &str = "active:scale-[0.98] transition-transform";
const PRESS_FULL: &str = "w-full active:scale-[0.98] transition-transform";

#[component]
pub(super) fn ExamBottomBar(
  #[prop(into)] answered: Signal<usize>,
  #[prop(into)] total: Signal<usize>,
  #[prop(into)] flagged: Signal<usize>,
  #[prop(into)] is_flagged: Signal<bool>,
  #[prop(into)] finished: Signal<bool>,
  #[prop(into)] at_start: Signal<bool>,
  #[prop(into)] at_end: Signal<bool>,
  on_prev: Callback<()>,
  on_next: Callback<()>,
  on_toggle_flag: Callback<()>,
  on_open_card: Callback<()>,
  on_submit: Callback<()>,
) -> impl IntoView {
  let flag_label = move || {
    if is_flagged.get() {
      t("exam.unflag")
    } else {
      t("exam.flag")
    }
  };

  view! {
    <BottomBar
      stats=move || view! { {move || t("exam.answered")} " " {answered} " / " {total} "｜" {move || t("exam.flag")} " " {flagged} }
      left=move || {
        view! {
          <Button
            variant=Variant::Secondary
            size=Size::Default
            class=PRESS
            disabled=Signal::derive(move || at_start.get())
            on_click=Callback::new(move |_| on_prev.run(()))
          >
            {move || t("exam.previous")}
          </Button>
        }
      }
      right=move || {
        view! {
          <Button
            variant=Variant::Outline
            size=Size::Default
            class=PRESS
            on_click=Callback::new(move |_| on_toggle_flag.run(()))
          >
            {flag_label}
          </Button>
          <Button
            variant=Variant::Outline
            size=Size::Default
            class=PRESS
            on_click=Callback::new(move |_| on_open_card.run(()))
          >
            {move || t("exam.answer-card")}
          </Button>
          <Button
            variant=Variant::Default
            size=Size::Default
            class=PRESS
            disabled=Signal::derive(move || at_end.get())
            on_click=Callback::new(move |_| on_next.run(()))
          >
            {move || t("exam.next")}
          </Button>
          <Button
            variant=Variant::Destructive
            size=Size::Default
            class=PRESS
            disabled=Signal::derive(move || finished.get())
            on_click=Callback::new(move |_| on_submit.run(()))
          >
            {move || if finished.get() { t("exam.submitted") } else { t("exam.submit") }}
          </Button>
        }
      }
      mobile_top=move || {
        view! {
          <div class="grid grid-cols-2 gap-2">
            <Button
              variant=Variant::Secondary
              size=Size::Default
              class=PRESS_FULL
              disabled=Signal::derive(move || at_start.get())
              on_click=Callback::new(move |_| on_prev.run(()))
            >
              {move || t("exam.previous")}
            </Button>
            <Button
              variant=Variant::Default
              size=Size::Default
              class=PRESS_FULL
              disabled=Signal::derive(move || at_end.get())
              on_click=Callback::new(move |_| on_next.run(()))
            >
              {move || t("exam.next")}
            </Button>
          </div>
        }
      }
      mobile_bottom=ViewFn::from(move || {
        view! {
          <div class="grid grid-cols-3 gap-2 mt-2">
            <Button
              variant=Variant::Outline
              size=Size::Default
              class=PRESS_FULL
              on_click=Callback::new(move |_| on_toggle_flag.run(()))
            >
              {flag_label}
            </Button>
            <Button
              variant=Variant::Outline
              size=Size::Default
              class=PRESS_FULL
              on_click=Callback::new(move |_| on_open_card.run(()))
            >
              {move || t("exam.answer-card")}
            </Button>
            <Button
              variant=Variant::Destructive
              size=Size::Default
              class=PRESS_FULL
              disabled=Signal::derive(move || finished.get())
              on_click=Callback::new(move |_| on_submit.run(()))
            >
              {move || if finished.get() { t("exam.submitted") } else { t("exam.submit") }}
            </Button>
          </div>
        }
      })
    />
  }
}
