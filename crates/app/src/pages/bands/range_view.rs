//! `range_view`：从 `mod.rs` 拆出的视图构造函数（一个组件一个文件）。

use ham_web_core::bands::Allocation;
use leptos::prelude::*;

use crate::i18n::t;
use crate::icons::{Icon, IconKind};

use super::SAT_ICON;

pub(super) fn range_view(a: &'static Allocation) -> impl IntoView {
  view! {
    <span class="inline-flex items-center gap-1 whitespace-nowrap">
      {a.is_satellite()
        .then(|| {
          view! {
            <Icon kind=IconKind::Satellite class=SAT_ICON />
            <span class="sr-only">{move || t("knowledge.amateur-satellite-service")}</span>
          }
        })}
      <span class="tabular-nums">{a.range}</span>
    </span>
  }
}
