//! 现代风单按钮提示窗（`MessageDialog` 的第二套实现）。

use leptos::prelude::*;

use crate::i18n::t;

use super::dialog::ModernDialog;

#[component]
pub fn ModernMessage(
  open: RwSignal<bool>,
  #[prop(into)] title: Signal<String>,
  #[prop(into)] description: Signal<String>,
  #[prop(into)] confirm_text: Signal<String>,
) -> impl IntoView {
  view! {
    <ModernDialog open=open title=title>
      {move || {
        let d = description.get();
        if d.is_empty() {
          view! { <p class="md-muted md-sm">{move || t("exam.dialog-notice")}</p> }.into_any()
        } else {
          view! { <p class="md-muted md-sm whitespace-pre-line">{d}</p> }.into_any()
        }
      }}
      <div class="md-dialog-actions">
        <button type="button" class="md-btn md-btn-accent" on:click=move |_| open.set(false)>
          {move || confirm_text.get()}
        </button>
      </div>
    </ModernDialog>
  }
}
