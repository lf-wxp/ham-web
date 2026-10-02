use leptos::prelude::*;

use super::INPUT;
use crate::i18n::t;

/// 天线增益换算：dBi ↔ dBd（相差 2.15 dB）。
#[component]
pub(super) fn GainConversion() -> impl IntoView {
  let dbi = RwSignal::new(2.15);
  let dbd = RwSignal::new(0.0);

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("增益 dBi")}</span>
        <input
          type="number"
          prop:value=move || dbi.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              dbi.set(v);
              dbd.set(v - 2.15);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("增益 dBd")}</span>
        <input
          type="number"
          prop:value=move || dbd.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              dbd.set(v);
              dbi.set(v + 2.15);
            }
          }
          class=INPUT
        />
      </label>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
        {move || t("半波偶极天线相对各向同性天线的增益为 2.15 dBi，故 dBi = dBd + 2.15。")}
      </div>
    </div>
  }
}
