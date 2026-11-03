//! 现代风考试底栏：统计 + 剩余时间、上一题 / 标记 / 答题卡 / 下一题 / 交卷。
//!
//! 入参与像素版 `pages/exam/exam_bottom_bar.rs` 一致；移动端只保留两行按钮，
//! 统计行直接藏掉（窄屏塞不下且信息重复）。

use ham_web_core::ExamRule;
use leptos::portal::Portal;
use leptos::prelude::*;

use crate::i18n::t;

use super::exam_clock::ModernExamClock;
use super::{MIcon, ModernIcon};

#[component]
pub fn ModernExamBar(
  #[prop(into)] answered: Signal<usize>,
  #[prop(into)] total: Signal<usize>,
  #[prop(into)] flagged: Signal<usize>,
  #[prop(into)] is_flagged: Signal<bool>,
  #[prop(into)] finished: Signal<bool>,
  #[prop(into)] at_start: Signal<bool>,
  #[prop(into)] at_end: Signal<bool>,
  #[prop(into)] strict: Signal<bool>,
  #[prop(into)] remaining: Signal<i64>,
  #[prop(into)] rule: Signal<ExamRule>,
  on_prev: Callback<()>,
  on_next: Callback<()>,
  on_toggle_flag: Callback<()>,
  on_open_card: Callback<()>,
  on_submit: Callback<()>,
) -> impl IntoView {
  let prev_disabled = Signal::derive(move || at_start.get() || strict.get());

  view! {
    // 同 `practice_bar`：固定定位的底栏必须挂在 body 上，避开 `.md-root` 的 transform。
    // 它落在 `<main>` 之外，必须自带 landmark（否则 axe 的 region 规则判「内容不在
    // 地标里」：固定在页面上的读数与操作按钮同样要能被跳读）。
    <Portal>
    <div class="md-bar" role="region" aria-label=move || t("exam.bar-label")>
      <div class="md-bar-inner md-bar-exam">
        <div class="md-bar-stats">
          <span>
            {move || t("exam.answered")} " " {move || answered.get()} " / " {move || total.get()}
            " · " {move || t("exam.flag")} " " {move || flagged.get()}
          </span>
          {move || {
            (rule.get().minutes > 0)
              .then(|| view! { <ModernExamClock remaining=remaining rule=rule /> })
          }}
        </div>
        <div class="md-bar-actions">
          <button
            type="button"
            class="md-btn md-btn-quiet md-btn-icon-only"
            disabled=move || prev_disabled.get()
            aria-label=move || t("exam.previous")
            on:click=move |_| on_prev.run(())
          >
            <ModernIcon kind=MIcon::ChevronDown class="md-flip" />
          </button>
          <button
            type="button"
            class="md-btn md-btn-quiet"
            aria-pressed=move || is_flagged.get().to_string()
            on:click=move |_| on_toggle_flag.run(())
          >
            {move || if is_flagged.get() { t("exam.unflag") } else { t("exam.flag") }}
          </button>
          <button
            type="button"
            class="md-btn md-btn-outline"
            on:click=move |_| on_open_card.run(())
          >
            {move || t("exam.answer-card")}
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
          <button
            type="button"
            class="md-btn md-btn-danger"
            disabled=move || finished.get()
            on:click=move |_| on_submit.run(())
          >
            {move || if finished.get() { t("exam.submitted") } else { t("exam.submit") }}
          </button>
        </div>
      </div>
    </div>
    </Portal>
  }
}
