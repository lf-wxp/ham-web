use ham_web_core::feedline::{FEEDLINE_SPECS, feedline_loss_db};
use leptos::prelude::*;

use super::fmt_num;
use crate::i18n::{t, tf};
use crate::ui::{Field, NativeSelect, NumberField, SelectOption};
use crate::util::unique_id;

/// 馈线损耗估算：按型号取速度因子与损耗数据，随频率按 √f 插值。
#[component]
pub(super) fn FeedlineLoss() -> impl IntoView {
  let kind = RwSignal::new(0usize);
  let freq = RwSignal::new(14.0);
  let length = RwSignal::new(30.0);

  let spec = move || FEEDLINE_SPECS[kind.get().min(FEEDLINE_SPECS.len() - 1)];
  let db = move || feedline_loss_db(&spec(), freq.get(), length.get());

  let model_options: Vec<SelectOption> = FEEDLINE_SPECS
    .iter()
    .enumerate()
    .map(|(i, s)| SelectOption::new(i.to_string(), s.name))
    .collect();

  let model_id = unique_id("feedline-model");
  let freq_id = unique_id("feedline-freq");
  let length_id = unique_id("feedline-length");

  view! {
    <div class="grid gap-3 sm:grid-cols-3">
      <Field label=Signal::derive(move || t("tools.feedline-model")) r#for=model_id.clone()>
        <NativeSelect
          id=model_id
          value=Signal::derive(move || kind.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(n) = v.parse::<usize>() {
              kind.set(n);
            }
          })
          options=model_options
          aria_label=Signal::derive(move || t("tools.feedline-model"))
        />
      </Field>
      <Field label=Signal::derive(move || t("log.frequency-mhz")) r#for=freq_id.clone()>
        <NumberField
          id=freq_id
          step=0.1
          value=Signal::derive(move || freq.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              freq.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.feedline-length-m")) r#for=length_id.clone()>
        <NumberField
          id=length_id
          step=1.0
          value=Signal::derive(move || length.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              length.set(v);
            }
          })
          controls=false
        />
      </Field>
      <div class="sm:col-span-3 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let d = db();
          let power_pct = (1.0 - 10f64.powf(-d / 10.0)) * 100.0;
          tf(
            "tools.total-loss-db-power-2",
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
