use ham_web_core::oscillator::{crystal_pull_ppm, series_resonance_hz};
use leptos::prelude::*;

use super::fmt_num;
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 晶体振荡器：频率牵引与串联谐振。
#[component]
pub(super) fn Oscillator() -> impl IntoView {
  let c1 = RwSignal::new(20.0);
  let c0 = RwSignal::new(5.0);
  let cl = RwSignal::new(20.0);
  let l = RwSignal::new(1.0);
  let c = RwSignal::new(1000.0);

  // `Field` 的标签与控件是兄弟节点，`r#for` / `id` 必须配对才能点击标签聚焦输入框
  //（e2e 与读屏都按「标签 → 控件」的关联来定位）。
  let c1_id = unique_id("oscillator-c1");
  let c0_id = unique_id("oscillator-c0");
  let cl_id = unique_id("oscillator-cl");
  let l_id = unique_id("oscillator-l");
  let c_id = unique_id("oscillator-c");

  view! {
    <div class="space-y-4">
      <div class="grid gap-3 sm:grid-cols-3">
        <Field label=Signal::derive(move || t("tools.motional-capacitance-c1-ff")) r#for=c1_id.clone()>
          <NumberField
            id=c1_id
            value=Signal::derive(move || c1.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                c1.set(v);
              }
            })
            controls=false
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.shunt-capacitance-c0-pf")) r#for=c0_id.clone()>
          <NumberField
            id=c0_id
            value=Signal::derive(move || c0.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                c0.set(v);
              }
            })
            controls=false
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.load-capacitance-cl-pf")) r#for=cl_id.clone()>
          <NumberField
            id=cl_id
            value=Signal::derive(move || cl.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                cl.set(v);
              }
            })
            controls=false
          />
        </Field>
      </div>
      <div class="rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground tabular-nums">
        {move || {
          let ppm = crystal_pull_ppm(c1.get() * 1e-15, c0.get() * 1e-12, cl.get() * 1e-12);
          if ppm.is_nan() {
            t("tools.pulling-enter-positive-c1")
          } else {
            tf("tools.frequency-pulling-f-f", &[&fmt_num(ppm)])
          }
        }}
      </div>

      <div class="grid gap-3 sm:grid-cols-2">
        <Field label=Signal::derive(move || t("tools.inductance-h-2")) r#for=l_id.clone()>
          <NumberField
            id=l_id
            value=Signal::derive(move || l.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                l.set(v);
              }
            })
            controls=false
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.capacitance-pf")) r#for=c_id.clone()>
          <NumberField
            id=c_id
            value=Signal::derive(move || c.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                c.set(v);
              }
            })
            controls=false
          />
        </Field>
      </div>
      <div class="rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground tabular-nums">
        {move || {
          let f = series_resonance_hz(l.get() * 1e-6, c.get() * 1e-12);
          if f.is_nan() {
            t("tools.series-resonance-enter-positive")
          } else {
            tf("tools.series-resonance-f-mhz", &[&fmt_num(f / 1e6)])
          }
        }}
      </div>
    </div>
  }
}
