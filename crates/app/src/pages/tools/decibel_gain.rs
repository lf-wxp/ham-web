use ham_web_core::decibel::{
  COMMON_DB_STEPS, db_to_power_ratio, db_to_voltage_ratio, format_ratio, power_ratio_to_db,
};
use leptos::prelude::*;

use super::{RESULT, fmt_num};
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 分贝增益：dB ↔ 功率 / 电压倍数（功率比 10lg、电压比 20lg），附常用对照表。
#[component]
pub(super) fn DecibelGain() -> impl IntoView {
  let db = RwSignal::new(3.0);
  let power_ratio = RwSignal::new(db_to_power_ratio(3.0));

  let db_id = unique_id("decibel-gain-db");
  let ratio_id = unique_id("decibel-gain-ratio");

  view! {
    <div class="space-y-4">
      <div class="grid gap-3 sm:grid-cols-2">
        <Field label=Signal::derive(move || t("tools.gain-db")) r#for=db_id.clone()>
          <NumberField
            id=db_id
            value=Signal::derive(move || db.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(d) = v.trim().parse::<f64>() {
                db.set(d);
                power_ratio.set(db_to_power_ratio(d));
              }
            })
            controls=false
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.power-ratio")) r#for=ratio_id.clone()>
          <NumberField
            id=ratio_id
            value=Signal::derive(move || power_ratio.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(r) = v.trim().parse::<f64>() {
                let d = power_ratio_to_db(r);
                if d.is_finite() {
                  power_ratio.set(r);
                  db.set(d);
                }
              }
            })
            controls=false
          />
        </Field>
        <div class=RESULT>
          {move || {
            let d = db.get();
            tf(
              "tools.db-power-voltage",
              &[
                &fmt_num(d),
                &fmt_num(power_ratio.get()),
                &fmt_num(db_to_voltage_ratio(d)),
              ],
            )
          }}
        </div>
      </div>

      <div>
        <div class="mb-1.5 text-xs text-muted-foreground">
          {move || t("tools.common-db-ratios")}
        </div>
        <div class="overflow-x-auto">
          <table class="w-full border-collapse text-sm">
            <thead class="bg-muted/60 text-xs">
              <tr>
                <th class="border px-3 py-2 text-left">{move || t("tools.db")}</th>
                <th class="border px-3 py-2 text-left">{move || t("tools.power-ratio")}</th>
                <th class="border px-3 py-2 text-left">{move || t("tools.voltage-ratio")}</th>
              </tr>
            </thead>
            <tbody>
              {COMMON_DB_STEPS
                .iter()
                .map(|&d| {
                  let label = if d > 0.0 { format!("+{d:.0}") } else { format!("{d:.0}") };
                  view! {
                    <tr class="border-t hover:bg-muted/40">
                      <td class="border px-3 py-2 tabular-nums">{label}</td>
                      <td class="border px-3 py-2 tabular-nums">
                        {format_ratio(db_to_power_ratio(d))}
                      </td>
                      <td class="border px-3 py-2 tabular-nums">
                        {format_ratio(db_to_voltage_ratio(d))}
                      </td>
                    </tr>
                  }
                })
                .collect_view()}
            </tbody>
          </table>
        </div>
      </div>
      <p class="text-xs text-muted-foreground">{move || t("tools.db-ratio-note")}</p>
    </div>
  }
}
