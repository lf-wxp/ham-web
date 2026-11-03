//! 现代风确认框：与像素风 `ui::ConfirmRoot` 同一套服务（`ui::confirm()`）驱动，
//! 只是外观换成现代弹层。语气（危险 / 普通）决定执行按钮的颜色。
//!
//! 刻意不做成「服务 + 组件」两份状态：开合、路由切换收起、连点保护都留在 `ConfirmRoot`，
//! 这里只负责画。

use leptos::prelude::*;

use crate::i18n::t;

use super::dialog::ModernDialog;

#[component]
pub fn ModernConfirm(
  open: RwSignal<bool>,
  #[prop(into)] title: Signal<String>,
  #[prop(into)] description: Signal<String>,
  #[prop(into)] confirm_text: Signal<String>,
  /// 危险动作（删除 / 清空）：执行按钮用红色。
  #[prop(into)]
  danger: Signal<bool>,
  on_confirm: Callback<()>,
  on_cancel: Callback<()>,
) -> impl IntoView {
  view! {
    <ModernDialog open=open title=title>
      {move || {
        (!description.get().is_empty())
          .then(|| {
            view! { <p class="md-muted md-sm md-confirm-desc">{move || description.get()}</p> }
          })
      }}
      <div class="md-dialog-actions">
        <button type="button" class="md-btn md-btn-quiet" on:click=move |_| on_cancel.run(())>
          {move || t("exam.cancel")}
        </button>
        <button
          type="button"
          class=move || if danger.get() { "md-btn md-btn-danger" } else { "md-btn md-btn-accent" }
          on:click=move |_| on_confirm.run(())
        >
          {move || confirm_text.get()}
        </button>
      </div>
    </ModernDialog>
  }
}
