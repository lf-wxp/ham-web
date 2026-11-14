use ham_web_core::rig_panel::{knob_point, knob_step};
use leptos::prelude::*;

use super::control::TextValue;
use super::readout::Readout;
use super::slider::Slider;

/// 旋钮外圈的刻度灯数。
const DOTS: usize = 11;
/// 拖动多少像素走一格：太小手一抖就跳，太大要拖很远。
const DRAG_PX_PER_STEP: f64 = 4.0;
/// 旋钮的画布边长与半径（像素格对齐）。
const SIZE: i32 = 80;
const RING_R: f64 = 34.0;
const DIAL_R: f64 = 11.0;
/// 本体八边形、它的硬投影（右下各 2px）与左上的高光折线。
const BODY: &str = "30,18 50,18 62,30 62,50 50,62 30,62 18,50 18,30";
const BODY_HI: &str = "32,22 48,22 58,32 58,48";

/// 旋钮：面板上「调谐」类连续量（音调、音量、增益）的控件。外观在 `style/pixel/radio.css`（`pxl-knob`）。
///
/// 本体不旋转（转动点阵会出现非整数格的糊边）：指示点与外圈 11 颗刻度灯按 `rig_panel::knob_point`
/// 摆在 270° 弧上。交互：上下拖动（每 4px 一格）、滚轮（有焦点时）、方向键（上 / 右加、下 / 左减）、
/// PageUp / PageDown 走 10 格、Home / End 直接到两端。`role="slider"`，读屏与键盘行为和 `Slider` 一致。
///
/// 旁边是标签与一块读数屏（七段数字 + 单位丝印）：调值时看读数，不看指针的角度。
/// 现代风没有旋钮：同一行渲染成「标签 + `Slider` + 数值」，调用点不用分叉（`docs/ui/pixel.md` §3 决策表）。
#[component]
pub fn Knob(
  #[prop(into)] value: Signal<f64>,
  on_change: Callback<f64>,
  min: f64,
  max: f64,
  #[prop(default = 1.0)] step: f64,
  /// 可见标签（「音调」）。
  #[prop(into)]
  label: TextValue,
  /// 无障碍名，可以比可见标签说得更完整（「播放音调」）。
  #[prop(into)]
  aria_label: TextValue,
  /// 读数屏上的数字部分（「700」「+12」），只用七段认得的字符。
  #[prop(into)]
  readout: Signal<String>,
  /// 读数屏上的单位丝印（「Hz」「dB」）。
  #[prop(into)]
  unit: TextValue,
  /// 读屏播报的当前值（如 `700 Hz`），必填：裸数字对读屏没有单位。现代风也拿它当可见数值。
  #[prop(into)]
  aria_valuetext: Signal<String>,
  /// 现代风回落到 `Slider` 时给它的宽度类（像素风的旋钮是定宽的，不读）。
  #[prop(optional, into)]
  class: String,
) -> impl IntoView {
  if crate::kit::is_modern_at_mount() {
    return view! {
      <label class="inline-flex items-center gap-2 text-xs text-muted-foreground">
        <span class="whitespace-nowrap">{move || label.get()}</span>
        <Slider
          value=value
          on_change=on_change
          min=min
          max=max
          step=step
          aria_label=aria_label
          aria_valuetext=aria_valuetext
          class=class
        />
        <span class="w-14 tabular-nums text-foreground">{move || aria_valuetext.get()}</span>
      </label>
    }
    .into_any();
  }

  let frac = move || {
    let span = max - min;
    if span > 0.0 {
      ((value.get() - min) / span).clamp(0.0, 1.0)
    } else {
      0.0
    }
  };
  let nudge = move |steps: i32| {
    let next = knob_step(value.get_untracked(), steps, min, max, step);
    if (next - value.get_untracked()).abs() > f64::EPSILON {
      on_change.run(next);
    }
  };
  // 拖动起点：指针 y 与当时的值；松手或指针离开捕获时清掉。
  let drag = StoredValue::new(None::<(f64, f64)>);
  let center = SIZE / 2;
  let dots = move || {
    let f = frac();
    (0..DOTS)
      .map(|i| {
        let at = i as f64 / (DOTS - 1) as f64;
        let (x, y) = knob_point(at, RING_R);
        view! {
          <rect
            class="pxl-knob-dot"
            data-lit=(at <= f + 1e-9).then_some("")
            x=center + x - 2
            y=center + y - 2
            width="4"
            height="4"
          />
        }
      })
      .collect_view()
  };
  let pointer = move || {
    let (x, y) = knob_point(frac(), DIAL_R);
    view! { <rect class="pxl-knob-pointer" x=center + x - 2 y=center + y - 2 width="4" height="4" /> }
  };

  view! {
    <div class="pxl-knob-row">
      <span
        class="pxl-knob"
        role="slider"
        tabindex="0"
        // 竖向拖动调值（`touch-action: none`），读屏据此告诉用户是上下方向。
        aria-orientation="vertical"
        aria-label=move || aria_label.get()
        aria-valuemin=min
        aria-valuemax=max
        aria-valuenow=move || value.get().to_string()
        aria-valuetext=move || aria_valuetext.get()
        on:keydown=move |e: web_sys::KeyboardEvent| {
          // Home / End 直接落到两端，不靠「走很多格再夹住」。
          let ends = match e.key().as_str() {
            "Home" => Some(min),
            "End" => Some(max),
            _ => None,
          };
          if let Some(v) = ends {
            e.prevent_default();
            if (v - value.get_untracked()).abs() > f64::EPSILON {
              on_change.run(v);
            }
            return;
          }
          let steps = match e.key().as_str() {
            "ArrowUp" | "ArrowRight" => 1,
            "ArrowDown" | "ArrowLeft" => -1,
            "PageUp" => 10,
            "PageDown" => -10,
            _ => return,
          };
          e.prevent_default();
          nudge(steps);
        }
        // 滚轮只在旋钮有焦点时调值：否则滚动页面时指针扫过旋钮，就会把页面滚动吞掉、顺手改了值。
        on:wheel=move |e: web_sys::WheelEvent| {
          let focused = crate::util::document().active_element();
          let target = e
            .current_target()
            .and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::Element>(t).ok());
          if focused.is_none() || focused != target {
            return;
          }
          e.prevent_default();
          nudge(if e.delta_y() < 0.0 { 1 } else { -1 });
        }
        on:pointerdown=move |e: web_sys::PointerEvent| {
          if let Some(el) = e
            .current_target()
            .and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::Element>(t).ok())
          {
            let _ = el.set_pointer_capture(e.pointer_id());
          }
          drag.set_value(Some((f64::from(e.client_y()), value.get_untracked())));
        }
        on:pointermove=move |e: web_sys::PointerEvent| {
          if let Some((y0, v0)) = drag.get_value() {
            let steps = ((y0 - f64::from(e.client_y())) / DRAG_PX_PER_STEP).round() as i32;
            let next = knob_step(v0, steps, min, max, step);
            if (next - value.get_untracked()).abs() > f64::EPSILON {
              on_change.run(next);
            }
          }
        }
        on:pointerup=move |_| drag.set_value(None)
        on:pointercancel=move |_| drag.set_value(None)
      >
        <svg width=SIZE height=SIZE viewBox=format!("0 0 {SIZE} {SIZE}") aria-hidden="true">
          {dots}
          <polygon class="pxl-knob-shadow" points=BODY transform="translate(2 2)" />
          <polygon class="pxl-knob-body" points=BODY />
          <polyline class="pxl-knob-hi" points=BODY_HI />
          {pointer}
        </svg>
      </span>
      <span class="flex flex-col gap-1">
        <span class="pxl-label text-xs text-muted-foreground">{move || label.get()}</span>
        <span class="pxl-display inline-flex w-fit items-center gap-1.5 px-1.5 py-1" aria-hidden="true">
          <Readout text=readout />
          <span class="pxl-label pxl-d-dim text-xs">{move || unit.get()}</span>
        </span>
        // 读数屏是图形；数值的文字版给按文字定位的 e2e 与浏览模式下的读屏（与原先旁边那行数值同一口径）。
        <span class="sr-only">{move || aria_valuetext.get()}</span>
      </span>
    </div>
  }
  .into_any()
}
