use ham_web_core::glossary::GlossaryEntry;
use leptos::prelude::*;

#[component]
pub(super) fn SlangCard(entry: &'static GlossaryEntry) -> impl IntoView {
  let term_class = if entry.term.is_ascii() {
    "font-mono text-base font-semibold"
  } else {
    "text-base font-semibold"
  };
  view! {
    <article class="flex flex-col rounded-xl border bg-card p-4">
      <div class="flex flex-wrap items-baseline gap-x-2 gap-y-1">
        <h3 class=term_class>{entry.term.as_str()}</h3>
        {entry
          .en
          .as_deref()
          .map(|en| view! { <span class="text-xs italic text-muted-foreground">{en}</span> })}
      </div>
      <p class="mt-2 text-sm leading-6 text-muted-foreground">{entry.desc.as_str()}</p>
      {(!entry.aliases.is_empty())
        .then(|| {
          view! { <div class="mt-2 text-xs text-muted-foreground">"又称：" {entry.aliases.join("、")}</div> }
        })}
    </article>
  }
}
