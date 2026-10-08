use leptos::prelude::*;

use super::{RESULT, fmt_num};
use crate::i18n::t;
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// dBm ↔ 功率换算。
#[component]
pub(super) fn DbmConverter() -> impl IntoView {
  let dbm = RwSignal::new(30.0);
  let watts = RwSignal::new(1.0);

  let dbm_id = unique_id("dbm-power-dbm");
  let watts_id = unique_id("dbm-power-w");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.power-level-dbm")) r#for=dbm_id.clone()>
        <NumberField
          id=dbm_id
          value=Signal::derive(move || dbm.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(d) = v.trim().parse::<f64>() {
              dbm.set(d);
              watts.set(10f64.powf((d - 30.0) / 10.0));
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
            {
              watts.set(w);
              dbm.set(30.0 + 10.0 * w.log10());
            }
          })
          controls=false
        />
      </Field>
      <div class=RESULT>
        {move || {
          let w = watts.get();
          format!(
            "{} dBm = {} W = {} mW",
            fmt_num(dbm.get()),
            fmt_num(w),
            fmt_num(w * 1000.0),
          )
        }}
      </div>
    </div>
  }
}
