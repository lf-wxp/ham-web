use ham_web_core::wire_gauge::{AWG_TABLE, copper_resistance_ohm, voltage_drop};
use leptos::prelude::*;

use super::{INPUT, fmt_num};
use crate::i18n::{t, tf};

/// 直流供电线径与压降估算。
#[component]
pub(super) fn WireGauge() -> impl IntoView {
  let amps = RwSignal::new(10.0);
  let len = RwSignal::new(5.0);
  let diameter = RwSignal::new(1.628);

  view! {
    <div class="space-y-4">
      <div class="grid gap-3 sm:grid-cols-3">
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("电流（A）")}</span>
          <input
            type="number"
            prop:value=move || amps.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                amps.set(v);
              }
            }
            class=INPUT
          />
        </label>
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("单程长度（m）")}</span>
          <input
            type="number"
            prop:value=move || len.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                len.set(v);
              }
            }
            class=INPUT
          />
        </label>
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("导线直径（mm）")}</span>
          <input
            type="number"
            prop:value=move || diameter.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                diameter.set(v);
              }
            }
            class=INPUT
          />
        </label>
      </div>
      <div class="space-y-1 rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
        <div class="tabular-nums">
          {move || {
            let r = copper_resistance_ohm(len.get(), diameter.get());
            tf("单根导线电阻 {} Ω（回路 ×2）", &[&fmt_num(r)])
          }}
        </div>
        <div class="tabular-nums">
          {move || {
            let vd = voltage_drop(amps.get(), len.get(), diameter.get());
            tf("回路压降 {} V", &[&fmt_num(vd)])
          }}
        </div>
      </div>

      <div>
        <div class="mb-1.5 text-xs text-muted-foreground">{move || t("常用 AWG 线规参考")}</div>
        <div class="overflow-x-auto">
          <table class="w-full border-collapse text-sm">
            <thead class="bg-muted/60 text-xs">
              <tr>
                <th class="border px-3 py-2 text-left">{move || t("AWG")}</th>
                <th class="border px-3 py-2 text-left">{move || t("直径 mm")}</th>
                <th class="border px-3 py-2 text-left">{move || t("截面积 mm²")}</th>
                <th class="border px-3 py-2 text-left">{move || t("Ω/100m")}</th>
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
        {move || t("载流能力取决于绝缘与敷设方式，表中仅给出电阻参考；大电流供电建议压降控制在 3% 以内。")}
      </p>
    </div>
  }
}
