use leptos::prelude::*;

/// 对话框底部按钮区。
#[component]
pub fn DialogFooter(children: Children) -> impl IntoView {
  view! {
    <div data-slot="dialog-footer" class="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end shrink-0">
      {children()}
    </div>
  }
}
