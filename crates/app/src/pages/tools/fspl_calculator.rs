use leptos::prelude::*;

use super::fmt_num;
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 自由空间路径损耗：FSPL(dB) = 20lg(d) + 20lg(f) + 32.45。
#[component]
pub(super) fn FsplCalculator() -> impl IntoView {
  let dist = RwSignal::new(100.0);
  let freq = RwSignal::new(145.0);

  let dist_id = unique_id("fspl-dist");
  let freq_id = unique_id("fspl-freq");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.distance-km")) r#for=dist_id.clone()>
        <NumberField
          id=dist_id
          value=Signal::derive(move || dist.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              dist.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("log.frequency-mhz")) r#for=freq_id.clone()>
        <NumberField
          id=freq_id
          value=Signal::derive(move || freq.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              freq.set(v);
            }
          })
          controls=false
        />
      </Field>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let (d, f) = (dist.get(), freq.get());
          if d <= 0.0 || f <= 0.0 {
            t("tools.enter-a-positive-distance")
          } else {
            let loss = 20.0 * d.log10() + 20.0 * f.log10() + 32.45;
            tf("tools.free-space-path-loss-2", &[&(fmt_num(loss)).to_string()])
          }
        }}
      </div>
    </div>
  }
}
