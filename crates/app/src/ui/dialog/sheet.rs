use leptos::html;
use leptos::portal::Portal;
use leptos::prelude::*;

use crate::icons::{Icon, IconKind};

use super::shared::{on_open, state_attr, trap_tab, use_presence};

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
        request_animation_frame(move || on_open(&el));
      });
      view! {
        <Portal>
          <div
            data-slot="sheet-overlay"
            data-state=state
            class="data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 fill-mode-both fixed inset-0 z-50 bg-black/50"
            on:click=move |_| open.set(false)
          ></div>
          <div
            node_ref=content
            role="dialog"
            aria-modal="true"
            tabindex="-1"
            on:keydown=move |e| {
              if let Some(el) = content.get_untracked() {
                trap_tab(&e, &el);
              }
            }
            data-slot="sheet-content"
            data-state=state
            class="bg-background data-[state=open]:animate-in data-[state=closed]:animate-out fill-mode-both fixed z-50 flex flex-col gap-4 shadow-lg transition ease-in-out data-[state=closed]:duration-300 data-[state=open]:duration-500 data-[state=closed]:slide-out-to-right data-[state=open]:slide-in-from-right inset-y-0 right-0 h-full w-3/4 border-l sm:max-w-sm"
          >
            {children()}
            <button
              type="button"
              data-state=state
              class="ring-offset-background focus-visible:ring-ring data-[state=open]:bg-secondary absolute top-4 right-4 rounded-xs opacity-70 transition-opacity hover:opacity-100 focus-visible:ring-2 focus-visible:ring-offset-2 outline-none disabled:pointer-events-none"
              on:click=move |_| open.set(false)
            >
              <Icon kind=IconKind::X class="size-4" />
              <span class="sr-only">"关闭"</span>
            </button>
          </div>
        </Portal>
      }
    })
  }
}
