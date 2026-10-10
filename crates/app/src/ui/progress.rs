use leptos::prelude::*;

use crate::cn::cn;
use crate::i18n::t;

/// 进度条。
#[component]
pub fn Progress(
  #[prop(into)] value: Signal<i64>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  // `motion-progress`：已填充段上叠一道流光，让「进度在走」这件事在静止画面里也成立。
  let class = cn(&[
    "motion-progress bg-primary/15 relative h-2 w-full overflow-hidden rounded-full",
    &class,
  ]);
  view! {
    <div
      data-slot="progress"
      role="progressbar"
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuenow=move || value.get().clamp(0, 100).to_string()
      aria-label=move || t("common.answering-progress")
      class=class
    >
      <div
        data-slot="progress-indicator"
        class="progress-fill h-full w-full flex-1 rounded-full transition-transform duration-700 ease-[var(--ease-out-expo)]"
        style=move || format!("transform: translateX(-{}%)", 100 - value.get().clamp(0, 100))
      ></div>
    </div>
  }
}
