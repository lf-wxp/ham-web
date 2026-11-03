//! 现代风续考提示：一段说明 + 重新开始 / 继续。

use leptos::prelude::*;

use crate::i18n::{t, tf};

use super::dialog::ModernDialog;

#[component]
pub fn ModernExamResume(
  open: RwSignal<bool>,
  #[prop(into)] expires_in_ms: Signal<i64>,
  #[prop(into)] answered: Signal<usize>,
  #[prop(into)] total: Signal<usize>,
  on_resume: Callback<()>,
  on_restart: Callback<()>,
) -> impl IntoView {
  // 与考试倒计时同一个排版函数（`mm:ss`，超过 1 小时才带小时）。
  let remaining = move || ham_web_core::text::format_ms(expires_in_ms.get());

  view! {
    <ModernDialog open=open title=Signal::derive(move || t("exam.resume-exam"))>
      <p class="md-muted md-sm">
        {move || {
          tf(
            "exam.found-an-unfinished-exam",
            &[&answered.get().to_string(), &total.get().to_string(), &remaining()],
          )
        }}
      </p>
      <div class="md-dialog-actions">
        <button type="button" class="md-btn md-btn-quiet" on:click=move |_| on_restart.run(())>
          {move || t("exam.restart")}
        </button>
        <button type="button" class="md-btn md-btn-accent" on:click=move |_| on_resume.run(())>
          {move || t("exam.resume-continue")}
        </button>
      </div>
    </ModernDialog>
  }
}
