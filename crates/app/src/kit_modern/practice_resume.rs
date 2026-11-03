//! 现代风「继续上次练习」提示。

use leptos::prelude::*;

use crate::i18n::t;

use super::dialog::ModernDialog;
use crate::ui::Switch;

#[component]
pub fn ModernPracticeResume(
  open: RwSignal<bool>,
  no_prompt: RwSignal<bool>,
  on_restart: Callback<()>,
  on_resume: Callback<()>,
) -> impl IntoView {
  view! {
    <ModernDialog open=open title=Signal::derive(move || t("exam.found-a-previous-session"))>
      <p class="md-muted md-sm">{move || t("exam.resume-where-you-left")}</p>

      <div class="md-setting-row">
        <span class="md-setting-title">{move || t("exam.don-t-ask-again")}</span>
        <Switch
          checked=no_prompt
          on_change=Callback::new(move |v: bool| no_prompt.set(v))
          aria_label=Signal::derive(move || t("exam.don-t-ask-again"))
        />
      </div>

      <div class="md-dialog-actions">
        <button type="button" class="md-btn md-btn-quiet" on:click=move |_| on_restart.run(())>
          {move || t("exam.restart")}
        </button>
        <button type="button" class="md-btn md-btn-accent" on:click=move |_| on_resume.run(())>
          {move || t("exam.resume")}
        </button>
      </div>
    </ModernDialog>
  }
}
