use leptos::prelude::*;

use super::fmt_num;
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 接收机灵敏度：S(dBm) = -174 + 10lg(BW) + NF。
#[component]
pub(super) fn ReceiverSensitivity() -> impl IntoView {
  let bw = RwSignal::new(2700.0);
  let nf = RwSignal::new(8.0);

  let bw_id = unique_id("receiver-sensitivity-bw");
  let nf_id = unique_id("receiver-sensitivity-nf");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.bandwidth-hz-2700-for")) r#for=bw_id.clone()>
        <NumberField
          id=bw_id
          value=Signal::derive(move || bw.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              bw.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.noise-figure-nf-db")) r#for=nf_id.clone()>
        <NumberField
          id=nf_id
          value=Signal::derive(move || nf.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              nf.set(v);
            }
          })
          controls=false
        />
      </Field>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let b = bw.get();
          if b <= 0.0 {
            t("tools.enter-a-positive-bandwidth")
          } else {
            let floor = -174.0 + 10.0 * b.log10();
            let sens = floor + nf.get();
            tf(
              "tools.receiver-sensitivity-dbm-noise",
              &[&fmt_num(sens), &fmt_num(floor), &fmt_num(nf.get())],
            )
          }
        }}
      </div>
    </div>
  }
}
