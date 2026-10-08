//! 三维方向图卡片：拖拽旋转视角，渲染见 [`super::sphere`]。
//!
//! 这一块**必须常驻 DOM**：渲染器在挂载时创建一次 WebGL2 上下文，之后只换数据。
//! 无解时用 `display:none` 隐藏而不是卸载 —— 浏览器上下文上限约 16 个，每改一个
//! 几何参数就重建一次的话很快就开始丢，实测拖拽会明显掉帧。

use ham_web_core::nec::NecResult;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::i18n::t;

use super::NOTE;
use super::sphere::PatternSphere;

/// 三维方向图。视角是本地状态：拖拽只改偏航 / 俯仰，不触发重新求解。
#[component]
pub(super) fn Pattern3d(result: Memo<Option<NecResult>>) -> impl IntoView {
  let rot_z = RwSignal::new(-30.0f64);
  let rot_x = RwSignal::new(20.0f64);
  let dragging = RwSignal::new(false);
  // 上一次指针坐标：不用 `movementX/Y`，合成事件里它们可能恒为 0。
  let last_pointer = StoredValue::new((0.0f64, 0.0f64));

  view! {
    <div
      class="space-y-2"
      style:display=move || if result.get().is_some() { "block" } else { "none" }
    >
      <div class=NOTE>{move || t("tools.nec-3d-pattern")}</div>
      <div
        class="relative mx-auto w-full max-w-[420px] cursor-grab touch-none select-none active:cursor-grabbing"
        role="group"
        aria-label=move || t("tools.nec-3d-pattern-alt")
        on:pointerdown=move |e: web_sys::PointerEvent| {
          dragging.set(true);
          last_pointer.set_value((e.client_x() as f64, e.client_y() as f64));
          if let Some(el) = e
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
          {
            let _ = el.set_pointer_capture(e.pointer_id());
          }
        }
        on:pointermove=move |e: web_sys::PointerEvent| {
          if !dragging.get() {
            return;
          }
          let (x0, y0) = last_pointer.get_value();
          let (x, y) = (e.client_x() as f64, e.client_y() as f64);
          last_pointer.set_value((x, y));
          rot_z.update(|v| *v += (x - x0) * 0.6);
          rot_x.update(|v| *v = (*v - (y - y0) * 0.6).clamp(-89.0, 89.0));
        }
        on:pointerup=move |e: web_sys::PointerEvent| {
          dragging.set(false);
          if let Some(el) = e
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
          {
            let _ = el.release_pointer_capture(e.pointer_id());
          }
        }
        on:pointercancel=move |_| dragging.set(false)
      >
        <PatternSphere
          result=Signal::derive(move || result.get())
          yaw=rot_z
          pitch=rot_x
        />
      </div>
      <div class=NOTE>{move || t("tools.nec-3d-hint")}</div>
    </div>
  }
}
