//! 练习页底部操作栏：题库 / 模式 / 进度统计与上一题 / 下一题（桌面与移动端）。

use ham_web_core::Bank;
use ham_web_core::practice::PracticeOrder;
use leptos::prelude::*;

use crate::components::common::BottomBar;
use crate::i18n::{bank_class, t};
use crate::ui::{Size, Variant, button_class};

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
          {move || t("题库")} " " {move || bank_class(bank.get().as_str())} " · " {move || t(order.get().label())} " · "
          {move || t("进度")} " " {move || index.get() + 1} " / " {move || len.get()}
        }
      }
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
          <button class=button_class(Variant::Default, Size::Default, PRESS) disabled=move || at_end.get() on:click=move |_| on_next.run(())>
            {move || t("下一题")}
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
    />
  }
}
