//! 练习页底部操作栏：题库 / 模式 / 进度统计与上一题 / 下一题（桌面与移动端）。

use ham_web_core::Bank;
use ham_web_core::practice::PracticeOrder;
use leptos::prelude::*;

use crate::components::common::{BottomBar, KeyHints};
use crate::data::kt_tracked;
use crate::i18n::{bank_class, t};
use crate::ui::{Button, Size, Variant};

#[component]
pub(super) fn PracticeBottomBar(
  #[prop(into)] bank: Signal<Bank>,
  #[prop(into)] order: Signal<PracticeOrder>,
  /// 已作答题数（进度口径以它为准，与顶部进度条一致）。
  #[prop(into)]
  answered: Signal<usize>,
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
          <div>
            {move || t("exam.bank")} " " {move || bank_class(bank.get().as_str())} " · " {move || kt_tracked(order.get().label())} " · "
            {move || t("exam.answered")} " " {move || answered.get()} " / " {move || len.get()}
          </div>
          <KeyHints />
        }
      }
      left=move || {
        view! {
          <Button
            variant=Variant::Secondary
            size=Size::Default
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
              class="w-full"
              disabled=Signal::derive(move || at_start.get())
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
    />
  }
}
