use leptos::prelude::*;

use super::{RESULT, fmt_num};
use crate::i18n::t;
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 频率单位换算：以 MHz 为基准换算到 Hz / kHz / GHz。
#[component]
pub(super) fn FrequencyUnits() -> impl IntoView {
  let mhz = RwSignal::new(145.0);
  let mhz_id = unique_id("freq-units-mhz");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("log.frequency-mhz")) r#for=mhz_id.clone()>
        <NumberField
          id=mhz_id
          value=Signal::derive(move || mhz.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              mhz.set(v);
            }
          })
          controls=false
        />
      </Field>
      <div class=RESULT>
        {move || {
          let m = mhz.get();
          format!(
            "{} Hz　{} kHz　{} MHz　{} GHz",
            fmt_num(m * 1e6),
            fmt_num(m * 1e3),
            fmt_num(m),
            fmt_num(m / 1e3),
          )
        }}
      </div>
    </div>
  }
}
