use ham_web_core::contest_score::{arrl_dx_score, cqww_score, wpx_score};
use leptos::prelude::*;

use super::INPUT;
use crate::i18n::{t, tf};

/// 竞赛记分器：CQ WPX / CQ WW / ARRL DX 分数计算。
#[component]
pub(super) fn ContestScorer() -> impl IntoView {
  let kind = RwSignal::new("WPX".to_owned());
  let qsos = RwSignal::new(100.0);
  let mult = RwSignal::new(30.0);
  let zones = RwSignal::new(20.0);

  view! {
    <div class="space-y-3">
      <div class="grid gap-3 sm:grid-cols-4">
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("竞赛")}</span>
          <select
            prop:value=move || kind.get()
            on:change=move |e| kind.set(event_target_value(&e))
            class=INPUT
          >
            <option value="WPX">"CQ WPX"</option>
            <option value="WW">"CQ WW"</option>
            <option value="ARRL">"ARRL DX"</option>
          </select>
        </label>
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("QSO 数")}</span>
          <input
            type="number"
            prop:value=move || qsos.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                qsos.set(v);
              }
            }
            class=INPUT
          />
        </label>
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">
            {move || {
              if kind.get() == "WPX" { t("前缀数") } else { t("实体数") }
            }}
          </span>
          <input
            type="number"
            prop:value=move || mult.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                mult.set(v);
              }
            }
            class=INPUT
          />
        </label>
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("CQ 分区数")}</span>
          <input
            type="number"
            prop:value=move || zones.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                zones.set(v);
              }
            }
            class=INPUT
          />
        </label>
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
          tf("总分 = {} 分", &[&total.to_string()])
        }}
      </div>
    </div>
  }
}
