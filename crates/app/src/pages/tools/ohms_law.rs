use leptos::prelude::*;

use super::{INPUT, RESULT, fmt_num};
use crate::i18n::t;

/// 欧姆定律 / 电功率：U = I·R，P = U·I。填任意两项，计算其余。
#[component]
pub(super) fn OhmsLaw() -> impl IntoView {
  let voltage = RwSignal::new(String::new());
  let current = RwSignal::new(String::new());
  let resistance = RwSignal::new(String::new());

  let calc = Memo::new(move |_| {
    let u = voltage.get().trim().parse::<f64>().ok();
    let i = current.get().trim().parse::<f64>().ok();
    let r = resistance.get().trim().parse::<f64>().ok();
    match (u, i, r) {
      (Some(u), Some(i), _) if i != 0.0 => Some(("R", u / i, u * i)),
      (Some(u), _, Some(r)) if r != 0.0 => Some(("I", u / r, u * u / r)),
      (_, Some(i), Some(r)) => Some(("U", i * r, i * i * r)),
      _ => None,
    }
  });

  let field = move |signal: RwSignal<String>, label: &'static str, unit: &'static str| {
    view! {
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">
          {move || t(label)} "（" {unit} "）"
        </span>
        <input
          type="number"
          prop:value=move || signal.get()
          on:input=move |e| signal.set(event_target_value(&e))
          class=INPUT
        />
      </label>
    }
  };

  view! {
    <div class="space-y-3">
      <div class="grid gap-3 sm:grid-cols-3">
        {field(voltage, "电压 U", "V")}
        {field(current, "电流 I", "A")}
        {field(resistance, "电阻 R", "Ω")}
      </div>
      <div class=RESULT>
        {move || {
          match calc.get() {
            Some((name, value, power)) => view! {
              <span>
                {name} " = " <span class="font-mono font-semibold text-foreground">{fmt_num(value)}</span>
                {move || t("　功率 P = ")} <span class="font-mono font-semibold text-foreground">{fmt_num(power)}</span> " W"
              </span>
            }.into_any(),
            None => view! { {move || t("填写任意两项（U、I、R）后自动计算。")} }.into_any(),
          }
        }}
      </div>
    </div>
  }
}
