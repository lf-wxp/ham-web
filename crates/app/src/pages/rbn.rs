//! RBN（Reverse Beacon Network）：查询「谁收到了我发的 CW / RTTY 信号」（实时监听）。

use ham_web_core::logbook::StationInfo;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::data;
use crate::i18n::t;
use crate::ui::{Size, Variant, button_class, input_class};
use crate::util::{set_title, storage};

/// 一条信标报告。
#[derive(Deserialize, Clone)]
struct RbnSpot {
  spotter: String,
  freq_khz: f64,
  band: String,
  mode: String,
  snr: i32,
  wpm: u32,
}

/// `/api/rbn` 返回。
#[derive(Deserialize, Clone)]
struct RbnPayload {
  spots: Vec<RbnSpot>,
}

#[component]
pub fn RbnPage() -> impl IntoView {
  set_title(&t("RBN 信标网络"));

  let default_call = storage::get_json::<StationInfo>("station-info")
    .map(|s| s.callsign)
    .unwrap_or_default();
  let callsign = RwSignal::new(default_call);
  let spots = RwSignal::new(Vec::<RbnSpot>::new());
  let loading = RwSignal::new(false);
  let failed = RwSignal::new(false);
  let queried = RwSignal::new(false);

  let run = move || {
    let call = callsign.get().trim().to_uppercase();
    if call.is_empty() {
      crate::util::alert("请输入呼号");
      return;
    }
    loading.set(true);
    failed.set(false);
    let url = format!("/api/rbn?callsign={call}");
    spawn_local(async move {
      match data::fetch_external_json::<RbnPayload>(&url).await {
        Ok(p) => {
          spots.set(p.spots);
          queried.set(true);
        }
        Err(_) => failed.set(true),
      }
      loading.set(false);
    });
  };

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("RBN 信标网络")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("CW / RTTY 信标台实时报告 · 谁收到了我")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("查询接收报告")}</h2>
          <div class="flex flex-wrap gap-3 p-4">
            <input
              type="text"
              placeholder=move || t("呼号（如 BG1XXX）")
              aria-label=move || t("呼号")
              prop:value=move || callsign.get()
              on:input=move |e| callsign.set(event_target_value(&e))
              class=input_class("max-w-xs")
            />
            <button type="button" class=button_class(Variant::Default, Size::Default, "") on:click=move |_| run()>
              {move || if loading.get() { t("监听中…") } else { t("查询") }}
            </button>
          </div>
        </section>

        {move || {
          if failed.get() {
            return view! {
              <section class="rounded-xl border bg-card p-6 text-sm text-muted-foreground">
                {move || t("查询暂不可用，请确认已通过后端（dev-full / serve）访问，或稍后重试。")}
              </section>
            }
            .into_any();
          }
          let list = spots.get();
          if queried.get() && list.is_empty() {
            return view! {
              <section class="rounded-xl border bg-card p-6 text-sm text-muted-foreground">
                {move || t("监听期间没有收到该呼号的信标报告（RBN 为实时流，仅在你发报时才会被记录）。")}
              </section>
            }
            .into_any();
          }
          if list.is_empty() {
            return view! { <div></div> }.into_any();
          }
          view! {
            <section class="rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">
                {t("信标报告（")} {list.len()} {t(" 条）")}
              </h2>
              <div class="overflow-x-auto">
                <table class="w-full min-w-[560px] border-collapse text-sm">
                  <thead class="bg-muted/60 text-xs">
                    <tr>
                      <th class="border px-3 py-2 text-left">{move || t("信标台")}</th>
                      <th class="border px-3 py-2 text-left">{move || t("频率")}</th>
                      <th class="border px-3 py-2 text-left">{move || t("波段")}</th>
                      <th class="border px-3 py-2 text-left">{move || t("模式")}</th>
                      <th class="border px-3 py-2 text-right">"SNR"</th>
                      <th class="border px-3 py-2 text-right">"WPM"</th>
                    </tr>
                  </thead>
                  <tbody>
                    {list
                      .iter()
                      .map(|s| {
                        view! {
                          <tr class="border-t transition-colors hover:bg-muted/40">
                            <td class="border px-3 py-2 font-medium">{s.spotter.clone()}</td>
                            <td class="border px-3 py-2 tabular-nums text-muted-foreground">{format!("{:.1} kHz", s.freq_khz)}</td>
                            <td class="border px-3 py-2">{s.band.clone()}</td>
                            <td class="border px-3 py-2">{s.mode.clone()}</td>
                            <td class=format!("border px-3 py-2 text-right tabular-nums {}", if s.snr >= 0 { "text-emerald-600" } else { "text-muted-foreground" })>
                              {format!("{} dB", s.snr)}
                            </td>
                            <td class="border px-3 py-2 text-right tabular-nums text-muted-foreground">{s.wpm}</td>
                          </tr>
                        }
                      })
                      .collect_view()}
                  </tbody>
                </table>
              </div>
            </section>
          }
          .into_any()
        }}

        <p class="text-xs text-muted-foreground">
          {move || t("数据来自 Reverse Beacon Network（telnet.reversebeacon.net:7000），为实时流：仅在发报（CW / RTTY）时才会被各地信标台记录并推送。")}
        </p>
      </div>
    </div>
  }
}
