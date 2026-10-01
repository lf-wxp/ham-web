use leptos::html;
use leptos::portal::Portal;
use leptos::prelude::*;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};

use super::shared::{on_open, state_attr, trap_tab, use_presence};

/// 居中模态对话框。
///
/// `class` 会与默认类名合并（可覆盖 `max-w-*`、`sm:max-w-*` 等）。
#[component]
pub fn Dialog(
  open: RwSignal<bool>,
  #[prop(optional, into)] class: String,
  #[prop(optional, default = true)] show_close: bool,
  /// 无可见标题时的无障碍名称；有标题时会自动用首个标题命名。
  #[prop(optional, into)]
  label: Option<String>,
  children: ChildrenFn,
) -> impl IntoView {
  let mounted = use_presence(open, 200);
  let class = cn(&[
    "bg-background data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 fill-mode-both fixed left-1/2 top-4 z-50 flex flex-col w-full max-w-[calc(100%-2rem)] -translate-x-1/2 translate-y-0 gap-4 rounded-lg border p-4 shadow-lg duration-200 sm:max-w-lg sm:top-1/2 sm:translate-y-[-50%] sm:p-6 max-h-[calc(100svh-2rem)] overflow-hidden min-h-0",
    &class,
  ]);
  let state = state_attr(open);

  move || {
    mounted.get().then(|| {
      let children = children.clone();
      let class = class.clone();
      let label = label.clone();
      let content = NodeRef::<html::Div>::new();
      content.on_load(move |el| {
        request_animation_frame(move || on_open(&el));
      });
      view! {
        <Portal>
          <div
            data-slot="dialog-overlay"
            data-state=state
            class="data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 fill-mode-both fixed inset-0 z-50 bg-black/50"
            on:click=move |_| open.set(false)
          ></div>
          <div
            node_ref=content
            role="dialog"
            aria-modal="true"
            aria-label=label.clone()
            tabindex="-1"
            on:keydown=move |e| {
              if let Some(el) = content.get_untracked() {
                trap_tab(&e, &el);
              }
            }
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
                    class="ring-offset-background focus-visible:ring-ring data-[state=open]:bg-accent data-[state=open]:text-muted-foreground absolute top-4 right-4 rounded-xs opacity-70 transition-opacity hover:opacity-100 focus-visible:ring-2 focus-visible:ring-offset-2 outline-none disabled:pointer-events-none [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4"
                    on:click=move |_| open.set(false)
                  >
                    <Icon kind=IconKind::X />
                    <span class="sr-only">"关闭"</span>
                  </button>
                }
              })}
          </div>
        </Portal>
      }
    })
  }
}
