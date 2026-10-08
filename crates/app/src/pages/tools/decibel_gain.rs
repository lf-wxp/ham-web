use leptos::prelude::*;

use super::{RESULT, fmt_num};
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 分贝增益：dB ↔ 功率 / 电压倍数（功率比 10lg、电压比 20lg）。
#[component]
pub(super) fn DecibelGain() -> impl IntoView {
  let db = RwSignal::new(3.0);
  let power_ratio = RwSignal::new(10f64.powf(3.0 / 10.0));

  let db_id = unique_id("decibel-gain-db");
  let ratio_id = unique_id("decibel-gain-ratio");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.gain-db")) r#for=db_id.clone()>
        <NumberField
          id=db_id
          value=Signal::derive(move || db.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(d) = v.trim().parse::<f64>() {
              db.set(d);
              power_ratio.set(10f64.powf(d / 10.0));
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.power-ratio")) r#for=ratio_id.clone()>
        <NumberField
          id=ratio_id
          value=Signal::derive(move || power_ratio.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(r) = v.trim().parse::<f64>()
              && r > 0.0
            {
              power_ratio.set(r);
              db.set(10.0 * r.log10());
            }
          })
          controls=false
        />
      </Field>
      <div class=RESULT>
        {move || {
          let d = db.get();
          tf(
            "tools.db-power-voltage",
            &[
              &fmt_num(d),
              &fmt_num(power_ratio.get()),
              &fmt_num(10f64.powf(d / 20.0)),
            ],
          )
        }}
      </div>
    </div>
  }
}
