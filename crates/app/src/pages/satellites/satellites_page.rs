use ham_web_core::satellites::{SATELLITE_TIPS, SATELLITES, TRACKING_SOFTWARE};
use leptos::prelude::*;

use crate::components::rotor_control::RotorControl;
use crate::util::set_title;

use super::CELL;
use super::iss_tracker::IssTracker;
use super::pass_predictor::PassPredictor;
use crate::i18n::t;

#[component]
pub fn SatellitesPage() -> impl IntoView {
  set_title("shell.satellites");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("shell.satellites")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("radio.fm-repeaters-and-linear")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <PassPredictor />

        <IssTracker />

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.antenna-rotator")}</h2>
          <p class="px-4 pt-3 text-xs text-muted-foreground">
            {move || t("radio.during-a-pass-connect")}
          </p>
          <div class="p-4"><RotorControl /></div>
        </section>

        // 横向可滚区域必须可聚焦，否则键盘用户滚不动它（axe: scrollable-region-focusable）
        <div
          class="overflow-x-auto rounded-xl border bg-card"
          tabindex="0"
          role="region"
          aria-label=move || t("radio.satellite-frequency-table")
        >
          <table class="w-full min-w-[680px] border-collapse text-sm">
            <thead class="bg-muted/60 text-xs">
              <tr>
                <th class=CELL>{move || t("radio.satellite")}</th>
                <th class=CELL>{move || t("radio.type")}</th>
                <th class=CELL>{move || t("radio.uplink")}</th>
                <th class=CELL>{move || t("radio.downlink")}</th>
                <th class=CELL>{move || t("radio.notes")}</th>
              </tr>
            </thead>
            <tbody>
              {SATELLITES
                .iter()
                .map(|s| {
                  view! {
                    <tr class="border-t transition-colors hover:bg-muted/40">
                      <td class=format!("{CELL} whitespace-nowrap")>
                        <div class="font-medium">{s.name}</div>
                        <div class="font-mono text-xs text-muted-foreground">{s.callsign}</div>
                      </td>
                      <td class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{s.kind}</td>
                      <td class=format!("{CELL} whitespace-nowrap font-mono tabular-nums")>{s.uplink}</td>
                      <td class=format!("{CELL} whitespace-nowrap font-mono tabular-nums")>{s.downlink}</td>
                      <td class=format!("{CELL} text-muted-foreground")>{s.note}</td>
                    </tr>
                  }
                })
                .collect_view()}
            </tbody>
          </table>
        </div>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.operating-tips")}</h2>
          <ul class="space-y-2 p-4">
            {SATELLITE_TIPS
              .iter()
              .map(|tip| {
                view! {
                  <li class="flex gap-2 text-sm text-muted-foreground">
                    <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                    <span>{*tip}</span>
                  </li>
                }
              })
              .collect_view()}
          </ul>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.tracking-and-prediction-software")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {TRACKING_SOFTWARE
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex items-baseline gap-2 rounded-lg px-3 py-2">
                    <span class="shrink-0 text-sm font-medium">{t}</span>
                    <span class="text-sm text-muted-foreground">{d}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>
      </div>
    </div>
  }
}
