use leptos::prelude::*;

use super::fmt_resistance;
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// T 型对称衰减器：返回（串联臂电阻, 并联臂电阻）。
fn tee_attenuator(z0: f64, db: f64) -> Option<(f64, f64)> {
  if z0 <= 0.0 || db <= 0.0 {
    return None;
  }
  let k = 10f64.powf(db / 20.0);
  if k <= 1.0 {
    return None;
  }
  let r_series = z0 * (k - 1.0) / (k + 1.0);
  let r_shunt = 2.0 * z0 * k / (k * k - 1.0);
  Some((r_series, r_shunt))
}

/// π 型对称衰减器：返回（并联臂电阻, 串联臂电阻）。
fn pi_attenuator(z0: f64, db: f64) -> Option<(f64, f64)> {
  if z0 <= 0.0 || db <= 0.0 {
    return None;
  }
  let k = 10f64.powf(db / 20.0);
  if k <= 1.0 {
    return None;
  }
  let r_shunt = z0 * (k * k - 1.0) / (2.0 * k);
  let r_series = z0 * (k + 1.0) / (k - 1.0);
  Some((r_shunt, r_series))
}

/// T 型 / π 型衰减器计算。
#[component]
pub(super) fn AttenuatorCalculator() -> impl IntoView {
  let z0 = RwSignal::new(50.0);
  let db = RwSignal::new(10.0);

  let z0_id = unique_id("attenuator-z0");
  let db_id = unique_id("attenuator-db");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
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
      <Field label=Signal::derive(move || t("tools.attenuation-db")) r#for=db_id.clone()>
        <NumberField
          id=db_id
          value=Signal::derive(move || db.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              db.set(v);
            }
          })
          controls=false
        />
      </Field>
      <div class="sm:col-span-2 space-y-1 rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
        {move || {
          match (tee_attenuator(z0.get(), db.get()), pi_attenuator(z0.get(), db.get())) {
            (Some((t_series, t_shunt)), Some((p_shunt, p_series))) => view! {
              <div class="tabular-nums">
                {tf(
                  "tools.t-pad-two-arms",
                  &[&fmt_resistance(t_series), &fmt_resistance(t_shunt)],
                )}
              </div>
              <div class="tabular-nums">
                {tf(
                  "tools.pad-two-arms-2",
                  &[&fmt_resistance(p_shunt), &fmt_resistance(p_series)],
                )}
              </div>
            }
            .into_any(),
            _ => view! {
              <span>{move || t("tools.enter-positive-impedance-and")}</span>
            }
            .into_any(),
          }
        }}
      </div>
      <p class="sm:col-span-2 text-xs text-muted-foreground">
        {move || t("tools.symmetrical-t-resistive-attenuators")}
      </p>
    </div>
  }
}
