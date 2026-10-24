use ham_web_core::phonetic::PhoneticEntry;
use leptos::prelude::*;

use crate::i18n::tf;
use crate::speech::speak_en;
use crate::ui::Tooltip;

#[component]
pub(super) fn PhoneticCard(entry: &'static PhoneticEntry) -> impl IntoView {
  view! {
    <Tooltip class="min-w-0" label=tf("common.read-aloud", &[(entry.word)])>
      <button
        type="button"
        on:click=move |_| speak_en(entry.word)
        class="group flex w-full flex-col items-center gap-1.5 pxl-card border bg-card p-3 text-center transition-colors hover:bg-accent/60"
      >
      <div class="flex items-baseline gap-2">
        <span class="text-2xl leading-none text-primary">{entry.letter}</span>
        <span class="text-sm">{entry.word}</span>
      </div>
      <span class="font-mono text-xs tracking-widest text-muted-foreground">{entry.pronunciation}</span>
      </button>
    </Tooltip>
  }
}
