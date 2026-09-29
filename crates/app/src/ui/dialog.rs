//! 模态对话框与侧边抽屉（对应 Radix Dialog / shadcn Sheet）。
//!
//! 行为：打开时挂载到 `<body>`、锁定页面滚动、聚焦首个可交互元素；Esc 或点击遮罩关闭；
//! 关闭时保留 `data-state="closed"` 一段时间以播放退出动画。

use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use leptos::ev;
use leptos::html;
use leptos::portal::Portal;
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};
use crate::util::body;

thread_local! {
  static SCROLL_LOCKS: Cell<u32> = const { Cell::new(0) };
}

fn lock_scroll(on: bool) {
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

fn focus_first(root: &web_sys::Element) {
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
fn use_presence(open: RwSignal<bool>, exit_ms: u64) -> RwSignal<bool> {
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

fn state_attr(open: RwSignal<bool>) -> impl Fn() -> &'static str + Copy + Send + Sync {
  move || if open.get() { "open" } else { "closed" }
}

/// 居中模态对话框。
///
/// `class` 会与默认类名合并（可覆盖 `max-w-*`、`sm:max-w-*` 等）。
#[component]
pub fn Dialog(
  open: RwSignal<bool>,
  #[prop(optional, into)] class: String,
  #[prop(optional, default = true)] show_close: bool,
  children: ChildrenFn,
) -> impl IntoView {
  let mounted = use_presence(open, 200);
  let class = cn(&[
    "bg-background data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 fixed left-1/2 top-4 z-50 flex flex-col w-full max-w-[calc(100%-2rem)] -translate-x-1/2 translate-y-0 gap-4 rounded-lg border p-4 shadow-lg duration-200 sm:max-w-lg sm:top-1/2 sm:translate-y-[-50%] sm:p-6 max-h-[calc(100svh-2rem)] overflow-hidden min-h-0",
    &class,
  ]);
  let state = state_attr(open);

  move || {
    mounted.get().then(|| {
      let children = children.clone();
      let class = class.clone();
      let content = NodeRef::<html::Div>::new();
      content.on_load(move |el| {
        request_animation_frame(move || focus_first(&el));
      });
      view! {
        <Portal>
          <div
            data-slot="dialog-overlay"
            data-state=state
            class="data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 fixed inset-0 z-50 bg-black/50"
            on:click=move |_| open.set(false)
          ></div>
          <div
            node_ref=content
            role="dialog"
            aria-modal="true"
            tabindex="-1"
            data-slot="dialog-content"
            data-state=state
            class=class.clone()
          >
            {children()}
            {show_close
              .then(|| {
                view! {
                  <button
                    type="button"
                    data-slot="dialog-close"
                    data-state=state
                    class="ring-offset-background focus:ring-ring data-[state=open]:bg-accent data-[state=open]:text-muted-foreground absolute top-4 right-4 rounded-xs opacity-70 transition-opacity hover:opacity-100 focus:ring-2 focus:ring-offset-2 focus:outline-hidden disabled:pointer-events-none [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4"
                    on:click=move |_| open.set(false)
                  >
                    <Icon kind=IconKind::X />
                    <span class="sr-only">"Close"</span>
                  </button>
                }
              })}
          </div>
        </Portal>
      }
    })
  }
}

/// 右侧抽屉（答题卡）。
#[component]
pub fn Sheet(open: RwSignal<bool>, children: ChildrenFn) -> impl IntoView {
  let mounted = use_presence(open, 300);
  let state = state_attr(open);
  move || {
    mounted.get().then(|| {
      let children = children.clone();
      let content = NodeRef::<html::Div>::new();
      content.on_load(move |el| {
        request_animation_frame(move || focus_first(&el));
      });
      view! {
        <Portal>
          <div
            data-slot="sheet-overlay"
            data-state=state
            class="data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 fixed inset-0 z-50 bg-black/50"
            on:click=move |_| open.set(false)
          ></div>
          <div
            node_ref=content
            role="dialog"
            aria-modal="true"
            tabindex="-1"
            data-slot="sheet-content"
            data-state=state
            class="bg-background data-[state=open]:animate-in data-[state=closed]:animate-out fixed z-50 flex flex-col gap-4 shadow-lg transition ease-in-out data-[state=closed]:duration-300 data-[state=open]:duration-500 data-[state=closed]:slide-out-to-right data-[state=open]:slide-in-from-right inset-y-0 right-0 h-full w-3/4 border-l sm:max-w-sm"
          >
            {children()}
            <button
              type="button"
              data-state=state
              class="ring-offset-background focus:ring-ring data-[state=open]:bg-secondary absolute top-4 right-4 rounded-xs opacity-70 transition-opacity hover:opacity-100 focus:ring-2 focus:ring-offset-2 focus:outline-hidden disabled:pointer-events-none"
              on:click=move |_| open.set(false)
            >
              <Icon kind=IconKind::X class="size-4" />
              <span class="sr-only">"Close"</span>
            </button>
          </div>
        </Portal>
      }
    })
  }
}

/// 对话框头部。
#[component]
pub fn DialogHeader(children: Children) -> impl IntoView {
  view! {
    <div data-slot="dialog-header" class="flex flex-col gap-2 text-center sm:text-left">
      {children()}
    </div>
  }
}

/// 对话框底部按钮区。
#[component]
pub fn DialogFooter(children: Children) -> impl IntoView {
  view! {
    <div data-slot="dialog-footer" class="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end shrink-0">
      {children()}
    </div>
  }
}

/// 对话框标题。
#[component]
pub fn DialogTitle(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <h2 data-slot="dialog-title" class=cn(&["text-lg leading-none font-semibold", &class])>
      {children()}
    </h2>
  }
}

/// 对话框描述。
#[component]
pub fn DialogDescription(
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  view! {
    <p data-slot="dialog-description" class=cn(&["text-muted-foreground text-sm", &class])>
      {children()}
    </p>
  }
}
