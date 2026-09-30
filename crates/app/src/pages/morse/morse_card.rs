use ham_web_core::morse::MorseChar;
use ham_web_core::phonetic::word_of;
use leptos::prelude::*;

use crate::icons::{Icon, IconKind};
use crate::morse_audio::play_morse;

use super::{CARD_WPM, morse_display};

#[component]
pub(super) fn MorseCard(entry: &'static MorseChar) -> impl IntoView {
  view! {
    <button
      type="button"
      title=format!("试听 {}：{}", entry.ch, entry.code)
      on:click=move |_| play_morse(entry.code, CARD_WPM)
      class="group relative flex flex-col items-center gap-1.5 rounded-xl border bg-card p-3 text-center transition-colors hover:bg-accent/60"
    >
      <span class="absolute right-2 top-2 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100">
        <Icon kind=IconKind::Play class="h-3.5 w-3.5" />
      </span>
      <div class="flex items-baseline gap-1.5">
        <span class="text-lg font-semibold tabular-nums">{entry.ch}</span>
        {word_of(entry.ch)
          .map(|p| view! { <span class="text-xs text-muted-foreground">{p}</span> })}
      </div>
      <span class="font-mono text-base font-semibold tracking-widest text-primary">
        {morse_display(entry.code)}
      </span>
    </button>
  }
}
