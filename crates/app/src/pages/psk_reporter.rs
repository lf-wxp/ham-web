//! PSK Reporter：查询「谁收到了我发的信号」（数字模式实时接收报告）。

use ham_web_core::logbook::StationInfo;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use wasm_bindgen::JsValue;

use crate::components::common::{EmptyState, PageContainer, PageHeader};
use crate::data;
use crate::i18n::t;
use crate::ui::{
  Button, Input, Size, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, Variant,
};
use crate::util::{set_title, storage};

/// 单条接收报告。
#[derive(Deserialize, Clone)]
struct PskReport {
  receiver: String,
  freq_khz: f64,
  band: String,
  mode: String,
  snr: i32,
  flow_start_seconds: i64,
  receiver_locator: String,
}

/// `/api/psk-reporter` 返回。
#[derive(Deserialize, Clone)]
struct PskPayload {
  reports: Vec<PskReport>,
}

fn format_utc(unix_sec: i64) -> String {
  let d = js_sys::Date::new(&JsValue::from_f64(unix_sec as f64 * 1000.0));
  format!(
    "{:04}-{:02}-{:02} {:02}:{:02} UTC",
    d.get_utc_full_year(),
    d.get_utc_month() + 1,
    d.get_utc_date(),
    d.get_utc_hours(),
    d.get_utc_minutes()
  )
}

#[component]
pub fn PskReporterPage() -> impl IntoView {
  set_title("PSK Reporter");

  // 默认呼号取本台信息。
  let default_call = storage::get_json::<StationInfo>("station-info")
    .map(|s| s.callsign)
    .unwrap_or_default();
  let callsign = RwSignal::new(default_call);
  let reports = RwSignal::new(Vec::<PskReport>::new());
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
    let url = format!("/api/psk-reporter?callsign={call}");
    spawn_local(async move {
      match data::fetch_external_json::<PskPayload>(&url).await {
        Ok(p) => {
          reports.set(p.reports);
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
        title="PSK Reporter".to_string()
        subtitle=move || t("radio.digital-mode-reception-reports")
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
              {move || if loading.get() { t("log.looking-up") } else { t("log.look-up") }}
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
          let list = reports.get();
          if queried.get() && list.is_empty() {
            return view! {
              <EmptyState title=Signal::derive(move || t("radio.no-recent-reception-reports")) />
            }
            .into_any();
          }
          if list.is_empty() {
            return view! { <div></div> }.into_any();
          }
          view! {
            <section class="pxl-card border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">
                {t("radio.reception-reports")} {list.len()} {t("radio.entry")}
              </h2>
              <Table class="min-w-[640px] text-sm">
                <TableHead>
                  <TableRow>
                    <TableHeader class="border px-3 py-2">{move || t("log.time")}</TableHeader>
                    <TableHeader class="border px-3 py-2">{move || t("radio.receivers")}</TableHeader>
                    <TableHeader class="border px-3 py-2">{move || t("contest.freq")}</TableHeader>
                    <TableHeader class="border px-3 py-2">{move || t("radio.band")}</TableHeader>
                    <TableHeader class="border px-3 py-2">{move || t("log.mode")}</TableHeader>
                    <TableHeader class="border px-3 py-2 text-right">"SNR"</TableHeader>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {list
                    .iter()
                    .map(|r| {
                      let r = r.clone();
                      view! {
                        <TableRow>
                          <TableCell class="border px-3 py-2 tabular-nums text-muted-foreground">{format_utc(r.flow_start_seconds)}</TableCell>
                          <TableCell class="border px-3 py-2 font-medium">
                            {r.receiver.clone()}
                            {(!r.receiver_locator.is_empty()).then(|| view! {
                              <span class="ml-1.5 font-mono text-xs text-muted-foreground">{r.receiver_locator.clone()}</span>
                            })}
                          </TableCell>
                          <TableCell class="border px-3 py-2 tabular-nums text-muted-foreground">{format!("{:.1} kHz", r.freq_khz)}</TableCell>
                          <TableCell class="border px-3 py-2">{r.band.clone()}</TableCell>
                          <TableCell class="border px-3 py-2">{r.mode.clone()}</TableCell>
                          <TableCell class=format!("border px-3 py-2 text-right tabular-nums {}", if r.snr >= 0 { "text-emerald-600" } else { "text-muted-foreground" })>
                            {format!("{} dB", r.snr)}
                          </TableCell>
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
          {move || t("radio.data-from-psk-reporter")}
        </p>
      </PageContainer>
    </div>
  }
}
