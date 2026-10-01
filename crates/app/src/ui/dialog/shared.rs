use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use leptos::ev;
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;

use crate::util::body;

thread_local! {
  static SCROLL_LOCKS: Cell<u32> = const { Cell::new(0) };
}

pub(super) fn lock_scroll(on: bool) {
  let count = SCROLL_LOCKS.with(|c| {
    let n = if on {
      c.get() + 1
    } else {
      c.get().saturating_sub(1)
    };
    c.set(n);
    n
  });
  if let Some(b) = body() {
    let style = b.style();
    let _ = if count > 0 {
      style.set_property("overflow", "hidden")
    } else {
      style.remove_property("overflow").map(|_| ())
    };
  }
}

const FOCUSABLE: &str = "input:not([disabled]), button:not([disabled]), [href], select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])";

thread_local! {
  static TITLE_SEQ: Cell<u32> = const { Cell::new(0) };
}

/// 打开时的无障碍处理：未显式命名时以内容中首个标题作为对话框名称（`aria-labelledby`），
/// 再聚焦首个可交互元素。
pub(super) fn on_open(root: &web_sys::Element) {
  if !root.has_attribute("aria-labelledby")
    && !root.has_attribute("aria-label")
    && let Ok(Some(title)) = root.query_selector("[data-slot$='-title'], h1, h2, h3")
  {
    if title.id().is_empty() {
      let n = TITLE_SEQ.with(|c| {
        c.set(c.get() + 1);
        c.get()
      });
      title.set_id(&format!("dialog-title-{n}"));
    }
    let _ = root.set_attribute("aria-labelledby", &title.id());
  }
  focus_first(root);
}

/// 模态内 Tab / Shift+Tab 循环聚焦，避免焦点跑到被遮罩的页面上。
pub(super) fn trap_tab(e: &web_sys::KeyboardEvent, root: &web_sys::Element) {
  if e.key() != "Tab" {
    return;
  }
  let Ok(nodes) = root.query_selector_all(FOCUSABLE) else {
    return;
  };
  let items: Vec<web_sys::HtmlElement> = (0..nodes.length())
    .filter_map(|i| nodes.item(i))
    .filter_map(|n| n.dyn_into::<web_sys::HtmlElement>().ok())
    .filter(|el| el.offset_parent().is_some())
    .collect();
  let (Some(first), Some(last)) = (items.first(), items.last()) else {
    e.prevent_default();
    return;
  };
  let active = web_sys::window()
    .and_then(|w| w.document())
    .and_then(|d| d.active_element());
  let at = |el: &web_sys::HtmlElement| {
    active
      .as_ref()
      .is_some_and(|a| a == el.unchecked_ref::<web_sys::Element>())
  };
  let at_root = active.as_ref().is_some_and(|a| a == root);
  if e.shift_key() && (at(first) || at_root) {
    e.prevent_default();
    let _ = last.focus();
  } else if !e.shift_key() && at(last) {
    e.prevent_default();
    let _ = first.focus();
  }
}

pub(super) fn focus_first(root: &web_sys::Element) {
  let target = root
    .query_selector(FOCUSABLE)
    .ok()
    .flatten()
    .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
    .or_else(|| root.clone().dyn_into::<web_sys::HtmlElement>().ok());
  if let Some(el) = target {
    let _ = el.focus();
  }
}

/// 打开/关闭的挂载状态：关闭后延迟卸载以播放退出动画，并负责滚动锁、Esc 关闭。
pub(super) fn use_presence(open: RwSignal<bool>, exit_ms: u64) -> RwSignal<bool> {
  let mounted = RwSignal::new(open.get_untracked());
  let locked = Arc::new(AtomicBool::new(false));
  // 打开前的焦点元素：关闭后把焦点还给触发按钮，键盘 / 读屏用户不丢失位置。
  let opener = StoredValue::new(SendWrapper::new(None::<web_sys::HtmlElement>));

  let lock = locked.clone();
  Effect::new(move |_| {
    if open.get() {
      if !mounted.get_untracked() || opener.with_value(|o| o.is_none()) {
        let active = web_sys::window()
          .and_then(|w| w.document())
          .and_then(|d| d.active_element())
          .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok());
        opener.set_value(SendWrapper::new(active));
      }
      mounted.set(true);
      if !lock.swap(true, Ordering::Relaxed) {
        lock_scroll(true);
      }
    } else {
      if lock.swap(false, Ordering::Relaxed) {
        lock_scroll(false);
      }
      if let Some(el) = opener.with_value(|o| (**o).clone()) {
        let _ = el.focus();
      }
      opener.set_value(SendWrapper::new(None));
      set_timeout(
        move || {
          if open.try_get_untracked() == Some(false) {
            mounted.try_set(false);
          }
        },
        Duration::from_millis(exit_ms),
      );
    }
  });

  let handle = window_event_listener(ev::keydown, move |e| {
    if e.key() == "Escape" && open.get_untracked() {
      e.prevent_default();
      open.set(false);
    }
  });
  let handle = SendWrapper::new(Some(handle));
  on_cleanup(move || {
    if let Some(h) = handle.take() {
      h.remove();
    }
    if locked.swap(false, Ordering::Relaxed) {
      lock_scroll(false);
    }
  });

  mounted
}

pub(super) fn state_attr(open: RwSignal<bool>) -> impl Fn() -> &'static str + Copy + Send + Sync {
  move || if open.get() { "open" } else { "closed" }
}
