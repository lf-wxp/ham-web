use leptos::prelude::*;

use super::{INPUT, fmt_num, fmt_resistance};
use crate::i18n::{t, tf};

/// 电阻色环（4 环）：前两环数字 + 乘数 + 容差。
#[component]
pub(super) fn ResistorColorCode() -> impl IntoView {
  let ring1 = RwSignal::new(4.0);
  let ring2 = RwSignal::new(7.0);
  let mult = RwSignal::new(3.0);
  let tol = RwSignal::new(5.0);

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("第 1 环（十位，0–9）")}</span>
        <input
          type="number"
          step="1"
          prop:value=move || ring1.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              ring1.set(v.clamp(0.0, 9.0));
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("第 2 环（个位，0–9）")}</span>
        <input
          type="number"
          step="1"
          prop:value=move || ring2.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              ring2.set(v.clamp(0.0, 9.0));
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("乘数（10 的指数）")}</span>
        <input
          type="number"
          step="1"
          prop:value=move || mult.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              mult.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("容差（%）")}</span>
        <input
          type="number"
          prop:value=move || tol.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              tol.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let value = (ring1.get() * 10.0 + ring2.get()) * 10f64.powf(mult.get());
          tf(
            "阻值 = {} ± {}%",
            &[&fmt_resistance(value), &fmt_num(tol.get())],
          )
        }}
      </div>
      <p class="sm:col-span-2 text-xs text-muted-foreground">
        {move || t("色环对照：黑 0、棕 1、红 2、橙 3、黄 4、绿 5、蓝 6、紫 7、灰 8、白 9；乘数金 ×0.1、银 ×0.01，其余为 10 的指数。")}
      </p>
    </div>
  }
}
