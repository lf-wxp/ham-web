//! 移动端菜单的开合状态。
//!
//! 菜单只有一个开关（顶栏右端的汉堡按钮，见 `nav_bar.rs`）；状态仍提升到这里、由 `App`
//! 注入一次，是因为**底部 Tab 也要读它**：底栏跳转时要顺手把可能开着面板收起来，
//! 否则新页面已经画出来，面板还盖在上面。

use leptos::prelude::*;

/// 移动端导航面板的 DOM id。
///
/// 顶栏汉堡的 `aria-controls` 指向它 —— 面板是条件渲染的，开关与 `nav_bar.rs` 里那个 `id`
/// 必须用同一份常量，改一处不会漏另一处。
pub const MOBILE_NAV_PANEL_ID: &str = "nav-mobile-panel";

#[derive(Clone, Copy)]
pub struct MobileMenu {
  pub open: RwSignal<bool>,
}

impl MobileMenu {
  /// 切换面板；面板被其它交互关掉时不要重新打开（避免「点面板里的链接又弹回来」）。
  pub fn set(self, open: bool) {
    self.open.set(open);
  }
}

/// 注入移动端菜单状态。在 [`crate::app::App`] 里调用一次。
pub fn provide_mobile_menu() -> MobileMenu {
  let ctx = MobileMenu {
    open: RwSignal::new(false),
  };
  provide_context(ctx);
  ctx
}

/// 取出移动端菜单状态。
#[must_use]
pub fn use_mobile_menu() -> MobileMenu {
  expect_context::<MobileMenu>()
}
