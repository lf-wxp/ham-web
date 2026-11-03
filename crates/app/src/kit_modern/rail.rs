//! 可拖拽的横向轨道。
//!
//! 为什么需要它：窄屏上的卡片列表要么竖着堆一屏（信息密度低），要么退化成「横向滚动条」，
//! 后者在触屏之外的设备上几乎没人会用。轨道把「拖拽」变成一等交互 —— 按住鼠标横向拖动、
//! 松手吸附到最近一张卡，键盘用户用左右方向键走同一段距离。
//!
//! 边界：
//!
//! - **只在真的溢出时接管**：内容没超过一屏时，按住拖动不该有任何反应；
//! - **拖拽结束后吞掉那次 click**：否则「想拖去看第三张」会变成「点进第一张」；
//! - **触屏不接管**：原生横向滚动已经有跟手与惯性，再包一层只会打架。

use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// 位移超过这么多像素才算「拖过」，用来吞掉随后的 click。
const DRAG_SLOP: f64 = 6.0;
/// 方向键一次走的距离（占可视宽度的比例，留一点重叠好让人知道自己是连续的）。
const KEY_STEP: f64 = 0.8;

/// 一次拖拽的状态。字段全是 `Copy`，`StoredValue` 不必关心所有权。
#[derive(Clone, Copy)]
struct Drag {
  pointer: i32,
  start_x: f64,
  start_left: f64,
  moved: f64,
  /// 是否已经捕获指针（位移越过 `DRAG_SLOP` 才捕获，见 `on_move`）。
  captured: bool,
}

/// 横向轨道：`children` 依次排开，溢出时可拖拽、可吸附、可键盘滚动。
///
/// 轨道本身是滚动容器（`overflow-x: auto`）—— `mobile_overflow` 的判据会跳过这类容器，
/// 所以窄屏不会因为它被判成整页横滚。
#[component]
pub fn ModernRail(
  children: Children,
  #[prop(optional, into)] class: String,
  /// 轨道的可访问名：`role="group"` 没有名字时读屏只会报「分组」。
  #[prop(optional, into)]
  aria_label: Option<Signal<String>>,
) -> impl IntoView {
  let drag = StoredValue::new(None::<Drag>);
  // 「刚拖过」的标记：click 紧跟 pointerup，用一个状态而不是计时器，
  // 免得在慢设备上计时器先跑完、反而把点击放过去。
  let dragged = StoredValue::new(false);

  let on_down = move |ev: web_sys::PointerEvent| {
    // 鼠标才接管：触屏的原生滚动已经有跟手和惯性。
    if ev.pointer_type() != "mouse" || ev.button() != 0 {
      return;
    }
    let Some(host) = host(&ev) else { return };
    if host.scroll_width() <= host.client_width() + 1 {
      return;
    }
    drag.set_value(Some(Drag {
      pointer: ev.pointer_id(),
      start_x: f64::from(ev.client_x()),
      start_left: f64::from(host.scroll_left()),
      moved: 0.0,
      captured: false,
    }));
    dragged.set_value(false);
    // 这里**不**捕获指针：按下就捕获的话，随后的 click 目标会被改写成轨道本身，轨道里的
    // 模块链接就点不开了（只想点一下的鼠标用户最常碰到）。越过 `DRAG_SLOP` 才算拖拽，见 `on_move`。
  };

  let on_move = move |ev: web_sys::PointerEvent| {
    let Some(state) = drag.get_value() else {
      return;
    };
    if ev.pointer_id() != state.pointer {
      return;
    }
    let Some(host) = host(&ev) else { return };
    let dx = f64::from(ev.client_x()) - state.start_x;
    if dx.abs() <= DRAG_SLOP && !state.captured {
      // 还没拖开：不滚动、不 `prevent_default`，让这次按下仍然可以是一次普通点击。
      return;
    }
    dragged.set_value(true);
    if !state.captured {
      // 捕获指针：拖出轨道边界后仍然收得到 move，松手不会「卡在拖拽态」。
      let _ = host.class_list().add_1("md-tracking");
      let _ = host.set_pointer_capture(ev.pointer_id());
    }
    host.set_scroll_left((state.start_left - dx).round() as i32);
    drag.update_value(|slot| {
      if let Some(s) = slot {
        s.moved = s.moved.max(dx.abs());
        s.captured = true;
      }
    });
    ev.prevent_default();
  };

  let on_up = move |ev: web_sys::PointerEvent| {
    let Some(state) = drag.get_value() else {
      return;
    };
    drag.set_value(None);
    if let Some(host) = host(&ev) {
      if state.captured {
        let _ = host.release_pointer_capture(state.pointer);
      }
      let _ = host.class_list().remove_1("md-tracking");
    }
    dragged.set_value(state.moved > DRAG_SLOP);
  };

  let on_click = move |ev: web_sys::MouseEvent| {
    if dragged.get_value() {
      dragged.set_value(false);
      ev.prevent_default();
      ev.stop_propagation();
    }
  };

  let on_key = move |ev: web_sys::KeyboardEvent| {
    // 只处理焦点就在轨道自身时的按键：轨道里的链接 / 输入框自己的方向键、Home / End
    // 不该被截走（按键冒泡到这里时 `target` 是后代，不等于 `current_target`）。
    if ev.target() != ev.current_target() {
      return;
    }
    let Some(host) = host(&ev) else { return };
    let max = f64::from(host.scroll_width() - host.client_width()).max(0.0);
    // 没有溢出就没有可滚的东西：把按键还给浏览器。
    if max <= 0.0 {
      return;
    }
    let step = f64::from(host.client_width()) * KEY_STEP;
    let target = match ev.key().as_str() {
      "ArrowRight" => f64::from(host.scroll_left()) + step,
      "ArrowLeft" => f64::from(host.scroll_left()) - step,
      "Home" => 0.0,
      "End" => max,
      _ => return,
    };
    ev.prevent_default();
    host.set_scroll_left(target.clamp(0.0, max).round() as i32);
  };

  view! {
    <div
      class=format!("md-track {class}")
      role="group"
      aria-label=move || aria_label.map(|l| l.get())
      tabindex="0"
      on:pointerdown=on_down
      on:pointermove=on_move
      on:pointerup=on_up
      on:pointercancel=on_up
      on:lostpointercapture=on_up
      on:keydown=on_key
      on:click=on_click
    >
      {children()}
    </div>
  }
}

/// 取当前事件的目标元素（轨道自身）。
fn host(ev: &web_sys::Event) -> Option<web_sys::HtmlElement> {
  ev.current_target()
    .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok())
}
