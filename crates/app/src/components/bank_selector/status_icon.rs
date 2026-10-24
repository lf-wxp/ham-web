//! `status_icon`：从 `bank_selector.rs` 拆出的视图构造函数（一个组件一个文件）。

use leptos::prelude::*;

use crate::icons::{Icon, IconKind};

use super::VersionWithStatus;

/// 三个状态分支必须用同一档尺寸：它们挤在同一行里随状态切换，尺寸不一致会让行高、
/// 基线在切换时抖动（`size-6` 是图标的标准档，见 `docs/ui-components.md` §2 硬规则 3）。
pub(super) fn status_icon(v: &VersionWithStatus) -> AnyView {
  match &v.status {
    None => {
      view! { <Icon kind=IconKind::AlertCircle class="size-6 text-muted-foreground" /> }.into_any()
    }
    Some(s) if s.is_available => {
      view! { <Icon kind=IconKind::CheckCircle class="size-6 text-win-text" /> }.into_any()
    }
    Some(_) => view! { <Icon kind=IconKind::XCircle class="size-6 text-hp-text" /> }.into_any(),
  }
}
