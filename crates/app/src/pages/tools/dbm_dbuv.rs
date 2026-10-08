use leptos::prelude::*;

use super::{RESULT, fmt_num};
use crate::i18n::t;
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// dBm ↔ dBμV 换算（50Ω 阻抗）。
#[component]
pub(super) fn DbmDbuv() -> impl IntoView {
  let dbm = RwSignal::new(0.0);
  let dbm_id = unique_id("dbm-dbuv-dbm");
  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.power-level-dbm")) r#for=dbm_id.clone()>
        <NumberField
          id=dbm_id
          value=Signal::derive(move || dbm.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              dbm.set(v);
            }
          })
          controls=false
        />
      </Field>
      <div class=RESULT>
        {move || format!("{} dBμV（50Ω）", fmt_num(dbm.get() + 107.0))}
      </div>
    </div>
  }
}
