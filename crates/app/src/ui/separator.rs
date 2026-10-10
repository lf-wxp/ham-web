use leptos::prelude::*;

/// 水平分割线。
#[component]
pub fn Separator() -> impl IntoView {
  view! {
    <div
      data-slot="separator"
      role="none"
      data-orientation="horizontal"
      class="bg-border shrink-0 data-[orientation=horizontal]:h-0.5 data-[orientation=horizontal]:w-full data-[orientation=vertical]:h-full data-[orientation=vertical]:w-0.5"
    ></div>
  }
}
