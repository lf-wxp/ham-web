use ham_web_core::wire_gauge::{AWG_TABLE, copper_resistance_ohm, voltage_drop};
use leptos::prelude::*;

use super::fmt_num;
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 直流供电线径与压降估算。
#[component]
pub(super) fn WireGauge() -> impl IntoView {
  let amps = RwSignal::new(10.0);
  let len = RwSignal::new(5.0);
  let diameter = RwSignal::new(1.628);

  let amps_id = unique_id("wire-gauge-amps");
  let len_id = unique_id("wire-gauge-len");
  let diameter_id = unique_id("wire-gauge-diameter");

  view! {
    <div class="space-y-4">
      <div class="grid gap-3 sm:grid-cols-3">
        <Field label=Signal::derive(move || t("tools.current-a")) r#for=amps_id.clone()>
          <NumberField
            id=amps_id
            value=Signal::derive(move || amps.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                amps.set(v);
              }
            })
            controls=false
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.one-way-length-m")) r#for=len_id.clone()>
          <NumberField
            id=len_id
            value=Signal::derive(move || len.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                len.set(v);
              }
            })
            controls=false
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.conductor-diameter-mm")) r#for=diameter_id.clone()>
          <NumberField
            id=diameter_id
            value=Signal::derive(move || diameter.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                diameter.set(v);
              }
            })
            controls=false
          />
        </Field>
      </div>
      <div class="space-y-1 rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
        <div class="tabular-nums">
          {move || {
            let r = copper_resistance_ohm(len.get(), diameter.get());
            tf("tools.single-conductor-resistance-2", &[&fmt_num(r)])
          }}
        </div>
        <div class="tabular-nums">
          {move || {
            let vd = voltage_drop(amps.get(), len.get(), diameter.get());
            tf("tools.loop-voltage-drop-v", &[&fmt_num(vd)])
          }}
        </div>
      </div>

      <div>
        <div class="mb-1.5 text-xs text-muted-foreground">{move || t("tools.common-awg-wire-gauges")}</div>
        <div class="overflow-x-auto">
          <table class="w-full border-collapse text-sm">
            <thead class="bg-muted/60 text-xs">
              <tr>
                <th class="border px-3 py-2 text-left">{move || t("common.awg")}</th>
                <th class="border px-3 py-2 text-left">{move || t("tools.diameter-mm")}</th>
                <th class="border px-3 py-2 text-left">{move || t("tools.cross-section-mm")}</th>
                <th class="border px-3 py-2 text-left">{move || t("common.ohm-per-100m")}</th>
              </tr>
            </thead>
            <tbody>
              {AWG_TABLE
                .iter()
                .map(|(awg, dia, area, ohm)| {
                  view! {
                    <tr class="border-t hover:bg-muted/40">
                      <td class="border px-3 py-2 tabular-nums">{*awg}</td>
                      <td class="border px-3 py-2 tabular-nums">{format!("{dia:.3}")}</td>
                      <td class="border px-3 py-2 tabular-nums">{format!("{area:.2}")}</td>
                      <td class="border px-3 py-2 tabular-nums">{format!("{ohm:.2}")}</td>
                    </tr>
                  }
                })
                .collect_view()}
            </tbody>
          </table>
        </div>
      </div>
      <p class="text-xs text-muted-foreground">
        {move || t("tools.current-ratings-depend-on")}
      </p>
    </div>
  }
}
