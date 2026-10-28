//! 气象卫星接收：NOAA APT / METEOR LRPT 速查。

use ham_web_core::weather_sat::{APT_GUIDE, WEATHER_CONCEPTS, WEATHER_SATS, WEATHER_TIPS};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::data::kt_tracked;
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 align-top";

#[component]
pub fn WeatherSatPage() -> impl IntoView {
  set_title("shell.weather-satellites");
  // 本页渲染 `crates/core` 的中文原文，译文词典要自己挂一次拉取（本页不是 `KnowledgePage`）。
  crate::data::use_knowledge_i18n();
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.weather-satellites")
        subtitle=move || t("radio.noaa-apt-meteor-lrpt")
      />

      <PageContainer>
        <section class="border bg-primary/5 p-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <div class="text-sm">{move || t("radio.got-an-apt-recording")}</div>
              <div class="mt-0.5 text-xs text-muted-foreground">
                {move || t("radio.upload-a-wav-to")}
              </div>
            </div>
            <a
              href="/apt-decoder"
              class="bg-primary px-3 py-1.5 text-xs text-primary-foreground"
            >
              {move || t("radio.open-the-apt-decoder")}
            </a>
          </div>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.getting-started-5-steps")}</h2>
          <ol class="space-y-3 p-4">
            {APT_GUIDE
              .iter()
              .map(|(title, desc, link)| {
                view! {
                  <li>
                    <div class="text-sm">{move || kt_tracked(title)}</div>
                    <div class="mt-0.5 text-sm text-muted-foreground">{move || kt_tracked(desc)}</div>
                    {link.map(|href| {
                      view! {
                        <a href=href class="mt-1 inline-block text-xs text-primary underline-offset-4 hover:underline">
                          {move || t("radio.go")}
                        </a>
                      }
                    })}
                  </li>
                }
              })
              .collect_view()}
          </ol>
        </section>

        <Table class="min-w-[640px] text-sm" wrapper_class="pxl-card border bg-card">
          <TableHead>
            <TableRow>
              <TableHeader class=CELL>{move || t("radio.satellite")}</TableHeader>
              <TableHeader class=CELL>{move || t("radio.signal")}</TableHeader>
              <TableHeader class=CELL>{move || t("contest.freq")}</TableHeader>
              <TableHeader class=CELL>{move || t("radio.notes")}</TableHeader>
            </TableRow>
          </TableHead>
          <TableBody>
            {WEATHER_SATS
              .iter()
              .map(|s| {
                view! {
                  <TableRow>
                    <TableCell class=format!("{CELL} whitespace-nowrap")>{move || kt_tracked(s.name)}</TableCell>
                    <TableCell class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{move || kt_tracked(s.signal)}</TableCell>
                    <TableCell class=format!("{CELL} whitespace-nowrap font-mono tabular-nums")>{move || kt_tracked(s.freq)}</TableCell>
                    <TableCell class=format!("{CELL} text-muted-foreground")>{move || kt_tracked(s.note)}</TableCell>
                  </TableRow>
                }
              })
              .collect_view()}
          </TableBody>
        </Table>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.key-concepts-heading")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {WEATHER_CONCEPTS
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex flex-col gap-1 px-3 py-2">
                    <span class="text-sm">{move || kt_tracked(t)}</span>
                    <span class="text-sm text-muted-foreground">{move || kt_tracked(d)}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.receiving-tips")}</h2>
          <ul class="space-y-2 p-4">
            {WEATHER_TIPS
              .iter()
              .map(|tip| {
                view! {
                  <li class="flex gap-2 text-sm text-muted-foreground">
                    <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                    <span>{move || kt_tracked(tip)}</span>
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
