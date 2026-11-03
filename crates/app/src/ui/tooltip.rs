//! 描述留在触发器旁；全站共享一个 body 气泡与委托监听，避免每个格子分配弹层和计时状态。

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use leptos::{ev, html, prelude::*};
use send_wrapper::SendWrapper;
use wasm_bindgen::{JsCast, closure::Closure};

use crate::cn::cn;
use crate::presence::{ExitSpeed, exit_ms};
use crate::ui::TextValue;
use crate::util::{document, unique_id, window};

fn closest_tip(target: Option<web_sys::EventTarget>) -> Option<web_sys::Element> {
  target?
    .dyn_into::<web_sys::Element>()
    .ok()?
    .closest("[data-tip]")
    .ok()
    .flatten()
}

/// 在 App 根 Owner 接线一次，监听与气泡都随根组件清理。
pub(crate) fn setup_tooltip_popup() {
  let Ok(popup) = document()
    .create_element("span")
    .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().map_err(Into::into))
  else {
    return;
  };
  // 初始类名也按当前风格给：否则现代风的 DOM 里会先挂着一个 `.pxl-tip-popup`
  //（虽然 `hidden`，但静态审计会把它当成残留，第一次显示前也可能闪一帧像素样式）。
  popup.set_class_name(if crate::theme::DisplayPrefs::style_is_modern_dom() {
    "md-tip-popup"
  } else {
    "pxl-tip-popup"
  });
  popup.set_hidden(true);
  let _ = popup.set_attribute("data-slot", "tooltip-popup");
  let _ = popup.set_attribute("aria-hidden", "true");
  let Some(body) = document().body() else {
    return;
  };
  let _ = body.append_child(&popup);
  let hovered = Rc::new(RefCell::new(None::<web_sys::Element>));
  let focused = Rc::new(RefCell::new(None::<web_sys::Element>));
  let dismissed = Rc::new(Cell::new(false));
  // 退场：先挂 `data-state="closed"` 播完淡出（动画，见 `tip-popup` 的 CSS），再 `hidden`。
  // 代际号让「淡出途中又悬停到下一个触发器」取消那次延迟隐藏，否则新气泡刚出现就被旧计时器藏掉。
  let hide_gen = Rc::new(Cell::new(0u32));
  let hide = {
    let popup = popup.clone();
    let hide_gen = Rc::clone(&hide_gen);
    move || {
      if popup.hidden() || popup.get_attribute("data-state").as_deref() == Some("closed") {
        return;
      }
      let _ = popup.set_attribute("data-state", "closed");
      let ticket = hide_gen.get().wrapping_add(1);
      hide_gen.set(ticket);
      let popup = popup.clone();
      let hide_gen = Rc::clone(&hide_gen);
      set_timeout(
        move || {
          if hide_gen.get() == ticket {
            popup.set_hidden(true);
          }
        },
        Duration::from_millis(exit_ms(ExitSpeed::Fast)),
      );
    }
  };
  let show = {
    let popup = popup.clone();
    let hide_gen = Rc::clone(&hide_gen);
    move || {
      hide_gen.set(hide_gen.get().wrapping_add(1));
      popup.set_hidden(false);
      let _ = popup.set_attribute("data-state", "open");
    }
  };
  let render = {
    let popup = popup.clone();
    let hide = hide.clone();
    let show = show.clone();
    let hovered = Rc::clone(&hovered);
    let focused = Rc::clone(&focused);
    let dismissed = Rc::clone(&dismissed);
    move || {
      // 风格可能在运行中切换，所以每次显示前按当前风格重设类名。
      // 这里**不能**用 `kit::is_modern_at_mount()`：气泡渲染在根 Owner 之外的普通闭包里，
      // 取 `DisplayPrefs` 上下文会直接 panic（实测会在悬停时弹出致命错误页）。
      // 读根元素上的 `data-ui-style` 是同一事实来源，且不依赖任何上下文。
      let modern = crate::theme::DisplayPrefs::style_is_modern_dom();
      popup.set_class_name(if modern {
        "md-tip-popup"
      } else {
        "pxl-tip-popup"
      });
      let anchor = focused
        .borrow()
        .clone()
        .or_else(|| hovered.borrow().clone());
      let Some(anchor) = anchor.filter(|a| a.is_connected() && !dismissed.get()) else {
        hide();
        return;
      };
      let Some(label) = anchor
        .query_selector(":scope > [role='tooltip']")
        .ok()
        .flatten()
      else {
        return;
      };
      popup.set_text_content(label.text_content().as_deref());
      show();
      let a = anchor.get_bounding_client_rect();
      let r = popup.get_bounding_client_rect();
      let width = document()
        .document_element()
        .map_or(0.0, |el| f64::from(el.client_width()));
      let height = window()
        .inner_height()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
      let left =
        (a.left() + (a.width() - r.width()) / 2.0).clamp(8.0, (width - r.width() - 8.0).max(8.0));
      let above = a.top() - r.height() - 8.0;
      let top = if above >= 8.0 {
        above
      } else {
        a.bottom() + 8.0
      };
      let top = top.clamp(8.0, (height - r.height() - 8.0).max(8.0));
      let style = popup.style();
      let _ = style.set_property("left", &format!("{left}px"));
      let _ = style.set_property("top", &format!("{top}px"));
    }
  };
  let over = window_event_listener(ev::mouseover, {
    let hovered = Rc::clone(&hovered);
    let dismissed = Rc::clone(&dismissed);
    let render = render.clone();
    // `MediaQueryList` 建一次：`match_media` 每次调用都新建一个对象，而 `mouseover` 是高频事件。
    // 对象本身是活的，用户换了输入设备（接上 / 拔掉鼠标）后下一次 `matches()` 读到的就是新值。
    let hover_query = window().match_media("(hover: hover)").ok().flatten();
    move |e| {
      if !hover_query.as_ref().is_some_and(|m| m.matches()) {
        return;
      }
      let tip = closest_tip(e.target());
      if *hovered.borrow() != tip {
        dismissed.set(false);
      }
      *hovered.borrow_mut() = tip;
      render();
    }
  });
  let out = window_event_listener(ev::mouseout, {
    let hovered = Rc::clone(&hovered);
    let render = render.clone();
    move |e| {
      *hovered.borrow_mut() = closest_tip(e.related_target());
      render();
    }
  });
  let focus_in = window_event_listener(ev::focusin, {
    let focused = Rc::clone(&focused);
    let dismissed = Rc::clone(&dismissed);
    let render = render.clone();
    move |e| {
      *focused.borrow_mut() = closest_tip(e.target());
      dismissed.set(false);
      render();
    }
  });
  let focus_out = window_event_listener(ev::focusout, {
    let focused = Rc::clone(&focused);
    let render = render.clone();
    move |e| {
      *focused.borrow_mut() = closest_tip(e.related_target());
      render();
    }
  });
  let key = window_event_listener(ev::keydown, {
    let dismissed = Rc::clone(&dismissed);
    let render = render.clone();
    move |e| {
      if e.key() == "Escape" {
        dismissed.set(true);
        render();
      }
    }
  });
  let click = window_event_listener(ev::click, {
    let dismissed = Rc::clone(&dismissed);
    let render = render.clone();
    move |_| {
      dismissed.set(true);
      render();
    }
  });
  let resize = window_event_listener(ev::resize, {
    let render = render.clone();
    move |_| render()
  });
  // scroll 不冒泡，捕获监听才能跟随表格、侧栏和弹窗内部滚动。
  let scroll = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| render());
  let _ = window().add_event_listener_with_callback_and_bool(
    "scroll",
    scroll.as_ref().unchecked_ref(),
    true,
  );
  let keep = SendWrapper::new((popup, scroll));
  on_cleanup(move || {
    over.remove();
    out.remove();
    focus_in.remove();
    focus_out.remove();
    key.remove();
    click.remove();
    resize.remove();
    let (popup, scroll) = keep.take();
    let _ = window().remove_event_listener_with_callback_and_bool(
      "scroll",
      scroll.as_ref().unchecked_ref(),
      true,
    );
    popup.remove();
  });
}

#[component]
pub fn Tooltip(
  #[prop(into)] label: TextValue,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let root = NodeRef::<html::Span>::new();
  let id = unique_id("tooltip");
  let description = id.clone();
  root.on_load(move |el| {
    let target = el
      .query_selector("button, a, input, [tabindex], [role]:not([role='tooltip'])")
      .ok()
      .flatten()
      .or_else(|| el.first_element_child());
    if let Some(target) = target {
      let existing = target.get_attribute("aria-describedby").unwrap_or_default();
      let value = format!("{existing} {description}");
      let _ = target.set_attribute("aria-describedby", value.trim());
    }
  });
  view! {
    // `data-tip` 是**纯钩子**（事件代理用它找触发器），与皮肤无关 ——
    // 两套设计系统都不该在 DOM 上互相留痕，所以不叫 `pxl-tip` 也不叫 `md-tip`。
    <span node_ref=root data-tip="" class=cn(&["relative inline-flex", &class])>
      {children()}
      <span id=id role="tooltip" hidden>{move || label.get()}</span>
    </span>
  }
}
