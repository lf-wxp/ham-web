use leptos::prelude::*;

/// 模态内部的反馈公告；视觉 Toast 仍由根组件统一播放。
#[component]
pub(super) fn ToastLiveRegion() -> impl IntoView {
  view! {
    <div class="sr-only" aria-live="polite" aria-atomic="true" data-slot="toast-live-region">
      {crate::ui::toast::current_message}
    </div>
  }
}
