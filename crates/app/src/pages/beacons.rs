//! 国际信标网络（NCDXF/IARU）速查：18 台信标、5 波段轮询与当前时隙高亮。

use std::time::Duration;

use ham_web_core::beacons::{BEACON_BANDS, BEACONS, DASH_POWERS, SLOT_SECS, beacon_slot};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::{t, tf};
use crate::util::set_title;

#[component]
pub fn BeaconsPage() -> impl IntoView {
  set_title(&t("国际信标网络"));

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
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=Signal::derive(move || t("国际信标网络"))
        subtitle=Signal::derive(move || t("NCDXF/IARU · 18 台 5 波段 · 判断传播开通"))
      />
      <PageContainer>
        // 当前时隙：正在发射的信标台。
        <section class="rounded-xl border bg-card p-4">
          <div class="flex flex-wrap items-center gap-x-4 gap-y-2">
            <div class="font-mono text-2xl font-semibold tabular-nums">{move || clock.get()}</div>
            {move || {
              let (idx, into) = slot.get();
              let (call, loc, region) = BEACONS[idx];
              let remaining = SLOT_SECS - into;
              let pct = into as f64 / SLOT_SECS as f64 * 100.0;
              view! {
                <div class="ml-auto flex items-center gap-3">
                  <div class="text-right">
                    <div class="text-sm font-semibold">
                      {tf("正在发射：{}（{} · {}）", &[call, loc, region])}
                    </div>
                    <div class="text-xs text-muted-foreground">
                      {tf("本台剩余约 {} 秒", &[&remaining.to_string()])}
                    </div>
                  </div>
                  <div class="h-2 w-24 overflow-hidden rounded-full bg-muted">
                    <div
                      class="h-full rounded-full bg-primary transition-all"
                      style=format!("width: {pct}%")
                    ></div>
                  </div>
                </div>
              }
            }}
          </div>
        </section>

        // 信标台列表。
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("信标台（按轮询顺序）")}</h2>
          <div class="grid gap-2 p-4 sm:grid-cols-2 lg:grid-cols-3">
            {BEACONS
              .iter()
              .enumerate()
              .map(|(i, &(call, loc, region))| {
                view! {
                  <div class=move || if i == slot.get().0 {
                    "rounded-lg border border-primary bg-primary/5 px-3 py-2"
                  } else {
                    "rounded-lg border px-3 py-2"
                  }>
                    <div class="flex items-center gap-2">
                      <span class="font-mono text-xs text-muted-foreground">{format!("{:02}", i + 1)}</span>
                      <span class="font-mono text-sm font-semibold">{call}</span>
                    </div>
                    <div class="mt-0.5 text-xs text-muted-foreground">{tf("{} · {}", &[loc, region])}</div>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        // 信标频率。
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("信标频率")}</h2>
          <div class="grid gap-2 p-4 sm:grid-cols-2 lg:grid-cols-5">
            {BEACON_BANDS
              .iter()
              .map(|&(band, freq)| {
                view! {
                  <div class="rounded-lg border px-3 py-2 text-center">
                    <div class="text-xs text-muted-foreground">{band}</div>
                    <div class="font-mono text-sm font-semibold tabular-nums">{format!("{freq:.3} MHz")}</div>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        // 信号格式。
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("信号格式")}</h2>
          <div class="space-y-3 p-4">
            <p class="text-sm text-muted-foreground">
              {move || t("每台发射 10 秒：先以 CW（约 20 WPM）发送呼号，再发 4 个各约 1 秒的长划，功率逐级下降 10 dB。")}
            </p>
            <div class="flex flex-wrap gap-2">
              {DASH_POWERS
                .iter()
                .map(|&(label, pwr)| {
                  view! {
                    <span class="rounded-md bg-muted/60 px-2 py-1 text-xs tabular-nums">
                      {tf("{}：{} W", &[label, &format!("{pwr}")])}
                    </span>
                  }
                })
                .collect_view()}
            </div>
            <p class="text-sm text-muted-foreground">
              {move || t("能听到第几个长划，就能估算这条路径的损耗余量：听到 0.1W 的一划说明路径极佳。")}
            </p>
          </div>
        </section>

        // 使用说明。
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("怎么看")}</h2>
          <ul class="space-y-2 p-4">
            <li class="flex gap-2 text-sm text-muted-foreground">
              <span class="mt-0.5 shrink-0 text-primary">"•"</span>
              <span>{move || t("把电台调谐到某个信标频率，按上表对照此刻是哪台在发射。")}</span>
            </li>
            <li class="flex gap-2 text-sm text-muted-foreground">
              <span class="mt-0.5 shrink-0 text-primary">"•"</span>
              <span>{move || t("听到某台信标，说明「你 ↔ 该台」这条传播路径当前开通，可据此推断该方向 DX 的可行性。")}</span>
            </li>
            <li class="flex gap-2 text-sm text-muted-foreground">
              <span class="mt-0.5 shrink-0 text-primary">"•"</span>
              <span>{move || t("傍晚 / 清晨与灰线时段效果最佳；换不同波段轮流听，可判断各波段开通顺序。")}</span>
            </li>
          </ul>
        </section>

        <p class="text-xs text-muted-foreground">
          {move || t("时隙按本地时钟估算并向下取整，实际相位可能存在秒级偏差；信标实际是否在线以现场收讯为准。")}
        </p>
      </PageContainer>
    </div>
  }
}
