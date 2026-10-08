//! 点对点传播预测卡片：输入收发网格 / 月份 / 太阳黑子数，调用 `/api/voacap` 估算各波段可用性。

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::data;
use crate::i18n::{t, tf};
use crate::ui::{
  Button, ControlSize, Field, Input, NativeSelect, NumberField, SelectOption, Size, Variant,
};
use crate::util::alert;
use crate::util::unique_id;

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
  /// 路径中点的地方时（0–24）；未指定时刻时为 `None`。
  ///
  /// 判断「有没有指定时刻」就靠它：`hour_utc` 只是输入回显，界面上要看的是地方时。
  local_hour: Option<f64>,
  /// 昼夜衰减因子（0.35–1）。
  diurnal: f64,
}

/// 点对点传播预测卡片。
#[component]
pub(super) fn VoacapCard() -> impl IntoView {
  let tx = RwSignal::new(String::new());
  let rx = RwSignal::new(String::new());
  let month = RwSignal::new(10u32);
  let ssn = RwSignal::new(100.0);
  // `None` 表示不指定时刻（按路径日照最佳情况估算）。
  let hour = RwSignal::new(None::<u32>);
  let result = RwSignal::new(None::<VoacapResponse>);
  let loading = RwSignal::new(false);
  let failed = RwSignal::new(false);

  // `Field` 的标签与控件是兄弟节点，`r#for` / `id` 必须配对才能点击标签聚焦输入框
  //（e2e 与读屏都按「标签 → 控件」的关联来定位）。
  let tx_id = unique_id("voacap-tx");
  let rx_id = unique_id("voacap-rx");
  let month_id = unique_id("voacap-month");
  let ssn_id = unique_id("voacap-ssn");

  let hour_options: Vec<SelectOption> = (0..24u32)
    .map(|h| SelectOption::new(h.to_string(), format!("{h:02}:00 UTC")))
    .collect();

  // 卡片卸载后 `spawn_local` 的续体不能再碰信号（释放后访问会 panic，见 `util::mount_guard`）。
  // 必须在组件体里创建：事件回调里调用 `on_cleanup` 是静默空操作。
  let alive = crate::util::mount_guard();
  let run = move || {
    let tx_g = tx.get().trim().to_uppercase();
    let rx_g = rx.get().trim().to_uppercase();
    if tx_g.len() < 4 || rx_g.len() < 4 {
      // 必须走词典：原先这里写死中文，en / es 界面下会弹出一条中文提示。
      alert(&t("radio.enter-maidenhead-grids-of"));
      return;
    }
    loading.set(true);
    failed.set(false);
    let hour_param = hour
      .get()
      .map_or_else(String::new, |h| format!("&hour={h}"));
    let url = format!(
      "/api/voacap?tx={tx_g}&rx={rx_g}&month={}&ssn={}{}",
      month.get(),
      ssn.get(),
      hour_param
    );
    let alive = alive.clone();
    spawn_local(async move {
      let fetched = data::fetch_external_json::<VoacapResponse>(&url).await;
      // 用户可能在预测返回前离开本页：此时信号已释放，碰它就是 panic。
      if !alive() {
        return;
      }
      match fetched {
        Ok(r) => result.set(Some(r)),
        Err(_) => failed.set(true),
      }
      loading.set(false);
    });
  };

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.point-to-point-propagation")}</h2>
      <p class="px-4 pt-3 text-xs text-muted-foreground">
        {move || t("tools.enter-both-grids-month")}
      </p>
      <div class="grid items-end gap-3 p-4 sm:grid-cols-[1fr_1fr_auto_auto_auto]">
        <Field label=Signal::derive(move || t("radio.transmit-grid")) r#for=tx_id.clone()>
          <Input
            id=tx_id
            value=tx
            on_change=Callback::new(move |v: String| tx.set(v))
            placeholder=Signal::derive(move || t("radio.your-grid-e-g"))
            aria_label=Signal::derive(move || t("radio.transmit-grid"))
            class="uppercase"
          />
        </Field>
        <Field label=Signal::derive(move || t("radio.receive-grid")) r#for=rx_id.clone()>
          <Input
            id=rx_id
            value=rx
            on_change=Callback::new(move |v: String| rx.set(v))
            placeholder=Signal::derive(move || t("radio.their-grid-e-g"))
            aria_label=Signal::derive(move || t("radio.receive-grid"))
            class="uppercase"
          />
        </Field>
        <Field label=Signal::derive(move || t("radio.month")) r#for=month_id.clone()>
          <NumberField
            id=month_id
            step=1.0
            min=1.0
            max=12.0
            value=Signal::derive(move || month.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<u32>() {
                month.set(v.clamp(1, 12));
              }
            })
            aria_label=Signal::derive(move || t("radio.month"))
            controls=false
          />
        </Field>
        <Field label=Signal::derive(move || t("radio.sunspot-number")) r#for=ssn_id.clone()>
          <NumberField
            id=ssn_id
            min=0.0
            max=400.0
            value=Signal::derive(move || ssn.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                ssn.set(v.clamp(0.0, 400.0));
              }
            })
            aria_label=Signal::derive(move || t("radio.sunspot-number"))
            controls=false
          />
        </Field>
        <NativeSelect
          value=Signal::derive(move || hour.get().map_or_else(String::new, |h| h.to_string()))
          on_change=Callback::new(move |v: String| {
            hour.set(if v.is_empty() { None } else { v.parse::<u32>().ok() });
          })
          options=hour_options
          placeholder=Signal::derive(move || t("tools.best-window"))
          size=ControlSize::Sm
          aria_label=Signal::derive(move || t("tools.utc-time"))
          class="w-28"
        />
        <Button
          variant=Variant::Default
          size=Size::Default
          on_click=Callback::new(move |_| run())
        >
          {move || if loading.get() { t("radio.forecasting") } else { t("radio.forecast") }}
        </Button>
      </div>
      <div class="px-4 pb-4">
        {move || {
          if failed.get() {
            return view! {
              <p class="text-sm text-muted-foreground">{move || t("radio.forecast-unavailable-make-sure")}</p>
            }
            .into_any();
          }
          result
            .get()
            .map(|r| {
              view! {
                <div class="space-y-3">
                  <div class="flex flex-wrap gap-x-5 gap-y-1 text-sm">
                    <span>{move || t("radio.distance")} <b class="tabular-nums">{format!("{:.0} km", r.distance_km)}</b></span>
                    <span>{move || t("radio.bearing")} <b class="tabular-nums">{format!("{:.0}°", r.bearing_deg)}</b></span>
                    <span>"foF2 " <b class="tabular-nums">{format!("{:.1} MHz", r.fo_f2)}</b></span>
                    <span>"MUF " <b class="tabular-nums">{format!("{:.1} MHz", r.muf)}</b></span>
                  </div>
                  <p class="text-xs text-muted-foreground">
                    {move || {
                      r.local_hour
                        .map(|lh| {
                          // 先按分钟取整再拆分时/分：否则 23.996h 会因分钟四舍五入到 60
                          // 而显示成「23:00」而不是进位到「00:00」。
                          let total_min = (lh * 60.0).round() as u32 % (24 * 60);
                          // `tf` 按 `{}` 出现顺序替换，格式化需在参数侧完成。
                          tf(
                            "tools.local-time-at-the",
                            &[
                              &format!("{:02}", total_min / 60),
                              &format!("{:02}", total_min % 60),
                              &format!("{:.0}", r.diurnal * 100.0),
                            ],
                          )
                        })
                        .unwrap_or_else(|| t("tools.no-time-specified-estimated"))
                    }}
                  </p>
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
                              {if b.usable { tf("radio.usable", &[&(format!("{pct:.0}")).to_string()]) } else { t("radio.unusable") }}
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
