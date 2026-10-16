//! 描述留在触发器旁；全站共享一个 body 气泡与委托监听，避免每个格子分配弹层和计时状态。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use leptos::{ev, html, prelude::*};
use send_wrapper::SendWrapper;
use wasm_bindgen::{JsCast, closure::Closure};

use crate::cn::cn;
use crate::ui::TextValue;
use crate::util::{document, unique_id, window};

fn closest_tip(target: Option<web_sys::EventTarget>) -> Option<web_sys::Element> {
  target?
    .dyn_into::<web_sys::Element>()
    .ok()?
    .closest(".pxl-tip")
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
  popup.set_class_name("pxl-tip-popup");
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
  let render = {
    let popup = popup.clone();
    let hovered = Rc::clone(&hovered);
    let focused = Rc::clone(&focused);
    let dismissed = Rc::clone(&dismissed);
    move || {
      let anchor = focused
        .borrow()
        .clone()
        .or_else(|| hovered.borrow().clone());
      let Some(anchor) = anchor.filter(|a| a.is_connected() && !dismissed.get()) else {
        popup.set_hidden(true);
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
      popup.set_hidden(false);
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
    move |e| {
      if !window()
        .match_media("(hover: hover)")
        .ok()
        .flatten()
        .is_some_and(|m| m.matches())
      {
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
    <span node_ref=root class=cn(&["pxl-tip relative inline-flex", &class])>
      {children()}
      <span id=id role="tooltip" hidden>{move || label.get()}</span>
    </span>
  }
}
