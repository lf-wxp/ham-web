use ham_web_core::feedline::{FEEDLINE_SPECS, feedline_loss_db};
use leptos::prelude::*;

use super::{INPUT, fmt_num};
use crate::i18n::{t, tf};

/// 馈线损耗估算：按型号取速度因子与损耗数据，随频率按 √f 插值。
#[component]
pub(super) fn FeedlineLoss() -> impl IntoView {
  let kind = RwSignal::new(0usize);
  let freq = RwSignal::new(14.0);
  let length = RwSignal::new(30.0);

  let spec = move || FEEDLINE_SPECS[kind.get().min(FEEDLINE_SPECS.len() - 1)];
  let db = move || feedline_loss_db(&spec(), freq.get(), length.get());

  view! {
    <div class="grid gap-3 sm:grid-cols-3">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("馈线型号")}</span>
        <select
          class=INPUT
          on:change=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<usize>() {
              kind.set(v);
            }
          }
        >
          {FEEDLINE_SPECS
            .iter()
            .enumerate()
            .map(|(i, s)| {
              view! { <option value=i selected=move || kind.get() == i>{s.name}</option> }
            })
            .collect_view()}
        </select>
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("频率（MHz）")}</span>
        <input
          type="number"
          step="0.1"
          prop:value=move || freq.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              freq.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("馈线长度（m）")}</span>
        <input
          type="number"
          step="1"
          prop:value=move || length.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              length.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <div class="sm:col-span-3 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let d = db();
          let power_pct = (1.0 - 10f64.powf(-d / 10.0)) * 100.0;
          tf(
            "总损耗 {} dB　功率损耗 {}%　（{}，速度因子 {}）",
            &[
              &format!("{d:.2}"),
              &format!("{power_pct:.1}"),
              spec().name,
              &fmt_num(spec().velocity_factor),
            ],
          )
        }}
      </div>
    </div>
  }
}
