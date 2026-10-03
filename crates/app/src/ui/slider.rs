//! 滑块：替代原生 `<input type="range">`（音调 / 音量 / 阈值一类连续量）。
//!
//! 跨浏览器的轨道与滑块自绘集中在 `style/input.css` 的 `.ui-slider` 里；组件只负责把
//! 当前进度以 `--slider-fill` 传给 CSS，填充比例无需 JS 逐帧计算。

use leptos::prelude::*;

use crate::cn::cn;

use super::control::TextValue;

/// 滑块。
#[component]
pub fn Slider(
  /// 当前值（受控）。
  #[prop(into)]
  value: Signal<f64>,
  /// 拖动后的新值。
  on_change: Callback<f64>,
  #[prop(optional, default = 0.0)] min: f64,
  #[prop(optional, default = 100.0)] max: f64,
  #[prop(optional, default = 1.0)] step: f64,
  #[prop(optional, into)] disabled: Signal<bool>,
  /// 无可见标签时的无障碍名称。
  #[prop(optional, into)]
  aria_label: Option<TextValue>,
  /// 读屏播报的当前值文案（如 `700 Hz`）。
  #[prop(optional, into)]
  aria_valuetext: Option<Signal<String>>,
  #[prop(optional, into)] id: Option<String>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let pct = move || {
    let span = max - min;
    if span <= 0.0 {
      0.0
    } else {
      ((value.get() - min) / span * 100.0).clamp(0.0, 100.0)
    }
  };
  let valuetext = aria_valuetext;

  view! {
    <input
      type="range"
      data-slot="slider"
      id=id
      class=cn(&["ui-slider", &class])
      style=move || format!("--slider-fill: {:.2}%", pct())
      min=min
      max=max
      step=step
      aria-label=move || aria_label.as_ref().map(TextValue::get)
      aria-valuemin=min
      aria-valuemax=max
      aria-valuenow=move || value.get().to_string()
      aria-valuetext=move || valuetext.map(|s| s.get())
      disabled=move || disabled.get()
      prop:value=move || value.get().to_string()
      on:input=move |e| {
        if let Ok(v) = event_target_value(&e).parse::<f64>() {
          on_change.run(v);
        }
      }
    />
  }
}
