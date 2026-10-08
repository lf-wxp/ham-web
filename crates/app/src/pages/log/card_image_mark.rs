//! 日志列表里的「有卡片影像」标记。
//!
//! 只读 localStorage 里的 id 索引当**提示**：清过 IndexedDB 时可能「有标记没图」，
//! 用户点进编辑页看不到缩略图就会重新上传；反过来不会出现（没有索引就没有标记）。
//! 影像本体与上传逻辑见 [`super::qsl_image`]。

use leptos::prelude::*;

use crate::i18n::t;
use crate::icons::{Icon, IconKind};

use super::use_log_store;

/// 列表里的「有卡片影像」标记（只读索引，不发异步查询）。
#[component]
pub(super) fn CardImageMark(entry_id: u64) -> impl IntoView {
  let store = use_log_store();
  view! {
    <Show when=move || store.qsl_images.with(|ids| ids.contains(&entry_id))>
      <span
        role="img"
        aria-label=move || t("log.card-image-on-file")
        class="inline-flex align-middle text-muted-foreground"
      >
        <Icon kind=IconKind::Camera />
      </span>
    </Show>
  }
}
