use ham_web_core::bands::Band;
use leptos::prelude::*;

use super::usage_badge::UsageBadge;
use super::{notes_view, range_view};
use crate::i18n::t;

#[component]
pub(super) fn BandCard(band: &'static Band, jump: Callback<&'static str>) -> impl IntoView {
  let allocations = if band.allocations.is_empty() {
    view! {
      <div class="mt-3 border-t border-dashed pt-3 text-xs text-muted-foreground">{move || t("knowledge.no-amateur-allocation")}</div>
    }
    .into_any()
  } else {
    let items = band
      .allocations
      .iter()
      .enumerate()
      .map(|(i, a)| {
        let notes = band.remark_for(i);
        view! {
          <li class="flex flex-wrap items-center gap-x-2 gap-y-1 py-2 text-sm">
            {range_view(a)}
            <span class="ml-auto">
              <UsageBadge usage=a.usage />
            </span>
            {(!notes.is_empty())
              .then(|| view! { <div class="w-full text-xs text-muted-foreground">{notes_view(notes, jump)}</div> })}
          </li>
        }
      })
      .collect_view();
    view! { <ul class="mt-3 divide-y border-t border-dashed">{items}</ul> }.into_any()
  };
  view! {
    <article class="rounded-xl border bg-card p-4">
      <div class="flex flex-wrap items-center gap-2">
        <span class="rounded-md bg-muted px-2 py-0.5 font-mono text-xs font-semibold text-foreground">
          {move || t("radio.reference")} " " {band.number}
        </span>
        <h3 class="font-semibold">{band.name}</h3>
        {band
          .microwave
          .then(|| view! { <span class="rounded-full border px-2 py-0.5 text-xs text-muted-foreground">{move || t("knowledge.microwave")}</span> })}
        <span class="ml-auto font-mono text-sm font-semibold">{band.freq_abbr}</span>
      </div>
      <dl class="mt-2 grid grid-cols-2 gap-x-3 gap-y-1 text-xs">
        <div>
          <dt class="text-muted-foreground">{move || t("radio.wavelength-range")}</dt>
          <dd class="font-medium tabular-nums">{band.wavelength}</dd>
        </div>
        <div>
          <dt class="text-muted-foreground">{move || t("knowledge.band")}</dt>
          <dd class="font-medium tabular-nums">{band.freq_name} " · " {band.freq_range}</dd>
        </div>
      </dl>
      {allocations}
    </article>
  }
}
