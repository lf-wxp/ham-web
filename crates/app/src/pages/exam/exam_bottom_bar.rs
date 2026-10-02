//! 考试页底部操作栏：作答 / 标记统计、上一题 / 下一题、标记、答题卡与交卷。

use leptos::prelude::*;

use crate::components::common::BottomBar;
use crate::i18n::t;
use crate::ui::{Size, Variant, button_class};

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
      t("取消标记")
    } else {
      t("标记")
    }
  };

  view! {
    <BottomBar
      stats=move || view! { {move || t("已作答")} " " {answered} " / " {total} "｜" {move || t("标记")} " " {flagged} }
      left=move || {
        view! {
          <button
            class=button_class(Variant::Secondary, Size::Default, PRESS)
            disabled=move || at_start.get()
            on:click=move |_| on_prev.run(())
          >
            {move || t("上一题")}
          </button>
        }
      }
      right=move || {
        view! {
          <button class=button_class(Variant::Outline, Size::Default, PRESS) on:click=move |_| on_toggle_flag.run(())>
            {flag_label}
          </button>
          <button class=button_class(Variant::Outline, Size::Default, PRESS) on:click=move |_| on_open_card.run(())>
            {move || t("答题卡")}
          </button>
          <button class=button_class(Variant::Default, Size::Default, PRESS) disabled=move || at_end.get() on:click=move |_| on_next.run(())>
            {move || t("下一题")}
          </button>
          <button
            class=button_class(Variant::Destructive, Size::Default, PRESS)
            disabled=move || finished.get()
            on:click=move |_| on_submit.run(())
          >
            {move || if finished.get() { t("已交卷") } else { t("交卷") }}
          </button>
        }
      }
      mobile_top=move || {
        view! {
          <div class="grid grid-cols-2 gap-2">
            <button
              class=button_class(Variant::Secondary, Size::Default, PRESS_FULL)
              disabled=move || at_start.get()
              on:click=move |_| on_prev.run(())
            >
              {move || t("上一题")}
            </button>
            <button
              class=button_class(Variant::Default, Size::Default, PRESS_FULL)
              disabled=move || at_end.get()
              on:click=move |_| on_next.run(())
            >
              {move || t("下一题")}
            </button>
          </div>
        }
      }
      mobile_bottom=ViewFn::from(move || {
        view! {
          <div class="grid grid-cols-3 gap-2 mt-2">
            <button class=button_class(Variant::Outline, Size::Default, PRESS_FULL) on:click=move |_| on_toggle_flag.run(())>
              {flag_label}
            </button>
            <button
              class=button_class(Variant::Outline, Size::Default, PRESS_FULL)
              on:click=move |_| on_open_card.run(())
            >
              {move || t("答题卡")}
            </button>
            <button
              class=button_class(Variant::Destructive, Size::Default, PRESS_FULL)
              disabled=move || finished.get()
              on:click=move |_| on_submit.run(())
            >
              {move || if finished.get() { t("已交卷") } else { t("交卷") }}
            </button>
          </div>
        }
      })
    />
  }
}
