//! [`TabsPanel`]：一块可被切走的视图。

use leptos::prelude::*;

use crate::cn::cn;

use super::shared::TabsCtx;

/// 标签对应的视图面板。
///
/// 面板内容**第一次被切到时才挂载**，之后常驻，非激活时靠 `hidden` 藏起来：
///
/// - 进页面只建当前那一个面板：此前所有面板一起建好再藏起来，`/morse` 的四个训练器
///   全部挂上，点进页面要多等三个用不到的面板；
/// - 切走再切回来不会丢状态（听抄到一半的题、填了一半的表单）；
/// - 面板外壳（`role="tabpanel"` 的那层）始终在，`aria-controls` 永远指向真实存在的节点。
///
/// 已挂载的非激活面板里的副作用（计时器、音频）照常运行。确实需要「切走就销毁」的面板，
/// 先在这里补一个卸载通道，不要在页面里退回手写。
#[component]
pub fn TabsPanel(
  /// 本面板对应的值（与某个 [`TabsTrigger`](super::TabsTrigger) 的 `value` 相同）。
  #[prop(into)]
  value: String,
  #[prop(optional, into)] class: String,
  children: ChildrenFn,
) -> impl IntoView {
  let ctx = expect_context::<TabsCtx>();
  let panel_id = ctx.panel_id(&value);
  let tab_id = ctx.tab_id(&value);
  let v = StoredValue::new(value);
  let active = move || ctx.is_active(&v);
  // 「曾经激活过」：一旦为真就不再回落，内容挂上之后不会因为切走而卸载。
  // `Memo` 只在值变化时通知，所以下面的闭包只会在假 → 真那一次重跑。
  // 内容用 `ChildrenFn`（可重复调用）而不是 `Children`：即便哪天这个闭包被多触发一次，
  // 最坏是重建一份，而不是把已挂上的面板内容静默换成空白。
  let opened = Memo::new(move |prev: Option<&bool>| prev.copied().unwrap_or(false) || active());
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
      {move || opened.get().then(|| children())}
    </div>
  }
}
