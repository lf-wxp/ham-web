use leptos::prelude::*;

use super::{INPUT, RESULT, fmt_num};
use crate::i18n::{t, tf};

/// 分贝增益：dB ↔ 功率 / 电压倍数（功率比 10lg、电压比 20lg）。
#[component]
pub(super) fn DecibelGain() -> impl IntoView {
  let db = RwSignal::new(3.0);
  let power_ratio = RwSignal::new(10f64.powf(3.0 / 10.0));

  let on_db = move |e: web_sys::Event| {
    if let Ok(d) = event_target_value(&e).parse::<f64>() {
      db.set(d);
      power_ratio.set(10f64.powf(d / 10.0));
    }
  };
  let on_ratio = move |e: web_sys::Event| {
    if let Ok(r) = event_target_value(&e).parse::<f64>()
      && r > 0.0
    {
      power_ratio.set(r);
      db.set(10.0 * r.log10());
    }
  };

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("增益（dB）")}</span>
        <input type="number" prop:value=move || db.get().to_string() on:input=on_db class=INPUT />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("功率倍数（倍）")}</span>
        <input type="number" prop:value=move || power_ratio.get().to_string() on:input=on_ratio class=INPUT />
      </label>
      <div class=RESULT>
        {move || {
          let d = db.get();
          tf(
            "{} dB → 功率 ×{}，电压 ×{}",
            &[
              &fmt_num(d),
              &fmt_num(power_ratio.get()),
              &fmt_num(10f64.powf(d / 20.0)),
            ],
          )
        }}
      </div>
    </div>
  }
}
