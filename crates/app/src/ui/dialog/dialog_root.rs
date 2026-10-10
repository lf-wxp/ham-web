use leptos::html;
use leptos::portal::Portal;
use leptos::prelude::*;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};

use super::shared::{on_open, state_attr, trap_tab, use_presence};
use crate::i18n::t;

/// 退场动画时长（毫秒）：与类名里的 `duration-200` 对应 —— 卸载得晚于动画结束，
/// 否则退场会被截断。
const EXIT_MS: u64 = 200;

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
  let mounted = use_presence(open, EXIT_MS);
  let class = cn(&[
    // 对话框是整页唯一的「主角」：`.pxl-popover` 窗口（实心底 + 粗描边 + 硬偏移投影）。
    // 入场是 `pop-in`（向上跳 8px、3 帧），不做缩放 —— 缩放会让点阵文字在中间帧变糊。
    // 居中用 `translate`（独立属性）而不是 `transform`，不会与动画的 transform 互相覆盖。
    "pxl-popover pxl-enter fixed left-1/2 top-4 z-50 flex flex-col w-full max-w-[calc(100%-2rem)] -translate-x-1/2 gap-4 p-4 sm:max-w-lg sm:top-1/2 sm:-translate-y-1/2 sm:p-6 max-h-[calc(100svh-2rem)] overflow-hidden min-h-0",
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
            class="pxl-overlay pxl-enter-fade fixed inset-0 z-50"
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
                    class="pxl-btn pxl-btn-destructive absolute top-2 right-2 size-8 px-0 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-5"
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
