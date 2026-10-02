//! 气象卫星接收：NOAA APT / METEOR LRPT 速查。

use ham_web_core::weather_sat::{APT_GUIDE, WEATHER_CONCEPTS, WEATHER_SATS, WEATHER_TIPS};
use leptos::prelude::*;

use crate::i18n::t;
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn WeatherSatPage() -> impl IntoView {
  set_title(&t("气象卫星接收"));
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("气象卫星接收")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("NOAA APT · METEOR LRPT · RTL-SDR 收图")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-primary/5 p-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <div class="text-sm font-semibold">{move || t("有 APT 录音？试试在浏览器本地解码")}</div>
              <div class="mt-0.5 text-xs text-muted-foreground">
                {move || t("上传 WAV，本地解调 2400 Hz 副载波并重建可见光 / 红外云图")}
              </div>
            </div>
            <a
              href="/apt-decoder"
              class="rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground"
            >
              {move || t("打开 APT 解码器")}
            </a>
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("接收上手（5 步）")}</h2>
          <ol class="space-y-3 p-4">
            {APT_GUIDE
              .iter()
              .map(|(title, desc, link)| {
                view! {
                  <li>
                    <div class="text-sm font-medium">{*title}</div>
                    <div class="mt-0.5 text-sm text-muted-foreground">{*desc}</div>
                    {link.map(|href| {
                      view! {
                        <a href=href class="mt-1 inline-block text-xs text-primary underline-offset-4 hover:underline">
                          {move || t("前往 →")}
                        </a>
                      }
                    })}
                  </li>
                }
              })
              .collect_view()}
          </ol>
        </section>

        <div class="overflow-x-auto rounded-xl border bg-card">
          <table class="w-full min-w-[640px] border-collapse text-sm">
            <thead class="bg-muted/60 text-xs">
              <tr>
                <th class=CELL>{move || t("卫星")}</th>
                <th class=CELL>{move || t("信号")}</th>
                <th class=CELL>{move || t("频率")}</th>
                <th class=CELL>{move || t("说明")}</th>
              </tr>
            </thead>
            <tbody>
              {WEATHER_SATS
                .iter()
                .map(|s| {
                  view! {
                    <tr class="border-t transition-colors hover:bg-muted/40">
                      <td class=format!("{CELL} whitespace-nowrap font-medium")>{s.name}</td>
                      <td class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{s.signal}</td>
                      <td class=format!("{CELL} whitespace-nowrap font-mono tabular-nums")>{s.freq}</td>
                      <td class=format!("{CELL} text-muted-foreground")>{s.note}</td>
                    </tr>
                  }
                })
                .collect_view()}
            </tbody>
          </table>
        </div>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("核心概念")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {WEATHER_CONCEPTS
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("接收要点")}</h2>
          <ul class="space-y-2 p-4">
            {WEATHER_TIPS
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
