use leptos::prelude::*;

use super::fmt_num;
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 噪声系数级联（Friis 公式，3 级）。
#[component]
pub(super) fn NoiseCascade() -> impl IntoView {
  let nf1 = RwSignal::new(2.0);
  let g1 = RwSignal::new(15.0);
  let nf2 = RwSignal::new(6.0);
  let g2 = RwSignal::new(20.0);
  let nf3 = RwSignal::new(10.0);

  let nf1_id = unique_id("noise-cascade-nf1");
  let g1_id = unique_id("noise-cascade-g1");
  let nf2_id = unique_id("noise-cascade-nf2");
  let g2_id = unique_id("noise-cascade-g2");
  let nf3_id = unique_id("noise-cascade-nf3");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.stage-1-nf-db")) r#for=nf1_id.clone()>
        <NumberField
          id=nf1_id
          value=Signal::derive(move || nf1.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              nf1.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.stage-1-gain-db")) r#for=g1_id.clone()>
        <NumberField
          id=g1_id
          value=Signal::derive(move || g1.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              g1.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.stage-2-nf-db")) r#for=nf2_id.clone()>
        <NumberField
          id=nf2_id
          value=Signal::derive(move || nf2.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              nf2.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.stage-2-gain-db")) r#for=g2_id.clone()>
        <NumberField
          id=g2_id
          value=Signal::derive(move || g2.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              g2.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.stage-3-nf-db")) r#for=nf3_id.clone()>
        <NumberField
          id=nf3_id
          value=Signal::derive(move || nf3.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              nf3.set(v);
            }
          })
          controls=false
        />
      </Field>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let to_lin = |db: f64| 10f64.powf(db / 10.0);
          let f1 = to_lin(nf1.get());
          let f2 = to_lin(nf2.get());
          let f3 = to_lin(nf3.get());
          let g1l = to_lin(g1.get());
          let g2l = to_lin(g2.get());
          let f_total = f1 + (f2 - 1.0) / g1l + (f3 - 1.0) / (g1l * g2l);
          let nf_total = 10.0 * f_total.log10();
          tf("tools.total-noise-figure-db", &[&(fmt_num(nf_total)).to_string()])
        }}
      </div>
    </div>
  }
}
