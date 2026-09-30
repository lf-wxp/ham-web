use ham_web_core::antennas::AntennaType;
use leptos::prelude::*;

#[component]
pub(super) fn AntennaCard(entry: &'static AntennaType) -> impl IntoView {
  view! {
    <article class="flex flex-col rounded-xl border bg-card p-4">
      <svg
        viewBox="0 0 100 60"
        class="h-16 w-full text-primary"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        inner_html=entry.svg
      ></svg>
      <div class="mt-3 flex items-baseline gap-2">
        <h3 class="font-semibold">{entry.name}</h3>
        <span class="font-mono text-xs font-semibold text-muted-foreground">{entry.abbr}</span>
      </div>
      <div class="mt-1.5 flex flex-wrap gap-1.5 text-xs">
        <span class="rounded-md bg-muted px-2 py-0.5 text-muted-foreground">{entry.gain}</span>
        <span class="rounded-md bg-muted px-2 py-0.5 text-muted-foreground">{entry.usage}</span>
      </div>
      <p class="mt-2 text-sm leading-6 text-muted-foreground">{entry.desc}</p>
    </article>
  }
}
