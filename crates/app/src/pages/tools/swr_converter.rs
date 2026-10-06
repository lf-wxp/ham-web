use leptos::prelude::*;

use super::{INPUT, RESULT, fmt_num};
use crate::i18n::{t, tf};

/// 驻波比 ↔ 反射系数 / 回波损耗。
#[component]
pub(super) fn SwrConverter() -> impl IntoView {
  let swr = RwSignal::new(1.5);
  let return_loss = RwSignal::new(-20.0 * ((1.5f64 - 1.0) / (1.5f64 + 1.0)).log10());

  let on_swr = move |e: web_sys::Event| {
    if let Ok(s) = event_target_value(&e).parse::<f64>()
      && s >= 1.0
    {
      swr.set(s);
      return_loss.set(-20.0 * ((s - 1.0) / (s + 1.0)).log10());
    }
  };
  let on_rl = move |e: web_sys::Event| {
    if let Ok(rl) = event_target_value(&e).parse::<f64>()
      && rl < 0.0
    {
      return_loss.set(rl);
      let gamma = 10f64.powf(rl / 20.0);
      swr.set((1.0 + gamma) / (1.0 - gamma));
    }
  };

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("驻波比 SWR")}</span>
        <input type="number" prop:value=move || swr.get().to_string() on:input=on_swr class=INPUT />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("S11（dB，负值）")}</span>
        <input type="number" prop:value=move || return_loss.get().to_string() on:input=on_rl class=INPUT />
      </label>
      <div class=RESULT>
        {move || {
          let s = swr.get();
          let gamma = (s - 1.0) / (s + 1.0);
          tf(
            "SWR {} → 反射系数 |Γ| = {}，S11 = {} dB（回波损耗取正值为 {} dB）",
            &[
              &fmt_num(s),
              &fmt_num(gamma),
              &fmt_num(return_loss.get()),
              &fmt_num(-return_loss.get()),
            ],
          )
        }}
      </div>
    </div>
  }
}
