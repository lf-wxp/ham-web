//! 滑块：替代原生 `<input type="range">`（音调 / 音量 / 阈值一类连续量）。
//!
//! 跨浏览器的轨道与滑块自绘集中在 `style/pixel/surfaces.css` 的 `.ui-slider` 里；组件只负责把
//! 当前进度以 `--slider-fill` 传给 CSS，填充比例无需 JS 逐帧计算。

use leptos::prelude::*;

use crate::cn::cn;

use super::control::TextValue;

/// 像素风轨道的格数与刻度道数（`docs/ui/pixel.md` §3.4）。
const CELLS: f64 = 24.0;
const TICKS: u8 = 20;

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
  let modern = crate::kit::is_modern_at_mount();
  // 像素风的轨道是 24 格电平条：填色按整格走（格心过了当前值才亮），不出现半格。
  let fill = move || {
    if modern {
      pct()
    } else {
      (pct() / 100.0 * CELLS + 0.5).floor() / CELLS * 100.0
    }
  };

  let input = view! {
    <input
      type="range"
      data-slot="slider"
      id=id
      class=if modern { cn(&["ui-slider", &class]) } else { "ui-slider".to_owned() }
      style=move || format!("--slider-fill: {:.2}%", fill())
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
  };
  if modern {
    return input.into_any();
  }
  // 推子上方的刻度尺：21 道，每 5 道一根长的。纯装饰，宽度类给外层（调用方传的 `w-32` 这类）。
  view! {
    <span class=cn(&["pxl-slider", &class])>
      <span class="pxl-slider-ticks" aria-hidden="true">
        {(0..=TICKS).map(|i| view! { <span data-major=(i % 5 == 0).then_some("")></span> }).collect_view()}
      </span>
      {input}
    </span>
  }
  .into_any()
}
