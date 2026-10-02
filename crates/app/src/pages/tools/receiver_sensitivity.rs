use leptos::prelude::*;

use super::{INPUT, fmt_num};
use crate::i18n::{t, tf};

/// 接收机灵敏度：S(dBm) = -174 + 10lg(BW) + NF。
#[component]
pub(super) fn ReceiverSensitivity() -> impl IntoView {
  let bw = RwSignal::new(2700.0);
  let nf = RwSignal::new(8.0);

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("带宽（Hz，SSB 约 2700）")}</span>
        <input
          type="number"
          prop:value=move || bw.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              bw.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("噪声系数 NF（dB）")}</span>
        <input
          type="number"
          prop:value=move || nf.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              nf.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let b = bw.get();
          if b <= 0.0 {
            t("请输入正带宽")
          } else {
            let floor = -174.0 + 10.0 * b.log10();
            let sens = floor + nf.get();
            tf(
              "接收灵敏度 ≈ {} dBm（噪声底线 {} dBm + NF {} dB）",
              &[&fmt_num(sens), &fmt_num(floor), &fmt_num(nf.get())],
            )
          }
        }}
      </div>
    </div>
  }
}
