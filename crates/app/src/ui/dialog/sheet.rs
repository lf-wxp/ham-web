use leptos::html;
use leptos::portal::Portal;
use leptos::prelude::*;

use crate::icons::{Icon, IconKind};

use super::shared::{on_open, state_attr, trap_tab, use_presence};
use crate::i18n::t;

/// 退场动画时长（毫秒）：与抽屉类名里的 `duration-300` 对应，卸载前要等它播完。
const EXIT_MS: u64 = 300;

/// 右侧抽屉（答题卡）。
#[component]
pub fn Sheet(open: RwSignal<bool>, children: ChildrenFn) -> impl IntoView {
  let mounted = use_presence(open, EXIT_MS);
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
            class="pxl-overlay pxl-enter-fade fixed inset-0 z-50"
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
            class="pxl-sheet data-[state=open]:animate-in data-[state=closed]:animate-out fill-mode-both fixed z-50 flex flex-col gap-4 data-[state=closed]:duration-200 data-[state=open]:duration-300 data-[state=closed]:slide-out-to-right data-[state=open]:slide-in-from-right inset-y-0 right-0 h-full w-3/4 sm:max-w-sm"
          >
            {children()}
            <button
              type="button"
              data-state=state
              class="pxl-btn pxl-btn-destructive absolute top-2 right-2 size-8 px-0"
              on:click=move |_| open.set(false)
            >
              <Icon kind=IconKind::X class="size-5" />
              <span class="sr-only">{move || t("log.close")}</span>
            </button>
          </div>
        </Portal>
      }
    })
  }
}
