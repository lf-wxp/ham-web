//! 现代风练习设置：顺序/随机分段、三个开关、计算变体说明、快捷键速查。
//!
//! 与像素版 `components/practice/practice_settings_dialog.rs` 同一份入参；
//! 开关用共享的 `ui::Switch`（它自己按风格换成现代外观），不再另写 `aria-pressed` 胶囊。

use ham_web_core::practice::PracticeOrder;
use leptos::prelude::*;

use crate::i18n::t;
use crate::shortcuts::SHORTCUT_HELP;

use super::dialog::ModernDialog;
use crate::ui::Switch;

#[component]
pub fn ModernPracticeSettings(
  open: RwSignal<bool>,
  #[prop(into)] order: Signal<PracticeOrder>,
  on_change_order: Callback<PracticeOrder>,
  #[prop(into)] show_answer: Signal<bool>,
  on_toggle_show_answer: Callback<bool>,
  #[prop(into)] show_explanation: Signal<bool>,
  on_toggle_show_explanation: Callback<bool>,
  #[prop(into)] variants: Signal<bool>,
  on_toggle_variants: Callback<bool>,
) -> impl IntoView {
  view! {
    <ModernDialog open=open title=Signal::derive(move || t("exam.settings"))>
      <p class="md-muted md-sm">{move || t("exam.order-answer-display-and")}</p>

      <div class="md-setting-row">
        <span class="md-setting-title">{move || t("exam.order-random")}</span>
        <div class="md-chip-row">
          <button
            type="button"
            class="md-chip"
            aria-pressed=move || (order.get() == PracticeOrder::Sequential).to_string()
            on:click=move |_| on_change_order.run(PracticeOrder::Sequential)
          >
            {move || t("exam.sequential")}
          </button>
          <button
            type="button"
            class="md-chip"
            aria-pressed=move || (order.get() == PracticeOrder::Random).to_string()
            on:click=move |_| on_change_order.run(PracticeOrder::Random)
          >
            {move || t("exam.random")}
          </button>
        </div>
      </div>

      <div class="md-setting-row">
        <span class="md-setting-title">{move || t("exam.show-correct-answer")}</span>
        <Switch
          checked=show_answer
          on_change=on_toggle_show_answer
          aria_label=Signal::derive(move || t("exam.show-correct-answer"))
        />
      </div>

      <div class="md-setting-row">
        <span class="md-setting-title">{move || t("exam.show-explanation")}</span>
        <Switch
          checked=show_explanation
          on_change=on_toggle_show_explanation
          aria_label=Signal::derive(move || t("exam.show-explanation"))
        />
      </div>

      <div class="md-setting-row md-setting-row-stack">
        <div class="flex flex-col gap-1">
          <span class="md-setting-title">{move || t("exam.calc-variants")}</span>
          <span class="md-muted md-sm">{move || t("exam.calc-variants-hint")}</span>
        </div>
        <Switch
          checked=variants
          on_change=on_toggle_variants
          aria_label=Signal::derive(move || t("exam.calc-variants"))
        />
      </div>

      <div class="md-kbd-list">
        <span class="md-setting-title">{move || t("exam.shortcuts")}</span>
        {SHORTCUT_HELP
          .iter()
          .map(|(keys, desc)| {
            view! {
              <div class="md-kbd-row">
                <kbd class="md-kbd">{(*keys).to_owned()}</kbd>
                <span class="md-muted">{move || t(desc)}</span>
              </div>
            }
          })
          .collect_view()}
      </div>
    </ModernDialog>
  }
}
