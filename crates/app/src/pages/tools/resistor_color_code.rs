use leptos::prelude::*;

use super::{fmt_num, fmt_resistance};
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 电阻色环（4 环）：前两环数字 + 乘数 + 容差。
#[component]
pub(super) fn ResistorColorCode() -> impl IntoView {
  let ring1 = RwSignal::new(4.0);
  let ring2 = RwSignal::new(7.0);
  let mult = RwSignal::new(3.0);
  let tol = RwSignal::new(5.0);

  let ring1_id = unique_id("resistor-code-ring1");
  let ring2_id = unique_id("resistor-code-ring2");
  let mult_id = unique_id("resistor-code-mult");
  let tol_id = unique_id("resistor-code-tol");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.band-1-tens-0")) r#for=ring1_id.clone()>
        <NumberField
          id=ring1_id
          step=1.0
          min=0.0
          max=9.0
          value=Signal::derive(move || ring1.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              ring1.set(v.clamp(0.0, 9.0));
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.band-2-ones-0")) r#for=ring2_id.clone()>
        <NumberField
          id=ring2_id
          step=1.0
          min=0.0
          max=9.0
          value=Signal::derive(move || ring2.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              ring2.set(v.clamp(0.0, 9.0));
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.multiplier-power-of-10")) r#for=mult_id.clone()>
        <NumberField
          id=mult_id
          step=1.0
          value=Signal::derive(move || mult.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              mult.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.tolerance")) r#for=tol_id.clone()>
        <NumberField
          id=tol_id
          value=Signal::derive(move || tol.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              tol.set(v);
            }
          })
          controls=false
        />
      </Field>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let value = (ring1.get() * 10.0 + ring2.get()) * 10f64.powf(mult.get());
          tf(
            "tools.resistance",
            &[&fmt_resistance(value), &fmt_num(tol.get())],
          )
        }}
      </div>
      <p class="sm:col-span-2 text-xs text-muted-foreground">
        {move || t("tools.colour-code-black-0")}
      </p>
    </div>
  }
}
