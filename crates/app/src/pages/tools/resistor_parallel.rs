use leptos::prelude::*;

use super::{RESULT, fmt_num};
use crate::i18n::{t, tf};
use crate::ui::{Field, Input};
use crate::util::unique_id;

/// 电阻串并联：输入若干电阻，计算串联与并联等效（Ω）。
#[component]
pub(super) fn ResistorParallel() -> impl IntoView {
  let input = RwSignal::new("100, 100".to_owned());
  let input_id = unique_id("resistor-parallel");

  view! {
    <div class="space-y-3">
      <Field
        label=Signal::derive(move || t("tools.resistance-values-separated-by"))
        r#for=input_id.clone()
      >
        <Input id=input_id value=input on_change=Callback::new(move |v: String| input.set(v)) />
      </Field>
      <div class=RESULT>
        {move || {
          let values: Vec<f64> = input
            .get()
            .split([',', '，', ' '])
            .filter_map(|s| s.trim().parse().ok())
            .filter(|v| *v > 0.0)
            .collect();
          if values.is_empty() {
            t("tools.enter-resistance-values")
          } else {
            let series: f64 = values.iter().sum();
            let parallel = 1.0 / values.iter().map(|v| 1.0 / v).sum::<f64>();
            tf(
              "tools.series-parallel",
              &[&fmt_num(series), &fmt_num(parallel)],
            )
          }
        }}
      </div>
    </div>
  }
}
