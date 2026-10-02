use leptos::prelude::*;

use super::{INPUT, fmt_resistance};
use crate::i18n::{t, tf};

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

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("特性阻抗（Ω）")}</span>
        <input
          type="number"
          prop:value=move || z0.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              z0.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("衰减（dB）")}</span>
        <input
          type="number"
          prop:value=move || db.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              db.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <div class="sm:col-span-2 space-y-1 rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
        {move || {
          match (tee_attenuator(z0.get(), db.get()), pi_attenuator(z0.get(), db.get())) {
            (Some((t_series, t_shunt)), Some((p_shunt, p_series))) => view! {
              <div class="tabular-nums">
                {tf(
                  "T 型：两臂 {} × 2，并联 {}",
                  &[&fmt_resistance(t_series), &fmt_resistance(t_shunt)],
                )}
              </div>
              <div class="tabular-nums">
                {tf(
                  "π 型：两臂 {} × 2，串联 {}",
                  &[&fmt_resistance(p_shunt), &fmt_resistance(p_series)],
                )}
              </div>
            }
            .into_any(),
            _ => view! {
              <span>{move || t("请输入正的阻抗与衰减量。")}</span>
            }
            .into_any(),
          }
        }}
      </div>
      <p class="sm:col-span-2 text-xs text-muted-foreground">
        {move || t("对称 T / π 型电阻衰减器，用于把信号衰减指定 dB 并保持阻抗匹配；大功率应用需按衰减量留足电阻功率余量。")}
      </p>
    </div>
  }
}
