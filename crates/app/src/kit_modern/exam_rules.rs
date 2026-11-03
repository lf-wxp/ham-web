//! 现代风考场规则演练：两列对照表（真实考场 / 本站仿真）+ 考前准备链接。
//! 表格语义保留（`<table>`），外观换成现代描边与行分隔。

use leptos::prelude::*;

use crate::i18n::t;

use super::dialog::ModernDialog;

#[component]
pub fn ModernExamRules(open: RwSignal<bool>, on_confirm: Callback<()>) -> impl IntoView {
  view! {
    <ModernDialog open=open title=Signal::derive(move || t("exam.rules-title"))>
      <p class="md-muted md-sm">{move || t("exam.rules-hint")}</p>

      <table class="md-compare">
        <thead>
          <tr>
            <th>{move || t("exam.rules-real-col")}</th>
            <th>{move || t("exam.rules-sim-col")}</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td class="md-muted">{move || t("exam.rules-real-time")}</td>
            <td>{move || t("exam.rules-sim-time")}</td>
          </tr>
          <tr>
            <td class="md-muted">{move || t("exam.rules-real-leave")}</td>
            <td>{move || t("exam.rules-sim-leave")}</td>
          </tr>
          <tr>
            <td class="md-muted">{move || t("exam.rules-real-submit")}</td>
            <td>{move || t("exam.rules-sim-submit")}</td>
          </tr>
        </tbody>
      </table>

      <div class="flex flex-col gap-1">
        <span class="md-setting-title">{move || t("exam.rules-prep")}</span>
        <a class="md-link-inline" href="/photo-processor">{move || t("exam.rules-prep-photo")}</a>
        <a class="md-link-inline" href="/practice">{move || t("exam.rules-prep-practice")}</a>
      </div>

      <div class="md-dialog-actions">
        <a class="md-btn md-btn-quiet" href="/practice">{move || t("exam.rules-cancel")}</a>
        <button
          type="button"
          class="md-btn md-btn-accent"
          on:click=move |_| {
            on_confirm.run(());
            open.set(false);
          }
        >
          {move || t("exam.rules-start")}
        </button>
      </div>
    </ModernDialog>
  }
}
