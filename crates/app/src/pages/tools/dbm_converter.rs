use ham_web_core::decibel::{COMMON_DBM_STEPS, dbm_to_mw, format_mw, mw_to_dbm};
use leptos::prelude::*;

use super::fmt_num;
use crate::i18n::t;
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// dBm ↔ 功率（mW / W）换算：双向输入，附公式、计算示例与常用对照。
#[component]
pub(super) fn DbmConverter() -> impl IntoView {
  let dbm = RwSignal::new(30.0);
  let mw = RwSignal::new(1000.0);
  let watts = RwSignal::new(1.0);

  let dbm_id = unique_id("dbm-power-dbm");
  let mw_id = unique_id("dbm-power-mw");
  let watts_id = unique_id("dbm-power-w");

  view! {
    <div class="space-y-4">
      <p class="text-xs text-muted-foreground">{move || t("tools.dbm-mw-formula")}</p>

      <div class="grid gap-3 sm:grid-cols-3">
        <Field label=Signal::derive(move || t("tools.power-level-dbm")) r#for=dbm_id.clone()>
          <NumberField
            id=dbm_id
            value=Signal::derive(move || dbm.get().to_string())
            on_change=Callback::new(move |v: String| {
              // 只接受有限、为正的换算结果：`1e9 dBm`（inf）、`-9999 dBm`（0）这类输入会让
              // 三个框同时变成 `inf` / `0`，且回不来。
              if let Ok(d) = v.trim().parse::<f64>() {
                let m = dbm_to_mw(d);
                if m.is_finite() && m > 0.0 {
                  dbm.set(d);
                  mw.set(m);
                  watts.set(m / 1000.0);
                }
              }
            })
            controls=false
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.power-mw")) r#for=mw_id.clone()>
          <NumberField
            id=mw_id
            value=Signal::derive(move || mw.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(m) = v.trim().parse::<f64>()
                && m.is_finite()
                && m > 0.0
              {
                mw.set(m);
                dbm.set(mw_to_dbm(m));
                watts.set(m / 1000.0);
              }
            })
            controls=false
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.power-w")) r#for=watts_id.clone()>
          <NumberField
            id=watts_id
            value=Signal::derive(move || watts.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(w) = v.trim().parse::<f64>()
                && w > 0.0
                && (w * 1000.0).is_finite()
              {
                let m = w * 1000.0;
                watts.set(w);
                mw.set(m);
                dbm.set(mw_to_dbm(m));
              }
            })
            controls=false
          />
        </Field>
      </div>

      <div class="rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
        {move || {
          format!(
            "{} dBm = {} mW = {} W",
            fmt_num(dbm.get()),
            fmt_num(mw.get()),
            fmt_num(watts.get()),
          )
        }}
      </div>

      <p class="text-xs text-muted-foreground">{move || t("tools.dbm-mw-examples")}</p>

      <div>
        <div class="mb-1.5 text-xs text-muted-foreground">{move || t("tools.dbm-mw-reference")}</div>
        <div class="overflow-x-auto">
          <table class="w-full border-collapse text-sm">
            <thead class="bg-muted/60 text-xs">
              <tr>
                <th class="border px-3 py-2 text-left">{move || t("tools.dbm")}</th>
                <th class="border px-3 py-2 text-left">{move || t("tools.power-mw-w")}</th>
              </tr>
            </thead>
            <tbody>
              {COMMON_DBM_STEPS
                .iter()
                .map(|&d| {
                  view! {
                    <tr class="border-t hover:bg-muted/40">
                      <td class="border px-3 py-2 tabular-nums">{format!("{d:.0}")}</td>
                      <td class="border px-3 py-2 tabular-nums">{format_mw(dbm_to_mw(d))}</td>
                    </tr>
                  }
                })
                .collect_view()}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  }
}
