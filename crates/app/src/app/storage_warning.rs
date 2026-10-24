//! 本地存储写入失败（配额已满）时的全局提示条。

use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::{Button, ButtonLink, Size, Variant};
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
          class="fixed inset-x-0 bottom-0 z-50 border-t border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-hp-text shadow-lg"
        >
          <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3">
            <span class="mr-auto">
              {move || t("shell.local-storage-is-full")}
            </span>
            <ButtonLink
              href="/backup"
              variant=Variant::Destructive
              size=Size::Sm
            >
              {move || t("shell.view-usage-and-back")}
            </ButtonLink>
            <Button
              variant=Variant::Ghost
              size=Size::Sm
              on_click=Callback::new(move |_| open.set(false))
            >
              {move || t("exam.ok")}
            </Button>
          </div>
        </div>
      }
    })
  }
}
