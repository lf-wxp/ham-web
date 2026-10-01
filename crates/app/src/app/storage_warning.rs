//! 本地存储写入失败（配额已满）时的全局提示条。

use leptos::prelude::*;

use crate::ui::{Size, Variant, button_class};
use crate::util::storage::WRITE_FAILED_EVENT;

#[component]
pub(super) fn StorageWarning() -> impl IntoView {
  let open = RwSignal::new(false);
  let handle = window_event_listener_untyped(WRITE_FAILED_EVENT, move |_| open.set(true));
  on_cleanup(move || handle.remove());

  move || {
    open.get().then(|| {
      view! {
        <div
          role="alert"
          class="fixed inset-x-0 bottom-0 z-50 border-t border-red-500/40 bg-red-50 px-4 py-3 text-sm text-red-800 shadow-lg dark:bg-red-950 dark:text-red-200"
        >
          <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3">
            <span class="mr-auto">
              "本地存储空间已满，最新的进度 / 错题 / 日志可能没有保存。请先导出备份，再清理不需要的数据。"
            </span>
            <a href="/tools#backup" class=button_class(Variant::Destructive, Size::Sm, "")>
              "查看占用并备份"
            </a>
            <button
              type="button"
              class=button_class(Variant::Ghost, Size::Sm, "")
              on:click=move |_| open.set(false)
            >
              "知道了"
            </button>
          </div>
        </div>
      }
    })
  }
}
