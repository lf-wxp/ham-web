use ham_web_core::glossary::GlossaryEntry;
use leptos::prelude::*;

use crate::i18n::t;

#[component]
pub(super) fn SlangCard(entry: &'static GlossaryEntry) -> impl IntoView {
  let term_class = if entry.term.is_ascii() {
    "font-mono text-base"
  } else {
    "text-base"
  };
  view! {
    <article class="flex flex-col pxl-card border bg-card p-4">
      <div class="flex flex-wrap items-baseline gap-x-2 gap-y-1">
        <h3 class=term_class>{move || crate::data::kt_tracked(entry.term.as_str())}</h3>
        {entry
          .en
          .as_deref()
          .map(|en| view! { <span class="text-xs italic text-muted-foreground">{en}</span> })}
      </div>
      <p class="mt-2 text-sm leading-6 text-muted-foreground">{move || crate::data::kt_tracked(entry.desc.as_str())}</p>
      {(!entry.aliases.is_empty())
        .then(|| {
          view! { <div class="mt-2 text-xs text-muted-foreground">{move || t("knowledge.also-known-as")} {move || entry.aliases.iter().map(|a| crate::data::kt_tracked(a)).collect::<Vec<_>>().join("、")}</div> }
        })}
    </article>
  }
}
