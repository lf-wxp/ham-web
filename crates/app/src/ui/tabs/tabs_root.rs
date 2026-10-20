//! [`Tabs`]：页内视图切换的根，持有当前激活值并向子组件提供上下文。

use leptos::prelude::*;

use crate::cn::cn;
use crate::util::unique_id;

use super::shared::TabsCtx;

/// 页内视图切换的根组件。
///
/// 受控组件：`value` 由调用方持有，`on_change` 收到的是**新值**（不是事件）。
/// 什么时候该用 Tabs、什么时候该用 `ChipGroup`，见 [`super`] 的模块文档。
#[component]
pub fn Tabs(
  /// 当前激活的 `value`（与某个 [`TabsTrigger`](super::TabsTrigger) /
  /// [`TabsPanel`](super::TabsPanel) 的 `value` 相同）。
  #[prop(into)]
  value: Signal<String>,
  /// 选中变化。回调收新值。
  on_change: Callback<String>,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  provide_context(TabsCtx {
    value,
    on_change,
    // 每个 Tabs 一份 id 前缀：同页多个 Tabs 的 tab / panel id 不会撞。
    id_base: StoredValue::new(unique_id("tabs")),
  });
  view! {
    // 刻意不加 `role="tablist"` 之外的角色：`TabsList` 才是 tablist，根只是排版容器。
    <div data-slot="tabs" class=cn(&[&class])>{children()}</div>
  }
}
