use leptos::portal::Portal;
use leptos::prelude::*;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};

use super::shared::{aria_hidden_attr, inert_attr, state_attr};
use crate::focus::{use_escape_close, use_modal_focus};
use crate::i18n::t;

/// 居中模态对话框。
///
/// `class` 会与默认类名合并（可覆盖 `max-w-*`、`sm:max-w-*` 等）。
#[component]
pub fn Dialog(
  open: RwSignal<bool>,
  #[prop(optional, into)] class: String,
  #[prop(optional, default = true)] show_close: bool,
  /// 无可见标题时的无障碍名称；有标题时会自动用首个标题命名。
  ///
  /// 传**词条 key 或中文原文**（组件内部再 `t()`），不要传 `t(…)` 的结果 ——
  /// 后者只求值一次，切语言后无障碍名会停在旧语言。
  #[prop(optional, into)]
  label: Option<String>,
  children: ChildrenFn,
) -> impl IntoView {
  let mounted = crate::presence::use_presence(open).visible;
  // 模态三件套（焦点 / 滚动锁 / Esc）走共享实现：`check-structure` 的 C7.8 守着。
  use_modal_focus(open);
  crate::overlay::use_scroll_lock(open);
  use_escape_close(open);
  let modern = crate::kit::is_modern_at_mount();
  let class = cn(&[
    // 对话框是整页唯一的「主角」：`.pxl-popover` 窗口（实心底 + 粗描边 + 硬偏移投影）。
    // 入场是 `pop-in`（向上跳 8px、3 帧），不做缩放 —— 缩放会让点阵文字在中间帧变糊。
    // 居中用 `translate`（独立属性）而不是 `transform`，不会与动画的 transform 互相覆盖。
    if modern {
      // 现代风：`.md-dialog-panel` 自带居中（`left/top: 50%` + `translate(-50%,-50%)`）、
      // 圆角、浮层阴影与入场动画；这里只补宽度上限与内距。
      // 层级用 `z-[70]`（= `.md-modal`）：`z-50` 会被移动端 dock（55）盖在遮罩上，还能点穿去导航。
      "md-dialog-panel data-[state=closed]:pointer-events-none fixed left-1/2 top-1/2 z-[70] flex flex-col w-full max-w-[calc(100%-2rem)] -translate-x-1/2 -translate-y-1/2 gap-4 p-4 sm:max-w-lg sm:p-6 max-h-[calc(100svh-2rem)] overflow-hidden min-h-0"
    } else {
      "pxl-popover pxl-enter data-[state=closed]:pointer-events-none fixed left-1/2 top-4 z-50 flex flex-col w-full max-w-[calc(100%-2rem)] -translate-x-1/2 gap-4 p-4 sm:max-w-lg sm:top-1/2 sm:-translate-y-1/2 sm:p-6 max-h-[calc(100svh-2rem)] overflow-hidden min-h-0"
    },
    &class,
  ]);
  let state = state_attr(open);
  let inert = inert_attr(open);
  let hidden = aria_hidden_attr(open);

  move || {
    mounted.get().then(|| {
      let children = children.clone();
      let class = class.clone();
      // `StoredValue` 是 `Copy`：交给属性闭包时不会把 `Option<String>` move 出去
      //（否则闭包变成 `FnOnce`，`children()` 那层 `ChildrenFn` 就不成立了）。
      let label = StoredValue::new(label.clone().unwrap_or_default());
      view! {
        <Portal>
          <div
            data-slot="dialog-overlay"
            data-state=state
            inert=inert
            aria-hidden=hidden
            class=if modern {
              "md-dialog-veil data-[state=closed]:pointer-events-none fixed inset-0 z-[70]"
            } else {
              "pxl-overlay pxl-enter-fade data-[state=closed]:pointer-events-none fixed inset-0 z-50"
            }
            on:click=move |_| open.set(false)
          ></div>
          <div
            data-modal-root=""
            role="dialog"
            aria-modal="true"
            aria-label=move || {
              let l = label.get_value();
              (!l.is_empty()).then(|| t(&l))
            }
            tabindex="-1"
            data-slot="dialog-content"
            data-state=state
            inert=inert
            aria-hidden=hidden
            class=class.clone()
          >
            {children()}
            <super::toast_live_region::ToastLiveRegion />
            {show_close
              .then(|| {
                view! {
                  <button
                    type="button"
                    data-slot="dialog-close"
                    data-state=state
                    class=if modern {
                      "md-icon-btn absolute top-3 right-3"
                    } else {
                      "pxl-btn pxl-btn-destructive absolute top-2 right-2 size-8 px-0 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-5"
                    }
                    on:click=move |_| open.set(false)
                  >
                    <Icon kind=IconKind::X />
                    <span class="sr-only">{move || t("log.close")}</span>
                  </button>
                }
              })}
          </div>
        </Portal>
      }
    })
  }
}
