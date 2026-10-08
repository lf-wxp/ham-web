use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 天线增益换算：dBi ↔ dBd（相差 2.15 dB）。
#[component]
pub(super) fn GainConversion() -> impl IntoView {
  let dbi = RwSignal::new(2.15);
  let dbd = RwSignal::new(0.0);

  let dbi_id = unique_id("gain-conversion-dbi");
  let dbd_id = unique_id("gain-conversion-dbd");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.gain-dbi")) r#for=dbi_id.clone()>
        <NumberField
          id=dbi_id
          value=Signal::derive(move || dbi.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              dbi.set(v);
              dbd.set(v - 2.15);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.gain-dbd")) r#for=dbd_id.clone()>
        <NumberField
          id=dbd_id
          value=Signal::derive(move || dbd.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              dbd.set(v);
              dbi.set(v + 2.15);
            }
          })
          controls=false
        />
      </Field>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
        {move || t("tools.a-half-wave-dipole")}
      </div>
    </div>
  }
}
