//! [`TabsPanel`]：一块可被切走的视图。

use leptos::prelude::*;

use crate::cn::cn;

use super::shared::TabsCtx;

/// 标签对应的视图面板。
///
/// 面板**常驻挂载**，非激活时靠 `hidden` 藏起来：
///
/// - 切走再切回来不会丢状态（听抄到一半的题、填了一半的表单）；
/// - `aria-controls` 永远指向真实存在的节点（卸载面板会留下悬空 IDREF）。
///
/// 代价是非激活面板里的副作用（计时器、音频）照常运行 —— 与迁移前「所有分区都挂在
/// 页面上」完全一致，不是新引入的行为。确实需要「切走就销毁」的面板，先在这里补一个
/// 卸载通道，不要在页面里退回手写。
#[component]
pub fn TabsPanel(
  /// 本面板对应的值（与某个 [`TabsTrigger`](super::TabsTrigger) 的 `value` 相同）。
  #[prop(into)]
  value: String,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let ctx = expect_context::<TabsCtx>();
  let panel_id = ctx.panel_id(&value);
  let tab_id = ctx.tab_id(&value);
  let v = StoredValue::new(value);
  let active = move || ctx.is_active(&v);
  view! {
    <div
      role="tabpanel"
      data-slot="tabs-panel"
      id=panel_id
      aria-labelledby=tab_id
      data-state=move || if active() { "active" } else { "inactive" }
      // `hidden` 用「有 / 无属性」表达：写成 `hidden="false"` 反而会藏起来。
      hidden=move || (!active()).then_some("")
      class=cn(&[&class])
    >
      {children()}
    </div>
  }
}
