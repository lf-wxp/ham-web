use leptos::prelude::*;

/// 对话框头部。
#[component]
pub fn DialogHeader(children: Children) -> impl IntoView {
  view! {
    <div data-slot="dialog-header" class="flex flex-col gap-2 text-center sm:text-left">
      {children()}
    </div>
  }
}
