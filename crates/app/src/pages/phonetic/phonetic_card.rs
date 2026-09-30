use ham_web_core::phonetic::PhoneticEntry;
use leptos::prelude::*;

use crate::speech::speak_en;

#[component]
pub(super) fn PhoneticCard(entry: &'static PhoneticEntry) -> impl IntoView {
  view! {
    <button
      type="button"
      title=format!("朗读 {}", entry.word)
      on:click=move |_| speak_en(entry.word)
      class="group flex flex-col items-center gap-1.5 rounded-xl border bg-card p-3 text-center transition-colors hover:bg-accent/60"
    >
      <div class="flex items-baseline gap-2">
        <span class="text-2xl font-semibold leading-none text-primary">{entry.letter}</span>
        <span class="text-sm font-medium">{entry.word}</span>
      </div>
      <span class="font-mono text-xs tracking-widest text-muted-foreground">{entry.pronunciation}</span>
    </button>
  }
}
