use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::{Dialog, DialogDescription, DialogHeader, DialogTitle};

/// 题图：缩略图 + 点击放大。
///
/// 只提供「整块缩略图」一种触发器：曾经还有一个 `small_trigger`（塞进答题卡格子的 16px
/// 「图」角标），它必然压住 36px 题号按钮的角上可点区，点偏了就变成开图而不是跳题，已删除。
#[component]
pub fn PreviewableImage(
  #[prop(into)] src: String,
  #[prop(into)] alt: String,
  #[prop(optional, into)] title: Option<String>,
) -> impl IntoView {
  let open = RwSignal::new(false);
  let title = title.unwrap_or_else(|| alt.clone());
  let trigger = view! {
    <div class="relative w-full h-40 sm:h-64">
      <img
        src=src.clone()
        alt=alt.clone()
        loading="lazy"
        decoding="async"
        class="absolute inset-0 h-full w-full object-contain border cursor-zoom-in"
        on:click=move |_| open.set(true)
      />
    </div>
  }
  .into_any();

  let (src, title) = (StoredValue::new(src), StoredValue::new(title));
  view! {
    {trigger}
    <Dialog open=open class="max-w-[90vw]">
      <DialogHeader>
        <DialogTitle class="sr-only">{title.get_value()}</DialogTitle>
        <DialogDescription class="sr-only">{move || t("common.click-outside-or-press")}</DialogDescription>
      </DialogHeader>
      <div class="relative w-full h-[80vh]">
        <img src=src.get_value() alt=title.get_value() class="absolute inset-0 h-full w-full object-contain" />
      </div>
    </Dialog>
  }
}
