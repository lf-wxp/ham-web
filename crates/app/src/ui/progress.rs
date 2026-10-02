use leptos::prelude::*;

use crate::cn::cn;
use crate::i18n::t;

/// 进度条。
#[component]
pub fn Progress(
  #[prop(into)] value: Signal<i64>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let class = cn(&[
    "bg-primary/20 relative h-2 w-full overflow-hidden rounded-full",
    &class,
  ]);
  view! {
    <div
      data-slot="progress"
      role="progressbar"
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuenow=move || value.get().to_string()
      aria-label=move || t("作答进度")
      class=class
    >
      <div
        data-slot="progress-indicator"
        class="bg-primary h-full w-full flex-1 transition-all"
        style=move || format!("transform: translateX(-{}%)", 100 - value.get())
      ></div>
    </div>
  }
}
