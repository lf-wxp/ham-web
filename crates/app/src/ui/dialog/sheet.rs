use leptos::portal::Portal;
use leptos::prelude::*;

use crate::icons::{Icon, IconKind};

use crate::focus::{use_escape_close, use_modal_focus};
use crate::i18n::t;
use crate::presence::{aria_hidden_attr, inert_attr, state_attr};

/// 右侧抽屉（答题卡）。
#[component]
pub fn Sheet(open: RwSignal<bool>, children: ChildrenFn) -> impl IntoView {
  let mounted = crate::presence::use_presence(open).visible;
  // 模态三件套（焦点 / 滚动锁 / Esc）走共享实现：`check-structure` 的 C7.8 守着。
  use_modal_focus(open);
  crate::overlay::use_scroll_lock(open);
  use_escape_close(open);
  let state = state_attr(open);
  let inert = inert_attr(open);
  let hidden = aria_hidden_attr(open);
  move || {
    mounted.get().then(|| {
      let children = children.clone();
      view! {
        <Portal>
          <div
            data-slot="sheet-overlay"
            data-state=state
            inert=inert
            aria-hidden=hidden
            class="pxl-overlay pxl-enter-fade data-[state=closed]:pointer-events-none fixed inset-0 z-50"
            // 用 pointerdown 而不是 click 关闭：`MobileNavDrawer` 在 pointerup 里打开抽屉
            //（区分轻点 / 拖动），随后浏览器派发同一次手势的合成 click —— 那时遮罩已
            // 盖住触发按钮，click 的命中测试落在遮罩上，会立刻把刚打开的抽屉关掉。
            // click 是旧手势的残余事件，pointerdown 才是新手势，这里只认新手势。
            on:pointerdown=move |_| open.set(false)
          ></div>
          <div
            data-modal-root=""
            role="dialog"
            aria-modal="true"
            tabindex="-1"
            data-slot="sheet-content"
            data-state=state
            inert=inert
            aria-hidden=hidden
            class="pxl-sheet data-[state=closed]:pointer-events-none data-[state=open]:animate-in data-[state=closed]:animate-out fill-mode-both fixed z-50 flex flex-col gap-4 data-[state=closed]:duration-(--motion-normal) data-[state=open]:duration-(--motion-normal) data-[state=closed]:slide-out-to-right data-[state=open]:slide-in-from-right inset-y-0 right-0 h-full w-3/4 sm:max-w-sm"
          >
            {children()}
            <super::toast_live_region::ToastLiveRegion />
            <button
              type="button"
              data-state=state
              class="pxl-btn pxl-btn-destructive absolute top-2 right-2 size-8 px-0"
              on:click=move |_| open.set(false)
            >
              <Icon kind=IconKind::X class="size-6" />
              <span class="sr-only">{move || t("log.close")}</span>
            </button>
          </div>
        </Portal>
      }
    })
  }
}
