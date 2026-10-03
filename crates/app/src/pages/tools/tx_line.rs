use ham_web_core::tx_line::{coax_z0, microstrip_z0, twin_lead_z0};
use leptos::prelude::*;

use super::{INPUT, fmt_num};
use crate::i18n::{t, tf};

/// 传输线特性阻抗：同轴 / 平行双线 / 微带线。
#[component]
pub(super) fn TxLine() -> impl IntoView {
  let er = RwSignal::new(2.25);
  let d_outer = RwSignal::new(3.5);
  let d_inner = RwSignal::new(1.0);
  let spacing = RwSignal::new(6.1);
  let diameter = RwSignal::new(1.0);
  let width = RwSignal::new(1.9);
  let height = RwSignal::new(1.0);

  view! {
    <div class="space-y-4">
      <div class="grid gap-3 sm:grid-cols-3">
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("介质常数 εr")}</span>
          <input
            type="number"
            prop:value=move || er.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                er.set(v);
              }
            }
            class=INPUT
          />
        </label>
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("同轴外导体内径（mm）")}</span>
          <input
            type="number"
            prop:value=move || d_outer.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                d_outer.set(v);
              }
            }
            class=INPUT
          />
        </label>
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("同轴内导体外径（mm）")}</span>
          <input
            type="number"
            prop:value=move || d_inner.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                d_inner.set(v);
              }
            }
            class=INPUT
          />
        </label>
      </div>
      <div class="rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground tabular-nums">
        {move || {
          let z = coax_z0(er.get(), d_outer.get(), d_inner.get());
          if z.is_nan() {
            t("同轴线：请输入有效参数（εr>0 且外径>内径）。")
          } else {
            tf("同轴线特性阻抗 ≈ {} Ω", &[&fmt_num(z)])
          }
        }}
      </div>

      <div class="grid gap-3 sm:grid-cols-2">
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("平行双线中心距（mm）")}</span>
          <input
            type="number"
            prop:value=move || spacing.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                spacing.set(v);
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
      <div class="rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground tabular-nums">
        {move || {
          let z = twin_lead_z0(er.get(), spacing.get(), diameter.get());
          if z.is_nan() {
            t("平行双线：请输入有效参数（中心距大于直径）。")
          } else {
            tf("平行双线特性阻抗 ≈ {} Ω", &[&fmt_num(z)])
          }
        }}
      </div>

      <div class="grid gap-3 sm:grid-cols-2">
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("微带走线宽度（mm）")}</span>
          <input
            type="number"
            prop:value=move || width.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                width.set(v);
              }
            }
            class=INPUT
          />
        </label>
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("介质厚度（mm）")}</span>
          <input
            type="number"
            prop:value=move || height.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                height.set(v);
              }
            }
            class=INPUT
          />
        </label>
      </div>
      <div class="rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground tabular-nums">
        {move || {
          let z = microstrip_z0(er.get(), width.get(), height.get());
          if z.is_nan() {
            t("微带线：请输入有效参数（εr>1 且宽厚为正）。")
          } else {
            tf("微带线特性阻抗 ≈ {} Ω", &[&fmt_num(z)])
          }
        }}
      </div>
    </div>
  }
}
