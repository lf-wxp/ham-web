//! PSK Reporter：查询「谁收到了我发的信号」（数字模式实时接收报告）。

use ham_web_core::logbook::StationInfo;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use wasm_bindgen::JsValue;

use crate::data;
use crate::i18n::t;
use crate::ui::{Size, Variant, button_class, input_class};
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
      crate::util::alert("请输入呼号");
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
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"PSK Reporter"</h1>
            <div class="text-xs text-muted-foreground">{move || t("数字模式接收报告 · 谁收到了我发的信号")}</div>
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
              {move || if loading.get() { t("查询中…") } else { t("查询") }}
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
          let list = reports.get();
          if queried.get() && list.is_empty() {
            return view! {
              <section class="rounded-xl border bg-card p-6 text-sm text-muted-foreground">
                {move || t("最近没有查询到该呼号的接收报告（默认仅返回近期数字模式记录）。")}
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
                {t("接收报告（")} {list.len()} {t(" 条）")}
              </h2>
              <div class="overflow-x-auto">
                <table class="w-full min-w-[640px] border-collapse text-sm">
                  <thead class="bg-muted/60 text-xs">
                    <tr>
                      <th class="border px-3 py-2 text-left">{move || t("时间")}</th>
                      <th class="border px-3 py-2 text-left">{move || t("接收台")}</th>
                      <th class="border px-3 py-2 text-left">{move || t("频率")}</th>
                      <th class="border px-3 py-2 text-left">{move || t("波段")}</th>
                      <th class="border px-3 py-2 text-left">{move || t("模式")}</th>
                      <th class="border px-3 py-2 text-right">"SNR"</th>
                    </tr>
                  </thead>
                  <tbody>
                    {list
                      .iter()
                      .map(|r| {
                        view! {
                          <tr class="border-t transition-colors hover:bg-muted/40">
                            <td class="border px-3 py-2 tabular-nums text-muted-foreground">{format_utc(r.flow_start_seconds)}</td>
                            <td class="border px-3 py-2 font-medium">
                              {r.receiver.clone()}
                              {(!r.receiver_locator.is_empty()).then(|| view! {
                                <span class="ml-1.5 font-mono text-xs text-muted-foreground">{r.receiver_locator.clone()}</span>
                              })}
                            </td>
                            <td class="border px-3 py-2 tabular-nums text-muted-foreground">{format!("{:.1} kHz", r.freq_khz)}</td>
                            <td class="border px-3 py-2">{r.band.clone()}</td>
                            <td class="border px-3 py-2">{r.mode.clone()}</td>
                            <td class=format!("border px-3 py-2 text-right tabular-nums {}", if r.snr >= 0 { "text-emerald-600" } else { "text-muted-foreground" })>
                              {format!("{} dB", r.snr)}
                            </td>
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
          {move || t("数据来自 PSK Reporter（pskreporter.info），仅展示近期数字模式（FT8 / FT4 / WSPR 等）接收报告。")}
        </p>
      </div>
    </div>
  }
}
