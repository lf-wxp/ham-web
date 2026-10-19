//! 陷波器（trap）：两波段 trap 偶极子的并联 LC 设计。

use ham_web_core::trap;
use ham_web_core::unit_math::{format_distance_m, format_inductance_h};
use leptos::prelude::*;

use super::fmt_resistance;
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

#[component]
pub(super) fn TrapTool() -> impl IntoView {
  let f_high = RwSignal::new(String::from("14.175"));
  let f_low = RwSignal::new(String::from("7.05"));
  let cap = RwSignal::new(String::from("67"));

  let f_high_id = unique_id("trap-f-high");
  let f_low_id = unique_id("trap-f-low");
  let cap_id = unique_id("trap-c");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.trap-f-high")) hint=Signal::derive(move || t("tools.trap-f-high-hint")) r#for=f_high_id.clone()>
        <NumberField id=f_high_id.clone() value=Signal::derive(move || f_high.get()) on_change=Callback::new(move |v: String| f_high.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.trap-f-low")) r#for=f_low_id.clone()>
        <NumberField id=f_low_id.clone() value=Signal::derive(move || f_low.get()) on_change=Callback::new(move |v: String| f_low.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.trap-c")) hint=Signal::derive(move || t("tools.trap-c-hint")) r#for=cap_id.clone()>
        <NumberField id=cap_id.clone() value=Signal::derive(move || cap.get()) on_change=Callback::new(move |v: String| cap.set(v)) controls=false />
      </Field>

      <div class="sm:col-span-2 space-y-1 bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let (Ok(fh), Ok(fl), Ok(c)) = (
            f_high.get().trim().parse::<f64>(),
            f_low.get().trim().parse::<f64>(),
            cap.get().trim().parse::<f64>(),
          ) else {
            return view! { <p>{move || t("tools.trap-invalid")}</p> }.into_any();
          };
          match trap::trap_dipole(fh, fl, c) {
            None => view! { <p>{move || t("tools.trap-invalid")}</p> }.into_any(),
            Some(d) => {
              // 参数已由 `trap_dipole` 校验过，这里理论上取不到 `None`；兜底显示「—」
              // 也好过把 NaN 当电抗值印出来。
              let loading = trap::trap_reactance_ohm(d.l_uh, d.c_pf, fl * 1e6)
                .map_or_else(|| "—".to_owned(), fmt_resistance);
              view! {
                <p>{tf("tools.trap-result", &[&format_inductance_h(d.l_uh * 1e-6), &format_distance_m(d.inner_m), &format_distance_m(d.outer_m)])}</p>
                <p>{tf("tools.trap-loading", &[&loading])}</p>
              }
                .into_any()
            }
          }
        }}
      </div>
      <p class="sm:col-span-2 text-xs text-muted-foreground">{move || t("tools.trap-note")}</p>
    </div>
  }
}
