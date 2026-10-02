//! 点对点传播预测卡片：输入收发网格 / 月份 / 太阳黑子数，调用 `/api/voacap` 估算各波段可用性。

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::data;
use crate::i18n::{t, tf};
use crate::ui::{Size, Variant, button_class, input_class};
use crate::util::alert;

/// 单波段预测结果。
#[derive(Deserialize, Clone)]
struct BandResult {
  band: String,
  usable: bool,
  reliability: f64,
}

/// `/api/voacap` 返回。
#[derive(Deserialize, Clone)]
struct VoacapResponse {
  distance_km: f64,
  bearing_deg: f64,
  fo_f2: f64,
  muf: f64,
  bands: Vec<BandResult>,
}

const INPUT: &str = "h-10 rounded-lg border bg-background px-3 text-sm tabular-nums outline-none focus-visible:ring-2 focus-visible:ring-ring/50";

/// 点对点传播预测卡片。
#[component]
pub(super) fn VoacapCard() -> impl IntoView {
  let tx = RwSignal::new(String::new());
  let rx = RwSignal::new(String::new());
  let month = RwSignal::new(10u32);
  let ssn = RwSignal::new(100.0);
  let result = RwSignal::new(None::<VoacapResponse>);
  let loading = RwSignal::new(false);
  let failed = RwSignal::new(false);

  let run = move || {
    let tx_g = tx.get().trim().to_uppercase();
    let rx_g = rx.get().trim().to_uppercase();
    if tx_g.len() < 4 || rx_g.len() < 4 {
      alert("请输入至少 4 位 Maidenhead 网格（如 OM89、IO91）");
      return;
    }
    loading.set(true);
    failed.set(false);
    let url = format!(
      "/api/voacap?tx={tx_g}&rx={rx_g}&month={}&ssn={}",
      month.get(),
      ssn.get()
    );
    spawn_local(async move {
      match data::fetch_external_json::<VoacapResponse>(&url).await {
        Ok(r) => result.set(Some(r)),
        Err(_) => failed.set(true),
      }
      loading.set(false);
    });
  };

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("点对点传播预测")}</h2>
      <p class="px-4 pt-3 text-xs text-muted-foreground">
        {move || t("输入双方网格与月份、太阳黑子数（可到「太阳活动」页查看当前值），估算两点间各波段的可用性与可靠度（简化模型，仅供参考）。")}
      </p>
      <div class="grid gap-3 p-4 sm:grid-cols-[1fr_1fr_auto_auto_auto]">
        <input
          type="text"
          placeholder=move || t("本台网格（如 OM89）")
          aria-label=move || t("发射端网格")
          prop:value=move || tx.get()
          on:input=move |e| tx.set(event_target_value(&e))
          class=input_class("")
        />
        <input
          type="text"
          placeholder=move || t("对方网格（如 IO91）")
          aria-label=move || t("接收端网格")
          prop:value=move || rx.get()
          on:input=move |e| rx.set(event_target_value(&e))
          class=input_class("")
        />
        <input
          type="number"
          min="1"
          max="12"
          aria-label=move || t("月份")
          prop:value=move || month.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<u32>() {
              month.set(v.clamp(1, 12));
            }
          }
          class=INPUT
        />
        <input
          type="number"
          min="0"
          max="400"
          aria-label=move || t("太阳黑子数")
          prop:value=move || ssn.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              ssn.set(v.clamp(0.0, 400.0));
            }
          }
          class=INPUT
        />
        <button type="button" class=button_class(Variant::Default, Size::Default, "") on:click=move |_| run()>
          {move || if loading.get() { t("预测中…") } else { t("预测") }}
        </button>
      </div>
      <div class="px-4 pb-4">
        {move || {
          if failed.get() {
            return view! {
              <p class="text-sm text-muted-foreground">{move || t("预测暂不可用，请确认已通过后端（dev-full / serve）访问。")}</p>
            }
            .into_any();
          }
          result
            .get()
            .map(|r| {
              view! {
                <div class="space-y-3">
                  <div class="flex flex-wrap gap-x-5 gap-y-1 text-sm">
                    <span>{move || t("距离 ")} <b class="tabular-nums">{format!("{:.0} km", r.distance_km)}</b></span>
                    <span>{move || t("方位 ")} <b class="tabular-nums">{format!("{:.0}°", r.bearing_deg)}</b></span>
                    <span>"foF2 " <b class="tabular-nums">{format!("{:.1} MHz", r.fo_f2)}</b></span>
                    <span>"MUF " <b class="tabular-nums">{format!("{:.1} MHz", r.muf)}</b></span>
                  </div>
                  <div class="space-y-1.5">
                    {r.bands
                      .iter()
                      .map(|b| {
                        let pct = b.reliability * 100.0;
                        view! {
                          <div class="flex items-center gap-2 text-sm">
                            <span class="w-12 shrink-0 font-medium">{b.band.clone()}</span>
                            <div class="h-3 flex-1 overflow-hidden rounded bg-muted">
                              <div
                                class=format!("h-full {}", if b.usable { "bg-emerald-500" } else { "bg-muted-foreground/30" })
                                style=format!("width: {pct:.0}%")
                              ></div>
                            </div>
                            <span class="w-24 shrink-0 text-right text-xs tabular-nums text-muted-foreground">
                              {if b.usable { tf("可用 · {}%", &[&(format!("{pct:.0}")).to_string()]) } else { t("不可用") }}
                            </span>
                          </div>
                        }
                      })
                      .collect_view()}
                  </div>
                </div>
              }
            })
            .into_any()
        }}
      </div>
    </section>
  }
}
