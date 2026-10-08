use leptos::prelude::*;

use super::{TAU, fmt_num};
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 容抗 / 感抗：Xc = 159155 / (f(MHz)·C(pF)) Ω，XL = 6.283·f(MHz)·L(μH) Ω。
#[component]
pub(super) fn Reactance() -> impl IntoView {
  let freq = RwSignal::new(7.0);
  let capacitance = RwSignal::new(100.0);
  let inductance = RwSignal::new(10.0);

  let freq_id = unique_id("reactance-freq");
  let c_id = unique_id("reactance-c");
  let l_id = unique_id("reactance-l");

  view! {
    <div class="grid gap-3 sm:grid-cols-3">
      <Field label=Signal::derive(move || t("tools.frequency-f-mhz")) r#for=freq_id.clone()>
        <NumberField
          id=freq_id
          value=Signal::derive(move || freq.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              freq.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.capacitance-c-pf")) r#for=c_id.clone()>
        <NumberField
          id=c_id
          value=Signal::derive(move || capacitance.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              capacitance.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.inductance-l-h")) r#for=l_id.clone()>
        <NumberField
          id=l_id
          value=Signal::derive(move || inductance.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              inductance.set(v);
            }
          })
          controls=false
        />
      </Field>
      <div class="sm:col-span-3 rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
        {move || {
          let f = freq.get();
          let fc = f * capacitance.get();
          let xc = if fc > 0.0 { (1_000_000.0 / TAU) / fc } else { 0.0 };
          let xl = TAU * f * inductance.get();
          tf("tools.xc-xl", &[&(fmt_num(xc)).to_string(), &(fmt_num(xl)).to_string()])
        }}
      </div>
    </div>
  }
}
