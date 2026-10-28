//! 国际信标网络（NCDXF/IARU）速查：18 台信标、5 波段轮询与当前时隙高亮。

use std::time::Duration;

use ham_web_core::beacons::{BEACON_BANDS, BEACONS, DASH_POWERS, SLOT_SECS, beacon_slot};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::data;
use crate::i18n::{t, tf};
use crate::util::set_title;

#[component]
pub fn BeaconsPage() -> impl IntoView {
  set_title("knowledge.international-beacon-network");
  // 本页渲染 `crates/core` 的中文原文（`kt_tracked`），译文词典要自己挂一次拉取，
  // 否则以英文 / 西语直接打开本页时词典永远不来、这些文案停在中文。
  data::use_knowledge_i18n();

  let now_ms = RwSignal::new(js_sys::Date::new_0().get_time());
  if let Ok(handle) = set_interval_with_handle(
    move || now_ms.set(js_sys::Date::new_0().get_time()),
    Duration::from_secs(1),
  ) {
    on_cleanup(move || handle.clear());
  }

  let slot = Memo::new(move |_| {
    let secs = (now_ms.get() / 1000.0).floor() as i64;
    beacon_slot(secs)
  });

  let clock = Memo::new(move |_| {
    let date = js_sys::Date::new_0();
    format!(
      "{:02}:{:02}:{:02} UTC",
      date.get_utc_hours(),
      date.get_utc_minutes(),
      date.get_utc_seconds()
    )
  });

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=Signal::derive(move || t("knowledge.international-beacon-network"))
        subtitle=Signal::derive(move || t("knowledge.ncdxf-iaru-18-beacons"))
      />
      <PageContainer>
        // 当前时隙：正在发射的信标台。
        <section class="pxl-card border bg-card p-4">
          <div class="flex flex-wrap items-center gap-x-4 gap-y-2">
            <div class="pxl-stat">{move || clock.get()}</div>
            {move || {
              let (idx, into) = slot.get();
              let (call, loc, region) = BEACONS[idx];
              let remaining = SLOT_SECS - into;
              let pct = into as f64 / SLOT_SECS as f64 * 100.0;
              view! {
                <div class="ml-auto flex items-center gap-3">
                  <div class="text-right">
                    <div class="text-sm">
                      {move || tf("knowledge.transmitting-now", &[call, &data::kt_tracked(loc), &data::kt_tracked(region)])}
                    </div>
                    <div class="text-xs text-muted-foreground">
                      {move || tf("knowledge.about-s-left-for", &[&remaining.to_string()])}
                    </div>
                  </div>
                  <div class="h-2 w-24 overflow-hidden bg-muted">
                    <div
                      class="h-full bg-primary transition"
                      style=format!("width: {pct}%")
                    ></div>
                  </div>
                </div>
              }
            }}
          </div>
        </section>

        // 信标台列表。
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.beacons-in-polling-order")}</h2>
          <div class="grid gap-2 p-4 sm:grid-cols-2 lg:grid-cols-3">
            {BEACONS
              .iter()
              .enumerate()
              .map(|(i, &(call, loc, region))| {
                view! {
                  <div class=move || if i == slot.get().0 {
                    "border border-primary bg-primary/5 px-3 py-2"
                  } else {
                    "border px-3 py-2"
                  }>
                    <div class="flex items-center gap-2">
                      <span class="font-mono text-xs text-muted-foreground">{format!("{:02}", i + 1)}</span>
                      <span class="font-mono text-sm">{call}</span>
                    </div>
                    <div class="mt-0.5 text-xs text-muted-foreground">{move || tf("common.pair", &[&data::kt_tracked(loc), &data::kt_tracked(region)])}</div>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        // 信标频率。
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.beacon-frequency")}</h2>
          <div class="grid gap-2 p-4 sm:grid-cols-2 lg:grid-cols-5">
            {BEACON_BANDS
              .iter()
              .map(|&(band, freq)| {
                view! {
                  <div class="border px-3 py-2 text-center">
                    <div class="text-xs text-muted-foreground">{band}</div>
                    <div class="font-mono text-sm tabular-nums">{format!("{freq:.3} MHz")}</div>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        // 信号格式。
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.signal-format")}</h2>
          <div class="space-y-3 p-4">
            <p class="text-sm text-muted-foreground">
              {move || t("knowledge.each-station-transmits-for")}
            </p>
            <div class="flex flex-wrap gap-2">
              {DASH_POWERS
                .iter()
                .map(|&(label, pwr)| {
                  view! {
                    <span class="bg-inset px-2 py-1 text-xs tabular-nums">
                      {move || tf("common.watt-value", &[&data::kt_tracked(label), &format!("{pwr}")])}
                    </span>
                  }
                })
                .collect_view()}
            </div>
            <p class="text-sm text-muted-foreground">
              {move || t("knowledge.the-last-dash-you")}
            </p>
          </div>
        </section>

        // 使用说明。
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.how-to-read-it")}</h2>
          <ul class="space-y-2 p-4">
            <li class="flex gap-2 text-sm text-muted-foreground">
              <span class="mt-0.5 shrink-0 text-primary">"•"</span>
              <span>{move || t("knowledge.tune-the-radio-to")}</span>
            </li>
            <li class="flex gap-2 text-sm text-muted-foreground">
              <span class="mt-0.5 shrink-0 text-primary">"•"</span>
              <span>{move || t("knowledge.hearing-a-beacon-means")}</span>
            </li>
            <li class="flex gap-2 text-sm text-muted-foreground">
              <span class="mt-0.5 shrink-0 text-primary">"•"</span>
              <span>{move || t("knowledge.best-around-dusk-dawn")}</span>
            </li>
          </ul>
        </section>

        <p class="text-xs text-muted-foreground">
          {move || t("knowledge.time-slots-are-estimated")}
        </p>
      </PageContainer>
    </div>
  }
}
