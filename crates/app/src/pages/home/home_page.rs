//! 首页入口：按当前界面风格派发到像素版 / 现代版。
//!
//! 这是迁移期**唯一允许**出现「风格分支」的地方（路由根组件），见
//! `docs/ui/decisions.md` §4：页面内容本身不写分支，两套实现各自独立。
//! 其余页面在 S3 迁到语义槽之后，这里会退化成一次普通的槽调用。

use leptos::prelude::*;

#[component]
pub fn HomePage() -> impl IntoView {
  view! {
    {move || {
      if crate::kit::is_modern() {
        view! { <crate::kit_modern::ModernHome /> }.into_any()
      } else {
        view! { <super::pixel_home::PixelHomePage /> }.into_any()
      }
    }}
  }
}
