use leptos::prelude::*;

use super::{RESULT, fmt_num};
use crate::i18n::{t, tp};
use crate::ui::{Field, Input};
use crate::util::unique_id;

/// 级联增益：多个 dB 相加（用逗号或空格分隔）。
#[component]
pub(super) fn CascadeGain() -> impl IntoView {
  let input = RwSignal::new("3, 6".to_owned());
  let input_id = unique_id("cascade-gain");

  view! {
    <div class="space-y-3">
      <Field
        label=Signal::derive(move || t("tools.stage-gains-db-separated"))
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
            .collect();
          let sum: f64 = values.iter().sum();
          tp(
            "tools.stages-cascaded-gain-db",
            values.len() as u32,
            &[
              &values.len().to_string(),
              &fmt_num(sum),
              &fmt_num(10f64.powf(sum / 10.0)),
            ],
          )
        }}
      </div>
    </div>
  }
}
