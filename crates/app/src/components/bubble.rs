//! 按钮上方的轻提示气泡，默认 2 秒后自动消失。

use std::time::Duration;

use leptos::prelude::*;

use crate::i18n::t;
use crate::icons::{Icon, IconKind};

#[component]
pub fn Bubble(open: RwSignal<bool>, #[prop(into)] text: Signal<String>) -> impl IntoView {
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

  move || {
    open.get().then(|| {
      view! {
        <div
          class="motion-fade absolute -top-2 left-0 -translate-y-full z-50 rounded-md border bg-popover text-foreground shadow px-3 py-2 text-xs"
          role="status"
          aria-live="polite"
        >
          <div class="flex items-start gap-2">
            <div class="min-w-0">{move || text.get()}</div>
            <button
              type="button"
              on:click=move |_| open.set(false)
              class="opacity-70 hover:opacity-100 transition-opacity"
              aria-label=move || t("关闭提示")
            >
              <Icon kind=IconKind::X class="h-3.5 w-3.5" />
            </button>
          </div>
          <div
            class="absolute left-4 -bottom-2 w-0 h-0"
            aria-hidden="true"
            style="border-left: 6px solid transparent; border-right: 6px solid transparent; border-top: 6px solid hsl(var(--popover)); filter: drop-shadow(0 1px 0 rgba(0,0,0,0.06));"
          ></div>
        </div>
      }
    })
  }
}
