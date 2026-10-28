use ham_web_core::bands::Usage;
use leptos::prelude::*;

use super::usage_class;

#[component]
pub(super) fn UsageBadge(usage: Usage) -> impl IntoView {
  view! {
    <span class=format!(
      "inline-flex whitespace-nowrap px-2 py-0.5 text-xs {}",
      usage_class(usage),
    )>{move || crate::data::core_text_tracked(usage.key())}</span>
  }
}
