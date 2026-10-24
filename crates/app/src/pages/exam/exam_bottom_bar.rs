//! 考试页底部操作栏：作答 / 标记统计、剩余时间读数、上一题 / 下一题、标记、答题卡与交卷。

use leptos::prelude::*;

use crate::components::common::{BottomBar, KeyHints};
use crate::i18n::t;
use crate::ui::{Button, Size, Variant};
use ham_web_core::ExamRule;

use super::exam_clock::ExamClock;

#[component]
pub(super) fn ExamBottomBar(
  #[prop(into)] answered: Signal<usize>,
  #[prop(into)] total: Signal<usize>,
  #[prop(into)] flagged: Signal<usize>,
  #[prop(into)] is_flagged: Signal<bool>,
  #[prop(into)] finished: Signal<bool>,
  #[prop(into)] at_start: Signal<bool>,
  #[prop(into)] at_end: Signal<bool>,
  /// 机考仿真：不能回看上一题。按钮要如实置灰，而不是看着能点、点了没反应。
  #[prop(into)]
  strict: Signal<bool>,
  /// 剩余毫秒数（固定在底栏：长题 / 看图滚动后，顶部读数早已滚出视口）。
  #[prop(into)]
  remaining: Signal<i64>,
  #[prop(into)] rule: Signal<ExamRule>,
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
  let prev_disabled = Signal::derive(move || at_start.get() || strict.get());

  view! {
    <BottomBar
      stats=move || view! {
        <div>
          {move || t("exam.answered")} " " {answered} " / " {total} "｜" {move || t("exam.flag")} " " {flagged}
          // `minutes == 0` 是不限时的自定义组卷：没有读数可放。
          {move || {
            (rule.get().minutes > 0).then(|| view! {
              "｜" {move || t("exam.remaining")} <ExamClock remaining=remaining rule=rule />
            })
          }}
        </div>
        <KeyHints />
      }
      left=move || {
        view! {
          <Button
            variant=Variant::Secondary
            size=Size::Default
            disabled=prev_disabled
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
            on_click=Callback::new(move |_| on_toggle_flag.run(()))
          >
            {flag_label}
          </Button>
          <Button
            variant=Variant::Outline
            size=Size::Default
            on_click=Callback::new(move |_| on_open_card.run(()))
          >
            {move || t("exam.answer-card")}
          </Button>
          <Button
            variant=Variant::Default
            size=Size::Default
            disabled=Signal::derive(move || at_end.get())
            on_click=Callback::new(move |_| on_next.run(()))
          >
            {move || t("exam.next")}
          </Button>
          <Button
            variant=Variant::Destructive
            size=Size::Default
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
              class="w-full"
              disabled=prev_disabled
              on_click=Callback::new(move |_| on_prev.run(()))
            >
              {move || t("exam.previous")}
            </Button>
            <Button
              variant=Variant::Default
              size=Size::Default
              class="w-full"
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
              class="w-full"
              on_click=Callback::new(move |_| on_toggle_flag.run(()))
            >
              {flag_label}
            </Button>
            <Button
              variant=Variant::Outline
              size=Size::Default
              class="w-full"
              on_click=Callback::new(move |_| on_open_card.run(()))
            >
              {move || t("exam.answer-card")}
            </Button>
            <Button
              variant=Variant::Destructive
              size=Size::Default
              class="w-full"
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
