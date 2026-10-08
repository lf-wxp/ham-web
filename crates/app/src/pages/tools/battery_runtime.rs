use leptos::prelude::*;

use super::{RESULT, fmt_num};
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 电池续航估算：续航 = 容量 / 电流。
#[component]
pub(super) fn BatteryRuntime() -> impl IntoView {
  let capacity = RwSignal::new(2000.0);
  let current = RwSignal::new(500.0);

  let capacity_id = unique_id("battery-capacity");
  let current_id = unique_id("battery-current");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.battery-capacity-mah")) r#for=capacity_id.clone()>
        <NumberField
          id=capacity_id
          value=Signal::derive(move || capacity.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              capacity.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.device-current-ma")) r#for=current_id.clone()>
        <NumberField
          id=current_id
          value=Signal::derive(move || current.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              current.set(v);
            }
          })
          controls=false
        />
      </Field>
      <div class=RESULT>
        {move || {
          let i = current.get();
          if i <= 0.0 {
            t("tools.enter-a-positive-current")
          } else {
            tf("tools.runtime-hours", &[&(fmt_num(capacity.get() / i)).to_string()])
          }
        }}
      </div>
    </div>
  }
}
