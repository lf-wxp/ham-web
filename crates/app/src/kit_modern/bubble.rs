//! 现代风轻提示气泡：小圆角卡 + 关闭按钮，2 秒自动消失（计时逻辑与像素版一致）。

use std::time::Duration;

use leptos::prelude::*;

use crate::i18n::t;

use super::{MIcon, ModernIcon};
use crate::presence::use_presence;

#[component]
pub fn ModernBubble(open: RwSignal<bool>, #[prop(into)] text: Signal<String>) -> impl IntoView {
  let presence = use_presence(open);
  let timer: StoredValue<Option<TimeoutHandle>> = StoredValue::new(None);
  Effect::new(move |_| {
    if open.get() {
      if let Some(h) = timer.get_value() {
        h.clear();
      }
      let h = set_timeout_with_handle(
        move || open.try_set(false).map_or((), |_| ()),
        Duration::from_millis(2000),
      )
      .ok();
      timer.set_value(h);
    }
  });
  on_cleanup(move || {
    if let Some(Some(h)) = timer.try_get_value() {
      h.clear();
    }
  });

  view! {
    {move || {
      presence.visible.get().then(|| {
        view! {
          <div
            class="md-bubble"
            data-state=presence.state()
            inert=presence.inert()
            aria-hidden=presence.aria_hidden()
            role="status"
            aria-live="polite"
          >
            <span class="md-bubble-text">{move || text.get()}</span>
            <button
              type="button"
              class="md-bubble-close"
              aria-label=move || t("learning.dismiss")
              on:click=move |_| open.set(false)
            >
              <ModernIcon kind=MIcon::Close />
            </button>
          </div>
        }
      })
    }}
  }
}
