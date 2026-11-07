//! 练习页底部操作栏（现代风）：毛玻璃细条 + 主次两个胶囊按钮。
//!
//! 与像素版 `pages/practice/bottom_bar.rs` 同一份入参；像素版是实体「机身」条 + 硬投影，
//! 这里是半透明玻璃条，滚动时内容从下面透过去。

use ham_web_core::Bank;
use ham_web_core::practice::PracticeOrder;
use leptos::portal::Portal;
use leptos::prelude::*;

use crate::data::kt_tracked;
use crate::i18n::{bank_class, t};

use super::{MIcon, ModernIcon};

#[component]
pub fn ModernPracticeBar(
  #[prop(into)] bank: Signal<Bank>,
  #[prop(into)] order: Signal<PracticeOrder>,
  #[prop(into)] answered: Signal<usize>,
  #[prop(into)] len: Signal<usize>,
  #[prop(into)] at_start: Signal<bool>,
  #[prop(into)] at_end: Signal<bool>,
  /// 当前题是否已作答：答完就在右下角浮出「下一题」，不用再去找侧边箭头。
  #[prop(into)]
  current_answered: Signal<bool>,
  on_prev: Callback<()>,
  on_next: Callback<()>,
) -> impl IntoView {
  view! {
    // 挂在 body 上：`.md-root` 的入场动画带 transform，祖先有 transform 时 fixed 会退化成
    // absolute（表现为「底栏被推到了页面最底部」）。像素版 BottomBar 用 Portal 解决的是同一个问题。
    <Portal>
    // 翻页**不套 View Transition**：题目切换是高频操作，VT 会先冻结一帧拍快照，
    // 实测第二次起会卡到几百毫秒（且连续快速点击会互相打断）。高频交互优先手感。
    //
    // 桌面：卡片两侧的圆形翻页箭头（打破「底部一条固定操作栏」的老交互）；
    // 窄屏：回到底部操作条（这里由 CSS 二选一，逻辑与回调完全共用）。
    <div class="md-rail md-rail-left">
      <button
        type="button"
        class="md-rail-btn"
        aria-label=move || t("exam.previous")
        disabled=move || at_start.get()
        on:click=move |_| on_prev.run(())
      >
        <ModernIcon kind=MIcon::ChevronDown class="md-flip-left" />
      </button>
    </div>
    <div class="md-rail md-rail-right">
      <button
        type="button"
        class="md-rail-btn"
        aria-label=move || t("exam.next")
        disabled=move || at_end.get()
        on:click=move |_| on_next.run(())
      >
        <ModernIcon kind=MIcon::ChevronDown class="md-flip-right" />
      </button>
    </div>

    // 答完题的出口：悬浮的「下一题」。桌面/窄屏都出现，位置避开底部操作条与 dock。
    <div class="md-next-float" data-visible=move || current_answered.get().to_string()>
      <button
        type="button"
        class="md-btn md-btn-accent"
        disabled=move || at_end.get()
        on:click=move |_| on_next.run(())
      >
        {move || t("exam.next")}
        <ModernIcon kind=MIcon::ArrowRight />
      </button>
    </div>

    // `data-fixed-bottom`：自绘窗口滚动条据此让位（`ui/scrollbar.rs`）。
    <div
      class="md-bar md-bar-practice"
      data-fixed-bottom
      role="region"
      aria-label=move || t("exam.bar-label")
    >
      <div class="md-bar-inner">
        <div class="md-bar-stats">
          <span>
            {move || t("exam.bank")} " " {move || bank_class(bank.get().as_str())} " · "
            {move || kt_tracked(order.get().label())}
          </span>
          <span>
            {move || t("exam.answered")} " " {move || answered.get()} " / " {move || len.get()}
          </span>
        </div>
        <div class="md-bar-actions">
          <button
            type="button"
            class="md-btn md-btn-quiet"
            disabled=move || at_start.get()
            on:click=move |_| on_prev.run(())
          >
            {move || t("exam.previous")}
          </button>
          <button
            type="button"
            class="md-btn md-btn-accent"
            disabled=move || at_end.get()
            on:click=move |_| on_next.run(())
          >
            {move || t("exam.next")}
            <ModernIcon kind=MIcon::ArrowRight />
          </button>
        </div>
      </div>
    </div>
    </Portal>
  }
}
