//! 「更多」目录面板里的一个入口（`menu.rs` 分块挂载时逐个调用）。

use ham_web_core::registry::Module;
use leptos::prelude::*;

use crate::data::core_text_tracked;

use super::ModernIcon;
use super::module_icon::module_icon;

/// 一个目录入口：图标 + 页面名，点击交给 `follow`（先收起面板、画完这一帧再导航）。
pub(super) fn menu_item(
  m: &'static Module,
  follow: Callback<(web_sys::MouseEvent, &'static str)>,
) -> impl IntoView {
  let href = m.nav_href();
  view! {
    <a class="md-menu-item" href=href on:click=move |ev| follow.run((ev, href))>
      <ModernIcon kind=module_icon(m) class="md-menu-icon-sm" />
      // `move ||`：译文自己订阅语言。直接写 `{core_text_tracked(..)}` 会在构造时求值，
      // 订阅落到外层的分块闭包上，切语言就把整块入口（含 SVG、悬停与焦点）重建一遍。
      {move || core_text_tracked(m.title)}
    </a>
  }
}
