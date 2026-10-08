//! 滤波器设计：Butterworth 低通 / 高通 / 带通 / 带阻元件值计算。

use ham_web_core::filter_design::{
  FilterKind, ResonatorTopology, design_bandpass, design_bandstop, design_highpass, design_lowpass,
};
use leptos::prelude::*;

use super::fmt_num;
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField, RadioGroup, RadioGroupItem};
use crate::util::unique_id;

/// 滤波器类型：`(枚举值, 单选值, 文案 key)`。
///
/// 单选值是与枚举一一对应的稳定字符串 —— `RadioGroup` 对外只有 `String`，
/// 用枚举名而不是下标，重排选项时不会把用户的选择换掉。
const KINDS: [(FilterKind, &str, &str); 4] = [
  (FilterKind::LowPass, "low-pass", "低通"),
  (FilterKind::HighPass, "high-pass", "高通"),
  (FilterKind::BandPass, "band-pass", "带通"),
  (FilterKind::BandStop, "band-stop", "带阻"),
];

#[component]
pub(super) fn FilterDesign() -> impl IntoView {
  let kind = RwSignal::new(FilterKind::LowPass);
  let order = RwSignal::new(3.0f64);
  let fc = RwSignal::new(7.1);
  let bw = RwSignal::new(0.5);
  let z0 = RwSignal::new(50.0);

  // `Field` 的标签与控件是兄弟节点，`r#for` / `id` 必须配对才能点击标签聚焦输入框
  //（e2e 与读屏都按「标签 → 控件」的关联来定位）。
  let order_id = unique_id("filter-order");
  let fc_id = unique_id("filter-fc");
  let z0_id = unique_id("filter-z0");
  let bw_id = unique_id("filter-bw");

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

  view! {
    <div class="grid gap-3">
      <RadioGroup
        value=Signal::derive(move || {
          KINDS
            .iter()
            .find(|(k, _, _)| *k == kind.get())
            .map_or("low-pass", |(_, value, _)| *value)
            .to_string()
        })
        on_change=Callback::new(move |v: String| {
          if let Some((k, _, _)) = KINDS.iter().find(|(_, value, _)| *value == v) {
            kind.set(*k);
          }
        })
        class="flex flex-wrap items-center gap-4"
      >
        {KINDS
          .iter()
          .map(|&(_, value, label)| {
            let id = unique_id("filter-kind");
            view! {
              <label class="flex cursor-pointer items-center gap-2 text-sm">
                <RadioGroupItem value=value id=id />
                {move || t(label)}
              </label>
            }
          })
          .collect_view()}
      </RadioGroup>

      <div class="grid gap-3 sm:grid-cols-2">
        <Field label=Signal::derive(move || t("tools.order-1-5")) r#for=order_id.clone()>
          <NumberField
            id=order_id
            step=1.0
            min=1.0
            max=5.0
            value=Signal::derive(move || order.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                order.set(v.clamp(1.0, 5.0));
              }
            })
            controls=false
          />
        </Field>
        <Field
          label=Signal::derive(move || {
            if kind.get() == FilterKind::LowPass || kind.get() == FilterKind::HighPass {
              t("tools.cut-off-frequency-mhz")
            } else {
              t("tools.centre-frequency-mhz")
            }
          })
          r#for=fc_id.clone()
        >
          <NumberField
            id=fc_id
            value=Signal::derive(move || fc.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                fc.set(v);
              }
            })
            controls=false
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.characteristic-impedance")) r#for=z0_id.clone()>
          <NumberField
            id=z0_id
            value=Signal::derive(move || z0.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                z0.set(v);
              }
            })
            controls=false
          />
        </Field>
        {move || {
          (kind.get() == FilterKind::BandPass || kind.get() == FilterKind::BandStop)
            .then(|| {
              let bw_id = bw_id.clone();
              view! {
                <Field label=Signal::derive(move || t("tools.bandwidth-mhz")) r#for=bw_id.clone()>
                  <NumberField
                    id=bw_id
                    value=Signal::derive(move || bw.get().to_string())
                    on_change=Callback::new(move |v: String| {
                      if let Ok(v) = v.trim().parse::<f64>() {
                        bw.set(v);
                      }
                    })
                    controls=false
                  />
                </Field>
              }
            })
        }}
      </div>

      <div class="overflow-x-auto rounded-lg border">
        <table class="w-full min-w-[480px] border-collapse text-sm">
          <thead class="bg-muted/60 text-xs">
            <tr>
              <th class="border px-3 py-2 text-left">{move || t("tools.stage")}</th>
              <th class="border px-3 py-2 text-left">{move || t("tools.position")}</th>
              <th class="border px-3 py-2 text-left">{move || t("tools.in-arm-configuration")}</th>
              <th class="border px-3 py-2 text-left">{move || t("tools.inductance-h-2")}</th>
              <th class="border px-3 py-2 text-left">{move || t("tools.capacitance-pf")}</th>
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
                        {move || if s.series { t("tools.series") } else { t("tools.shunt") }}
                      </td>
                      <td class="border px-3 py-2 text-muted-foreground">
                        {move || {
                          match s.topology {
                            ResonatorTopology::Single => "—".to_string(),
                            ResonatorTopology::SeriesLc => t("tools.series-lc"),
                            ResonatorTopology::ParallelLc => t("tools.parallel-lc"),
                          }
                        }}
                      </td>
                      <td class="border px-3 py-2 font-mono tabular-nums">
                        {move || s.l_uh.map(|v| tf("common.microhenry", &[&fmt_num(v)])).unwrap_or_else(|| "—".into())}
                      </td>
                      <td class="border px-3 py-2 font-mono tabular-nums">
                        {move || s.c_pf.map(|v| tf("common.picofarad", &[&fmt_num(v)])).unwrap_or_else(|| "—".into())}
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
        {move || t("tools.butterworth-prototype-g-onward")}
      </p>
    </div>
  }
}
