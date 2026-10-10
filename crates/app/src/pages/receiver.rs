//! 接收机关键指标详解。

use ham_web_core::receiver::{NOISE_BASICS, RECEIVER_METRICS, RECEIVER_TIPS};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn ReceiverPage() -> impl IntoView {
  set_title("shell.receiver-specs");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.receiver-specs")
        subtitle=move || t("knowledge.sensitivity-selectivity-dynamic-range")
      />

      <PageContainer>
        <div class="overflow-x-auto rounded-xl border bg-card">
          <table class="w-full min-w-[640px] border-collapse text-sm">
            <thead class="bg-muted/60 text-xs">
              <tr>
                <th class=CELL>{move || t("knowledge.metrics")}</th>
                <th class=CELL>{move || t("knowledge.meaning")}</th>
                <th class=CELL>{move || t("knowledge.deciding-factors")}</th>
              </tr>
            </thead>
            <tbody>
              {RECEIVER_METRICS
                .iter()
                .map(|&(name, meaning, factor)| {
                  view! {
                    <tr class="border-t transition-colors hover:bg-muted/40">
                      <td class=format!("{CELL} whitespace-nowrap font-medium")>{name}</td>
                      <td class=format!("{CELL} text-muted-foreground")>{meaning}</td>
                      <td class=format!("{CELL} text-muted-foreground")>{factor}</td>
                    </tr>
                  }
                })
                .collect_view()}
            </tbody>
          </table>
        </div>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.noise-and-sensitivity-basics")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {NOISE_BASICS
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex flex-col gap-1 rounded-lg px-3 py-2">
                    <span class="text-sm font-medium">{t}</span>
                    <span class="text-sm text-muted-foreground">{d}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.improving-reception")}</h2>
          <ul class="space-y-2 p-4">
            {RECEIVER_TIPS
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
      </PageContainer>
    </div>
  }
}
