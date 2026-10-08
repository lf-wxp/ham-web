use leptos::prelude::*;

use super::{RESULT, fmt_num};
use crate::i18n::t;
use crate::ui::{ControlSize, Field, NumberField};
use crate::util::unique_id;

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

  // `Field` 的标签与控件是兄弟节点，`r#for` / `id` 必须配对才能点击标签聚焦输入框
  //（e2e 与读屏都按「标签 → 控件」的关联来定位）。
  let field = move |signal: RwSignal<String>, label: &'static str, unit: &'static str| {
    let id = unique_id("ohms");
    view! {
      <Field label=Signal::derive(move || format!("{}（{unit}）", t(label))) r#for=id.clone()>
        <NumberField
          id=id
          value=signal
          on_change=Callback::new(move |v: String| signal.set(v))
          size=ControlSize::Default
          controls=false
        />
      </Field>
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
                {move || t("tools.power-p")} <span class="font-mono font-semibold text-foreground">{fmt_num(power)}</span> " W"
              </span>
            }.into_any(),
            None => view! { {move || t("tools.fill-in-any-two")} }.into_any(),
          }
        }}
      </div>
    </div>
  }
}
