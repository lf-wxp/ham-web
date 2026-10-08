use std::time::Duration;

use ham_web_core::koch::farnsworth;
use ham_web_core::morse::MorseChar;
use ham_web_core::phonetic::word_of;
use leptos::prelude::*;

use crate::icons::{Icon, IconKind};
use crate::morse_audio::{morse_symbol_times, play_morse_timed_with};
use crate::morse_settings::use_morse_settings;

use super::{CARD_WPM, morse_display};
use crate::i18n::tf;

#[component]
pub(super) fn MorseCard(entry: &'static MorseChar) -> impl IntoView {
  let settings = use_morse_settings();
  let highlight = RwSignal::new(None::<usize>);
  let play_gen = RwSignal::new(0u32);

  let play_card = move |_| {
    let timing = farnsworth(CARD_WPM, CARD_WPM);
    let times = morse_symbol_times(entry.code, timing);
    let seq = play_gen.get_untracked().wrapping_add(1);
    play_gen.set(seq);
    play_morse_timed_with(entry.code, timing, settings.tone_hz(), settings.volume());
    // 逐符号高亮：按时间线推进
    for (i, &(start, _)) in times.iter().enumerate() {
      set_timeout(
        move || {
          if play_gen.get_untracked() == seq {
            highlight.set(Some(i));
          }
        },
        Duration::from_secs_f64(start),
      );
    }
    let total = times.last().map_or(0.12, |(_, e)| *e);
    set_timeout(
      move || {
        if play_gen.get_untracked() == seq {
          highlight.set(None);
        }
      },
      Duration::from_secs_f64(total.max(0.12)),
    );
  };

  view! {
    <button
      type="button"
      title=tf("common.preview-2", &[(entry.ch), (entry.code)])
      on:click=play_card
      class="group relative flex flex-col items-center gap-1.5 rounded-xl border bg-card p-3 text-center transition-all duration-200 ease-out hover:-translate-y-0.5 hover:border-primary/40 hover:bg-accent/60 hover:shadow-md hover:shadow-primary/10 active:translate-y-0 active:scale-[0.97] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60 focus-visible:ring-offset-2 focus-visible:ring-offset-background"
    >
      <span class="absolute right-2 top-2 text-primary opacity-0 transition-all duration-200 ease-out group-hover:scale-110 group-hover:opacity-100">
        <Icon kind=IconKind::Play class="h-3.5 w-3.5" />
      </span>
      <div class="flex items-baseline gap-1.5">
        <span class="text-lg font-semibold leading-none tabular-nums">{entry.ch}</span>
        {word_of(entry.ch)
          .map(|p| view! { <span class="text-[11px] leading-none text-muted-foreground">{p}</span> })}
      </div>
      <span class="font-mono text-base font-semibold tracking-widest">
        {move || {
          let code = morse_display(entry.code);
          code
            .chars()
            .enumerate()
            .map(|(i, ch)| {
              let active = highlight.get() == Some(i);
              view! {
                <span class=if active { "text-emerald-500 dark:text-emerald-400" } else { "text-primary" }>{ch.to_string()}</span>
              }
            })
            .collect_view()
        }}
      </span>
    </button>
  }
}
