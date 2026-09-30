use leptos::prelude::*;

use super::{INPUT, RESULT, fmt_num};

/// 天线长度估算：半波偶极 143/f、1/4 波长 71.5/f、5/8 波长 187.5/f（米，f 为 MHz）。
#[component]
pub(super) fn AntennaLength() -> impl IntoView {
  let freq = RwSignal::new(14.2);

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">"频率（MHz）"</span>
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
      <div class=RESULT>
        {move || {
          let f = freq.get();
          if f <= 0.0 {
            "请输入正频率".to_owned()
          } else {
            format!(
              "半波偶极 ≈ {} m　1/4 波长 ≈ {} m　5/8 波长 ≈ {} m",
              fmt_num(143.0 / f),
              fmt_num(71.5 / f),
              fmt_num(187.5 / f),
            )
          }
        }}
      </div>
    </div>
  }
}
