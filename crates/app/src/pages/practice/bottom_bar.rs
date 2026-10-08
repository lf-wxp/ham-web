//! 练习页底部操作栏：题库 / 模式 / 进度统计与上一题 / 下一题（桌面与移动端）。

use ham_web_core::Bank;
use ham_web_core::practice::PracticeOrder;
use leptos::prelude::*;

use crate::components::common::BottomBar;
use crate::i18n::{bank_class, t};
use crate::ui::{Button, Size, Variant};

const PRESS: &str = "active:scale-[0.98] transition-transform";
const PRESS_FULL: &str = "w-full active:scale-[0.98] transition-transform";

#[component]
pub(super) fn PracticeBottomBar(
  #[prop(into)] bank: Signal<Bank>,
  #[prop(into)] order: Signal<PracticeOrder>,
  #[prop(into)] index: Signal<usize>,
  #[prop(into)] len: Signal<usize>,
  #[prop(into)] at_start: Signal<bool>,
  #[prop(into)] at_end: Signal<bool>,
  on_prev: Callback<()>,
  on_next: Callback<()>,
) -> impl IntoView {
  view! {
    <BottomBar
      stats=move || {
        view! {
          {move || t("exam.bank")} " " {move || bank_class(bank.get().as_str())} " · " {move || t(order.get().label())} " · "
          {move || t("exam.progress")} " " {move || index.get() + 1} " / " {move || len.get()}
        }
      }
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
            variant=Variant::Default
            size=Size::Default
            class=PRESS
            disabled=Signal::derive(move || at_end.get())
            on_click=Callback::new(move |_| on_next.run(()))
          >
            {move || t("exam.next")}
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
    />
  }
}
