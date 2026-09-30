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

pub(super) fn focus_first(root: &web_sys::Element) {
  let selector = "input:not([disabled]), button:not([disabled]), [href], select, textarea, [tabindex]:not([tabindex='-1'])";
  let target = root
    .query_selector(selector)
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

  let lock = locked.clone();
  Effect::new(move |_| {
    if open.get() {
      mounted.set(true);
      if !lock.swap(true, Ordering::Relaxed) {
        lock_scroll(true);
      }
    } else {
      if lock.swap(false, Ordering::Relaxed) {
        lock_scroll(false);
      }
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
