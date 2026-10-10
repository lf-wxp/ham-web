//! A/B/C 类操作证权限对比。

use ham_web_core::license_classes::{BAND_PERMISSIONS, CLASS_TIPS, CLASS_USAGE};
use ham_web_core::reference::LICENSE_CLASSES;
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::{t, tf};
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn LicenseClassesPage() -> impl IntoView {
  set_title("shell.a-b-c-license");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.a-b-c-license")
        subtitle=move || t("radio.frequency-range-power-limit")
      />

      <PageContainer>
        <div class="overflow-x-auto rounded-xl border bg-card">
          <table class="w-full min-w-[640px] border-collapse text-sm">
            <thead class="bg-muted/60 text-xs">
              <tr>
                <th class=CELL>{move || t("exam.class-2")}</th>
                <th class=CELL>{move || t("exam.frequency-range")}</th>
                <th class=CELL>{move || t("exam.power-limit")}</th>
                <th class=CELL>{move || t("radio.notes")}</th>
              </tr>
            </thead>
            <tbody>
              {LICENSE_CLASSES
                .iter()
                .map(|c| {
                  view! {
                    <tr class="border-t transition-colors hover:bg-muted/40">
                      <td class=format!("{CELL} whitespace-nowrap font-medium")>
                        {tf("learning.class", &[(c.class)])}
                      </td>
                      <td class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{c.freq}</td>
                      <td class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{c.power}</td>
                      <td class=format!("{CELL} text-muted-foreground")>{c.note}</td>
                    </tr>
                  }
                })
                .collect_view()}
            </tbody>
          </table>
        </div>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.typical-equipment-and-scenarios")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {CLASS_USAGE
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.band-privileges-quick-reference")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {BAND_PERMISSIONS
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex items-baseline gap-2 rounded-lg px-3 py-2">
                    <span class="text-sm font-medium">{t}</span>
                    <span class="text-sm text-muted-foreground">{d}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.exam-tips")}</h2>
          <ul class="space-y-2 p-4">
            {CLASS_TIPS
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
