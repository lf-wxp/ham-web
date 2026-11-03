//! 全局导航：按当前界面风格派发到像素风顶栏或现代风顶栏。
//!
//! 与首页一样，这是迁移期允许出现「风格分支」的外壳组件之一（`docs/ui/decisions.md` §4）：
//! 两套顶栏各有自己的 DOM、类名与交互（像素风是三段下拉 + HUD，现代风是细栏 + 抽屉），
//! 不共享任何标记。

use leptos::prelude::*;

use super::nav_bar::PixelNav;

#[component]
pub fn Navigation() -> impl IntoView {
  view! {
    {move || {
      if crate::kit::is_modern() {
        view! { <crate::kit_modern::ModernNav /> }.into_any()
      } else {
        view! { <PixelNav /> }.into_any()
      }
    }}
  }
}
