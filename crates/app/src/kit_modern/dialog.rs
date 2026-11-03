//! 现代风模态框：暗色磨砂遮罩 + 居中面板 + 右上关闭。
//!
//! 与像素风 `ui::Dialog` 是两份实现（圆角、阴影、遮罩浓度、关闭按钮全不同）。
//! 语义保持一致：`role="dialog"` + `aria-modal` + 可见标题；点击遮罩与 Esc 关闭；
//! 焦点编排走共用的 [`crate::focus::use_modal_focus`]（`aria-modal` 声明了就必须真的
//! 把 Tab 关在面板里，否则读屏被告知「背景 inert」而键盘仍能走进去）。

use leptos::portal::Portal;
use leptos::prelude::*;

use crate::i18n::t;

use super::backdrop::Backdrop;
use super::{MIcon, ModernIcon};
use crate::focus::use_escape_close;
use crate::presence::use_presence;

#[component]
pub fn ModernDialog(
  open: RwSignal<bool>,
  #[prop(into)] title: Signal<String>,
  children: ChildrenFn,
) -> impl IntoView {
  use_escape_close(open);
  crate::overlay::use_scroll_lock(open);
  crate::focus::use_modal_focus(open);

  let children = StoredValue::new(children);
  let presence = use_presence(open);
  let backdrop = Backdrop::new(open);

  view! {
    {move || {
      presence.visible.get().then(|| {
        view! {
          <Portal>
            <div
              class="md-modal"
              data-state=move || { if presence.closing.get() { "closed" } else { "open" } }
              inert=move || presence.closing.get().then_some("")
              aria-hidden=move || presence.closing.get().then_some("true")
              on:pointerdown=backdrop.on_pointerdown()
              on:click=backdrop.on_click()
            >
              <div
                class="md-modal-panel"
                data-modal-root=""
                tabindex="-1"
                role="dialog"
                aria-modal="true"
                aria-label=move || title.get()
              >
                <div class="md-modal-head">
                  <h2 class="md-h2">{move || title.get()}</h2>
                  <button
                    type="button"
                    class="md-icon-btn"
                    aria-label=move || t("log.close")
                    on:click=move |_| open.set(false)
                  >
                    <ModernIcon kind=MIcon::Close />
                  </button>
                </div>
                <div class="md-modal-body">{children.with_value(|c| c())}</div>
              </div>
            </div>
          </Portal>
        }
      })
    }}
  }
}
