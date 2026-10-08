//! RBN（Reverse Beacon Network）：查询「谁收到了我发的 CW / RTTY 信号」（实时监听）。

use ham_web_core::logbook::StationInfo;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::data;
use crate::i18n::t;
use crate::ui::{Button, Input, Size, Variant};
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
  set_title("shell.rbn");

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
            <h1 class="text-base font-semibold leading-tight">{move || t("shell.rbn")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("radio.live-cw-rtty-skimmer")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.look-up-reception-reports")}</h2>
          <div class="flex flex-wrap gap-3 p-4">
            <Input
              value=callsign
              on_change=Callback::new(move |v: String| callsign.set(v))
              placeholder=Signal::derive(move || t("radio.callsign-e-g-bg1xxx"))
              aria_label=Signal::derive(move || t("log.callsign"))
              class="max-w-xs"
            />
            <Button
              variant=Variant::Default
              size=Size::Default
              on_click=Callback::new(move |_| run())
            >
              {move || if loading.get() { t("radio.listening") } else { t("log.look-up") }}
            </Button>
          </div>
        </section>

        {move || {
          if failed.get() {
            return view! {
              <section class="rounded-xl border bg-card p-6 text-sm text-muted-foreground">
                {move || t("radio.lookup-unavailable-make-sure")}
              </section>
            }
            .into_any();
          }
          let list = spots.get();
          if queried.get() && list.is_empty() {
            return view! {
              <section class="rounded-xl border bg-card p-6 text-sm text-muted-foreground">
                {move || t("radio.no-skimmer-reports-for")}
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
                {t("radio.beacon-reports")} {list.len()} {t("radio.entry")}
              </h2>
              <div class="overflow-x-auto">
                <table class="w-full min-w-[560px] border-collapse text-sm">
                  <thead class="bg-muted/60 text-xs">
                    <tr>
                      <th class="border px-3 py-2 text-left">{move || t("radio.skimmers")}</th>
                      <th class="border px-3 py-2 text-left">{move || t("contest.freq")}</th>
                      <th class="border px-3 py-2 text-left">{move || t("radio.band")}</th>
                      <th class="border px-3 py-2 text-left">{move || t("log.mode")}</th>
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
          {move || t("radio.data-from-the-reverse")}
        </p>
      </div>
    </div>
  }
}
