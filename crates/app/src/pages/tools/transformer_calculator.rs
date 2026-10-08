use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 变压器阻抗变换：匝数比 Np:Ns = √(Zp/Zs)。
#[component]
pub(super) fn TransformerCalculator() -> impl IntoView {
  let zp = RwSignal::new(200.0);
  let zs = RwSignal::new(50.0);

  let zp_id = unique_id("transformer-zp");
  let zs_id = unique_id("transformer-zs");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.primary-impedance-zp")) r#for=zp_id.clone()>
        <NumberField
          id=zp_id
          value=Signal::derive(move || zp.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              zp.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.secondary-impedance-zs")) r#for=zs_id.clone()>
        <NumberField
          id=zs_id
          value=Signal::derive(move || zs.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              zs.set(v);
            }
          })
          controls=false
        />
      </Field>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let p = zp.get();
          let s = zs.get();
          if p <= 0.0 || s <= 0.0 {
            t("tools.enter-positive-primary-and")
          } else {
            let n = (p / s).sqrt();
            tf(
              "tools.turns-ratio-np-ns",
              &[&format!("{n:.3}"), &format!("{:.3}", p / s)],
            )
          }
        }}
      </div>
      <p class="sm:col-span-2 text-xs text-muted-foreground">
        {move || t("tools.an-ideal-transformer-s")}
      </p>
    </div>
  }
}
