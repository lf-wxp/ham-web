use leptos::prelude::*;

use super::{RESULT, TAU, fmt_num};
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// LC 谐振频率：f(MHz) = 159.15 / √(L(μH) × C(pF))。
#[component]
pub(super) fn LcResonance() -> impl IntoView {
  let inductance = RwSignal::new(10.0);
  let capacitance = RwSignal::new(100.0);

  let l_id = unique_id("lc-resonance-l");
  let c_id = unique_id("lc-resonance-c");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
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
      <div class=RESULT>
        {move || {
          let lc = inductance.get() * capacitance.get();
          let f = if lc > 0.0 { (1000.0 / TAU) / lc.sqrt() } else { 0.0 };
          tf("tools.resonant-frequency-f-mhz", &[&(fmt_num(f)).to_string()])
        }}
      </div>
    </div>
  }
}
