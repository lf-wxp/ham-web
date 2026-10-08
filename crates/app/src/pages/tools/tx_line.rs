use ham_web_core::tx_line::{coax_z0, microstrip_z0, twin_lead_z0};
use leptos::prelude::*;

use super::fmt_num;
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

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

  // `Field` 的标签与控件是兄弟节点，`r#for` / `id` 必须配对才能点击标签聚焦输入框
  //（e2e 与读屏都按「标签 → 控件」的关联来定位）。
  let er_id = unique_id("tx-line-er");
  let d_outer_id = unique_id("tx-line-d-outer");
  let d_inner_id = unique_id("tx-line-d-inner");
  let spacing_id = unique_id("tx-line-spacing");
  let diameter_id = unique_id("tx-line-diameter");
  let width_id = unique_id("tx-line-width");
  let height_id = unique_id("tx-line-height");

  view! {
    <div class="space-y-4">
      <div class="grid gap-3 sm:grid-cols-3">
        <Field label=Signal::derive(move || t("tools.dielectric-constant-r")) r#for=er_id.clone()>
          <NumberField
            id=er_id
            value=Signal::derive(move || er.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                er.set(v);
              }
            })
            controls=false
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.coax-outer-conductor-id")) r#for=d_outer_id.clone()>
          <NumberField
            id=d_outer_id
            value=Signal::derive(move || d_outer.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                d_outer.set(v);
              }
            })
            controls=false
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.coax-inner-conductor-od")) r#for=d_inner_id.clone()>
          <NumberField
            id=d_inner_id
            value=Signal::derive(move || d_inner.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                d_inner.set(v);
              }
            })
            controls=false
          />
        </Field>
      </div>
      <div class="rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground tabular-nums">
        {move || {
          let z = coax_z0(er.get(), d_outer.get(), d_inner.get());
          if z.is_nan() {
            t("tools.coax-enter-valid-parameters")
          } else {
            tf("tools.coax-characteristic-impedance", &[&fmt_num(z)])
          }
        }}
      </div>

      <div class="grid gap-3 sm:grid-cols-2">
        <Field label=Signal::derive(move || t("tools.twin-lead-conductor-spacing")) r#for=spacing_id.clone()>
          <NumberField
            id=spacing_id
            value=Signal::derive(move || spacing.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                spacing.set(v);
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
      <div class="rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground tabular-nums">
        {move || {
          let z = twin_lead_z0(er.get(), spacing.get(), diameter.get());
          if z.is_nan() {
            t("tools.twin-lead-enter-valid")
          } else {
            tf("tools.twin-lead-characteristic-impedance", &[&fmt_num(z)])
          }
        }}
      </div>

      <div class="grid gap-3 sm:grid-cols-2">
        <Field label=Signal::derive(move || t("tools.microstrip-trace-width-mm")) r#for=width_id.clone()>
          <NumberField
            id=width_id
            value=Signal::derive(move || width.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                width.set(v);
              }
            })
            controls=false
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.substrate-thickness-mm")) r#for=height_id.clone()>
          <NumberField
            id=height_id
            value=Signal::derive(move || height.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                height.set(v);
              }
            })
            controls=false
          />
        </Field>
      </div>
      <div class="rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground tabular-nums">
        {move || {
          let z = microstrip_z0(er.get(), width.get(), height.get());
          if z.is_nan() {
            t("tools.microstrip-enter-valid-parameters")
          } else {
            tf("tools.microstrip-characteristic-impedance", &[&fmt_num(z)])
          }
        }}
      </div>
    </div>
  }
}
