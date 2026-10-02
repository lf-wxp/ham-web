use leptos::prelude::*;

use super::{INPUT, RESULT, TAU, fmt_num};
use crate::i18n::{t, tf};

/// LC 谐振频率：f(MHz) = 159.15 / √(L(μH) × C(pF))。
#[component]
pub(super) fn LcResonance() -> impl IntoView {
  let inductance = RwSignal::new(10.0);
  let capacitance = RwSignal::new(100.0);

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("电感 L（μH）")}</span>
        <input
          type="number"
          prop:value=move || inductance.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              inductance.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("电容 C（pF）")}</span>
        <input
          type="number"
          prop:value=move || capacitance.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              capacitance.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <div class=RESULT>
        {move || {
          let lc = inductance.get() * capacitance.get();
          let f = if lc > 0.0 { (1000.0 / TAU) / lc.sqrt() } else { 0.0 };
          tf("谐振频率 f = {} MHz", &[&(fmt_num(f)).to_string()])
        }}
      </div>
    </div>
  }
}
