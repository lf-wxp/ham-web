use ham_web_core::bands::Usage;
use leptos::prelude::*;

use super::usage_class;
use crate::i18n::t;

#[component]
pub(super) fn UsageBadge(usage: Usage) -> impl IntoView {
  view! {
    <span class=format!(
      "inline-flex whitespace-nowrap px-2 py-0.5 text-xs {}",
      usage_class(usage),
    )>{move || t(usage.key())}</span>
  }
}
