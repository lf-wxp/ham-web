//! A/B/C 类操作证权限对比。

use ham_web_core::license_classes::{BAND_PERMISSIONS, CLASS_TIPS, CLASS_USAGE};
use ham_web_core::reference::LICENSE_CLASSES;
use leptos::prelude::*;

use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn LicenseClassesPage() -> impl IntoView {
  set_title("A/B/C 类操作证权限");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"A/B/C 类操作证权限"</h1>
            <div class="text-xs text-muted-foreground">"频率范围 · 功率上限 · 适用场景"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <div class="overflow-x-auto rounded-xl border bg-card">
          <table class="w-full min-w-[640px] border-collapse text-sm">
            <thead class="bg-muted/60 text-xs">
              <tr>
                <th class=CELL>"类别"</th>
                <th class=CELL>"频率范围"</th>
                <th class=CELL>"功率上限"</th>
                <th class=CELL>"说明"</th>
              </tr>
            </thead>
            <tbody>
              {LICENSE_CLASSES
                .iter()
                .map(|c| {
                  view! {
                    <tr class="border-t transition-colors hover:bg-muted/40">
                      <td class=format!("{CELL} whitespace-nowrap font-medium")>
                        {format!("{} 类", c.class)}
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"典型设备与场景"</h2>
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"波段权限速查"</h2>
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"备考要点"</h2>
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
      </div>
    </div>
  }
}
