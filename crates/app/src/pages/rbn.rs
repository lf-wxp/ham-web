//! RBN（Reverse Beacon Network）：查询「谁收到了我发的 CW / RTTY 信号」（实时监听）。

use ham_web_core::logbook::StationInfo;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::components::common::{EmptyState, PageContainer, PageHeader};
use crate::data;
use crate::i18n::t;
use crate::ui::{
  Button, Input, Size, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, Variant,
};
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
      crate::ui::push_error(t("common.enter-a-callsign"));
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
      <PageHeader
        title=move || t("shell.rbn")
        subtitle=move || t("radio.live-cw-rtty-skimmer")
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
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
              <EmptyState title=Signal::derive(move || t("radio.lookup-unavailable-make-sure")) />
            }
            .into_any();
          }
          let list = spots.get();
          if queried.get() && list.is_empty() {
            return view! {
              <EmptyState title=Signal::derive(move || t("radio.no-skimmer-reports-for")) />
            }
            .into_any();
          }
          if list.is_empty() {
            return view! { <div></div> }.into_any();
          }
          view! {
            <section class="pxl-card border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">
                {t("radio.beacon-reports")} {list.len()} {t("radio.entry")}
              </h2>
              <Table class="min-w-[560px] text-sm">
                <TableHead>
                  <TableRow>
                    <TableHeader class="border px-3 py-2">{move || t("radio.skimmers")}</TableHeader>
                    <TableHeader class="border px-3 py-2">{move || t("contest.freq")}</TableHeader>
                    <TableHeader class="border px-3 py-2">{move || t("radio.band")}</TableHeader>
                    <TableHeader class="border px-3 py-2">{move || t("log.mode")}</TableHeader>
                    <TableHeader class="border px-3 py-2 text-right">"SNR"</TableHeader>
                    <TableHeader class="border px-3 py-2 text-right">"WPM"</TableHeader>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {list
                    .iter()
                    .map(|s| {
                      let s = s.clone();
                      view! {
                        <TableRow>
                          <TableCell class="border px-3 py-2 font-medium">{s.spotter.clone()}</TableCell>
                          <TableCell class="border px-3 py-2 tabular-nums text-muted-foreground">{format!("{:.1} kHz", s.freq_khz)}</TableCell>
                          <TableCell class="border px-3 py-2">{s.band.clone()}</TableCell>
                          <TableCell class="border px-3 py-2">{s.mode.clone()}</TableCell>
                          <TableCell class=format!("border px-3 py-2 text-right tabular-nums {}", if s.snr >= 0 { "text-emerald-600" } else { "text-muted-foreground" })>
                            {format!("{} dB", s.snr)}
                          </TableCell>
                          <TableCell class="border px-3 py-2 text-right tabular-nums text-muted-foreground">{s.wpm}</TableCell>
                        </TableRow>
                      }
                    })
                    .collect_view()}
                </TableBody>
              </Table>
            </section>
          }
          .into_any()
        }}

        <p class="text-xs text-muted-foreground">
          {move || t("radio.data-from-the-reverse")}
        </p>
      </PageContainer>
    </div>
  }
}
