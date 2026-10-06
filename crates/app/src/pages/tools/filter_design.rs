//! 滤波器设计：Butterworth 低通 / 高通 / 带通 / 带阻元件值计算。

use ham_web_core::filter_design::{
  FilterKind, ResonatorTopology, design_bandpass, design_bandstop, design_highpass, design_lowpass,
};
use leptos::prelude::*;

use super::{INPUT, fmt_num};
use crate::i18n::{t, tf};

const KINDS: [(FilterKind, &str); 4] = [
  (FilterKind::LowPass, "低通"),
  (FilterKind::HighPass, "高通"),
  (FilterKind::BandPass, "带通"),
  (FilterKind::BandStop, "带阻"),
];

#[component]
pub(super) fn FilterDesign() -> impl IntoView {
  let kind = RwSignal::new(FilterKind::LowPass);
  let order = RwSignal::new(3.0f64);
  let fc = RwSignal::new(7.1);
  let bw = RwSignal::new(0.5);
  let z0 = RwSignal::new(50.0);

  let stages = move || {
    let n = order.get().round().clamp(1.0, 5.0) as usize;
    let fc_hz = fc.get() * 1e6;
    let z = z0.get();
    match kind.get() {
      FilterKind::LowPass => design_lowpass(n, fc_hz, z),
      FilterKind::HighPass => design_highpass(n, fc_hz, z),
      FilterKind::BandPass => design_bandpass(n, fc_hz, bw.get() * 1e6, z),
      FilterKind::BandStop => design_bandstop(n, fc_hz, bw.get() * 1e6, z),
    }
  };

  let tab_class = move |active: bool| {
    if active {
      "rounded-md bg-primary px-2.5 py-1 text-xs font-medium text-primary-foreground"
    } else {
      "rounded-md px-2.5 py-1 text-xs font-medium text-muted-foreground hover:bg-accent"
    }
  };

  view! {
    <div class="grid gap-3">
      <div class="flex flex-wrap gap-1.5">
        {KINDS
          .iter()
          .map(|&(k, label)| {
            view! {
              <button
                type="button"
                class=move || tab_class(kind.get() == k)
                on:click=move |_| kind.set(k)
              >
                {move || t(label)}
              </button>
            }
          })
          .collect_view()}
      </div>

      <div class="grid gap-3 sm:grid-cols-2">
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("阶数（1–5）")}</span>
          <input
            type="number"
            min="1"
            max="5"
            prop:value=move || order.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                order.set(v.clamp(1.0, 5.0));
              }
            }
            class=INPUT
          />
        </label>
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || {
            if kind.get() == FilterKind::LowPass || kind.get() == FilterKind::HighPass {
              t("截止频率（MHz）")
            } else {
              t("中心频率（MHz）")
            }
          }}</span>
          <input
            type="number"
            prop:value=move || fc.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                fc.set(v);
              }
            }
            class=INPUT
          />
        </label>
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("特性阻抗（Ω）")}</span>
          <input
            type="number"
            prop:value=move || z0.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                z0.set(v);
              }
            }
            class=INPUT
          />
        </label>
        {move || {
          (kind.get() == FilterKind::BandPass || kind.get() == FilterKind::BandStop).then(|| {
            view! {
              <label class="flex flex-col gap-1.5 text-sm">
                <span class="text-xs text-muted-foreground">{move || t("带宽（MHz）")}</span>
                <input
                  type="number"
                  prop:value=move || bw.get().to_string()
                  on:input=move |e| {
                    if let Ok(v) = event_target_value(&e).parse::<f64>() {
                      bw.set(v);
                    }
                  }
                  class=INPUT
                />
              </label>
            }
          })
        }}
      </div>

      <div class="overflow-x-auto rounded-lg border">
        <table class="w-full min-w-[480px] border-collapse text-sm">
          <thead class="bg-muted/60 text-xs">
            <tr>
              <th class="border px-3 py-2 text-left">{move || t("级")}</th>
              <th class="border px-3 py-2 text-left">{move || t("位置")}</th>
              <th class="border px-3 py-2 text-left">{move || t("臂内接法")}</th>
              <th class="border px-3 py-2 text-left">{move || t("电感（μH）")}</th>
              <th class="border px-3 py-2 text-left">{move || t("电容（pF）")}</th>
            </tr>
          </thead>
          <tbody>
            {move || {
              stages()
                .iter()
                .enumerate()
                .map(|(i, &s)| {
                  view! {
                    <tr class="border-t">
                      <td class="border px-3 py-2 font-mono tabular-nums">{i + 1}</td>
                      <td class="border px-3 py-2 text-muted-foreground">
                        {move || if s.series { t("串联") } else { t("并联") }}
                      </td>
                      <td class="border px-3 py-2 text-muted-foreground">
                        {move || {
                          match s.topology {
                            ResonatorTopology::Single => "—".to_string(),
                            ResonatorTopology::SeriesLc => t("串联 LC"),
                            ResonatorTopology::ParallelLc => t("并联 LC"),
                          }
                        }}
                      </td>
                      <td class="border px-3 py-2 font-mono tabular-nums">
                        {move || s.l_uh.map(|v| tf("{} μH", &[&fmt_num(v)])).unwrap_or_else(|| "—".into())}
                      </td>
                      <td class="border px-3 py-2 font-mono tabular-nums">
                        {move || s.c_pf.map(|v| tf("{} pF", &[&fmt_num(v)])).unwrap_or_else(|| "—".into())}
                      </td>
                    </tr>
                  }
                })
                .collect_view()
            }}
          </tbody>
        </table>
      </div>

      <p class="text-xs text-muted-foreground">
        {move || t("Butterworth 原型；g₁ 起交替串 / 并联。带通 / 带阻每级为 LC 谐振回路、谐振于中心频率。臂内接法决定阻带还是通带：串联臂内串联 LC 或并联臂内并联 LC → 谐振时直通（带通）；串联臂内并联 LC 或并联臂内串联 LC → 谐振时阻断（带阻）。")}
      </p>
    </div>
  }
}
