//! `status_icon`：从 `bank_selector.rs` 拆出的视图构造函数（一个组件一个文件）。

use leptos::prelude::*;

use crate::icons::{Icon, IconKind};

use super::VersionWithStatus;

pub(super) fn status_icon(v: &VersionWithStatus) -> AnyView {
  match &v.status {
    None => view! { <Icon kind=IconKind::AlertCircle class="h-4 w-4 text-gray-400" /> }.into_any(),
    Some(s) if s.is_available => {
      view! { <Icon kind=IconKind::CheckCircle class="h-4 w-4 text-green-500" /> }.into_any()
    }
    Some(_) => view! { <Icon kind=IconKind::XCircle class="h-4 w-4 text-red-500" /> }.into_any(),
  }
}
