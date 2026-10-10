//! 移动端菜单的开合状态。
//!
//! 顶栏（`Navigation`）与底部 Tab（`MobileTabs` 的「更多」）都要开关同一个面板，
//! 所以状态提升到这里、由 `App` 注入一次，而不是各自持有一份。

use leptos::prelude::*;

/// 移动端导航面板的 DOM id。
///
/// 顶栏开关与底部 Tab 的「更多」都靠 `aria-controls` 指向它 —— 面板是条件渲染的，
/// 两处开关必须和 `nav_bar.rs` 里那个 `id` 用同一份常量，改一处不会漏另一处。
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
