use ham_web_core::contest_score::{arrl_dx_score, cqww_score, wpx_score};
use leptos::prelude::*;

use crate::i18n::{t, tp};
use crate::ui::{Field, NativeSelect, NumberField, SelectOption};
use crate::util::unique_id;

/// 竞赛记分器：CQ WPX / CQ WW / ARRL DX 分数计算。
#[component]
pub(super) fn ContestScorer() -> impl IntoView {
  let kind = RwSignal::new("WPX".to_owned());
  let qsos = RwSignal::new(100.0);
  let mult = RwSignal::new(30.0);
  let zones = RwSignal::new(20.0);

  let contest_options = vec![
    SelectOption::new("WPX", "CQ WPX"),
    SelectOption::new("WW", "CQ WW"),
    SelectOption::new("ARRL", "ARRL DX"),
  ];

  // `Field` 的标签与控件是兄弟节点，`r#for` / `id` 必须配对才能点击标签聚焦输入框
  //（e2e 与读屏都按「标签 → 控件」的关联来定位）。
  let kind_id = unique_id("contest-scorer-kind");
  let qsos_id = unique_id("contest-scorer-qsos");
  let mult_id = unique_id("contest-scorer-mult");
  let zones_id = unique_id("contest-scorer-zones");

  view! {
    <div class="space-y-3">
      <div class="grid gap-3 sm:grid-cols-4">
        <Field label=Signal::derive(move || t("contest.contest")) r#for=kind_id.clone()>
          <NativeSelect
            id=kind_id
            value=kind
            on_change=Callback::new(move |v: String| kind.set(v))
            options=contest_options
            aria_label=Signal::derive(move || t("contest.contest"))
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.qso-count")) r#for=qsos_id.clone()>
          <NumberField
            id=qsos_id
            value=Signal::derive(move || qsos.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                qsos.set(v);
              }
            })
            controls=false
          />
        </Field>
        <Field
          label=Signal::derive(move || {
            if kind.get() == "WPX" { t("tools.prefixes") } else { t("tools.entities") }
          })
          r#for=mult_id.clone()
        >
          <NumberField
            id=mult_id
            value=Signal::derive(move || mult.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                mult.set(v);
              }
            })
            controls=false
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.cq-zones")) r#for=zones_id.clone()>
          <NumberField
            id=zones_id
            value=Signal::derive(move || zones.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                zones.set(v);
              }
            })
            controls=false
          />
        </Field>
      </div>
      <div class="rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums">
        {move || {
          let q = qsos.get().max(0.0) as u32;
          let m = mult.get().max(0.0) as u32;
          let z = zones.get().max(0.0) as u32;
          let total = match kind.get().as_str() {
            "WW" => cqww_score(q, z, m),
            "ARRL" => arrl_dx_score(q, m),
            _ => wpx_score(q, m),
          };
          tp("tools.total-points", total, &[&total.to_string()])
        }}
      </div>
    </div>
  }
}
