use leptos::prelude::*;

use super::{INPUT, fmt_num};
use crate::i18n::{t, tf};

/// 自由空间路径损耗：FSPL(dB) = 20lg(d) + 20lg(f) + 32.45。
#[component]
pub(super) fn FsplCalculator() -> impl IntoView {
  let dist = RwSignal::new(100.0);
  let freq = RwSignal::new(145.0);

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("距离（km）")}</span>
        <input
          type="number"
          prop:value=move || dist.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              dist.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("频率（MHz）")}</span>
        <input
          type="number"
          prop:value=move || freq.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              freq.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let (d, f) = (dist.get(), freq.get());
          if d <= 0.0 || f <= 0.0 {
            t("请输入正的距离与频率")
          } else {
            let loss = 20.0 * d.log10() + 20.0 * f.log10() + 32.45;
            tf("自由空间路径损耗 ≈ {} dB", &[&(fmt_num(loss)).to_string()])
          }
        }}
      </div>
    </div>
  }
}
