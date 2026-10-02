use leptos::prelude::*;

use super::{INPUT, TAU, fmt_num};
use crate::i18n::{t, tf};

/// 容抗 / 感抗：Xc = 159155 / (f(MHz)·C(pF)) Ω，XL = 6.283·f(MHz)·L(μH) Ω。
#[component]
pub(super) fn Reactance() -> impl IntoView {
  let freq = RwSignal::new(7.0);
  let capacitance = RwSignal::new(100.0);
  let inductance = RwSignal::new(10.0);

  view! {
    <div class="grid gap-3 sm:grid-cols-3">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("频率 f（MHz）")}</span>
        <input
          type="number"
          prop:value=move || freq.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              freq.set(v);
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
      <div class="sm:col-span-3 rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
        {move || {
          let f = freq.get();
          let fc = f * capacitance.get();
          let xc = if fc > 0.0 { (1_000_000.0 / TAU) / fc } else { 0.0 };
          let xl = TAU * f * inductance.get();
          tf("容抗 Xc = {} Ω，感抗 XL = {} Ω", &[&(fmt_num(xc)).to_string(), &(fmt_num(xl)).to_string()])
        }}
      </div>
    </div>
  }
}
