//! 现代风考试设置：三个开关式胶囊 + 快捷键 + 计分规则说明。
//! 与像素版 `components/exam/exam_settings_dialog.rs` 同一份入参。

use leptos::prelude::*;

use crate::i18n::t;
use crate::shortcuts::SHORTCUT_HELP;

use super::dialog::ModernDialog;
use crate::ui::Switch;

#[component]
pub fn ModernExamSettings(
  open: RwSignal<bool>,
  #[prop(into)] show_explanation: Signal<bool>,
  on_change_show_explanation: Callback<bool>,
  #[prop(into)] weighted: Signal<bool>,
  on_change_weighted: Callback<bool>,
  #[prop(into)] strict: Signal<bool>,
  on_change_strict: Callback<bool>,
) -> impl IntoView {
  view! {
    <ModernDialog open=open title=Signal::derive(move || t("exam.settings"))>
      <p class="md-muted md-sm">{move || t("exam.question-selection-shortcuts-and")}</p>

      <div class="md-setting-row">
        <span class="md-setting-title">{move || t("exam.show-explanations-after-submitting")}</span>
        <Switch
          checked=show_explanation
          on_change=on_change_show_explanation
          aria_label=Signal::derive(move || t("exam.show-explanations-after-submitting"))
        />
      </div>

      <div class="md-setting-row md-setting-row-stack">
        <span class="md-setting-title">{move || t("exam.draw-questions-by-topic")}</span>
        <Switch
          checked=weighted
          on_change=on_change_weighted
          aria_label=Signal::derive(move || t("exam.draw-questions-by-topic"))
        />
      </div>

      <div class="md-setting-row md-setting-row-stack">
        <span class="md-setting-title">{move || t("exam.exam-simulation-full-screen")}</span>
        <Switch
          checked=strict
          on_change=on_change_strict
          aria_label=Signal::derive(move || t("exam.exam-simulation-full-screen"))
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

      <p class="md-muted md-sm md-hairline pt-3">{move || t("exam.exam-rules-class-a")}</p>
    </ModernDialog>
  }
}
