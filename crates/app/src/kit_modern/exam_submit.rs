//! 现代风交卷确认：三项统计 + 取消 / 交卷。
//! 与像素版 `components/exam/exam_submit_confirm_dialog.rs` 同一份入参。

use leptos::prelude::*;

use crate::i18n::t;

use super::dialog::ModernDialog;

#[component]
pub fn ModernExamSubmit(
  open: RwSignal<bool>,
  on_confirm: Callback<()>,
  #[prop(into)] total: Signal<usize>,
  #[prop(into)] answered: Signal<usize>,
  #[prop(into)] flagged: Signal<usize>,
) -> impl IntoView {
  let unanswered = move || total.get().saturating_sub(answered.get());

  view! {
    <ModernDialog open=open title=Signal::derive(move || t("exam.submit-now"))>
      <p class="md-muted md-sm">{move || t("exam.submitting-stops-the-timer")}</p>

      <div class="md-stat-list">
        <div class="md-stat-row">
          <span>{move || t("exam.answered")}</span>
          <span class="md-mono">{move || answered.get()} " / " {move || total.get()}</span>
        </div>
        <div class="md-stat-row">
          <span>{move || t("exam.unanswered")}</span>
          <span class=move || {
            if unanswered() > 0 { "md-mono md-danger-text" } else { "md-mono" }
          }>{unanswered}</span>
        </div>
        <div class="md-stat-row">
          <span>{move || t("exam.flagged")}</span>
          <span class="md-mono">{move || flagged.get()}</span>
        </div>
      </div>

      <div class="md-dialog-actions">
        <button type="button" class="md-btn md-btn-quiet" on:click=move |_| open.set(false)>
          {move || t("exam.cancel")}
        </button>
        <button type="button" class="md-btn md-btn-danger" on:click=move |_| on_confirm.run(())>
          {move || t("exam.submit-confirm")}
        </button>
      </div>
    </ModernDialog>
  }
}
