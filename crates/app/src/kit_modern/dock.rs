//! 现代风移动端底部导航（dock）。
//!
//! 为什么需要：顶栏的链接在 `<768px` 会被隐藏，之前只剩「设置 / 更多」两个图标，
//! 等于窄屏上没有主导航。现在手机改用底部 dock（5 个一级入口），顶栏只留品牌与设置。
//!
//! 768 起导航回到顶栏（平板竖屏放得下 5 项，塞进底部 dock 等于「把手机界面放大」）；
//! 900 起顶栏显示完整 6 项。dock 与顶栏导航互斥，同一个断点上只出现一套。
//! 练习页与考试页会主动收起 dock（见 `kit.css` 的 `body[data-page-mode]`，标记由 `page_mode::use_page_mode` 写）。

use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::i18n::t;

use super::{MIcon, ModernIcon};

#[component]
pub fn ModernDock(menu_open: RwSignal<bool>) -> impl IntoView {
  let location = use_location();
  let current = move |href: &'static str| {
    if location.pathname.get() == href {
      "page"
    } else {
      "false"
    }
  };

  view! {
    <nav class="md-dock" aria-label=move || t("shell.main-navigation")>
      <a class="md-dock-item" href="/" aria-current=move || current("/")>
        <ModernIcon kind=MIcon::Home />
        <span>{move || t("shell.home")}</span>
      </a>
      <a class="md-dock-item" href="/practice" aria-current=move || current("/practice")>
        <ModernIcon kind=MIcon::Play />
        <span>{move || t("shell.practice")}</span>
      </a>
      <a class="md-dock-item" href="/reference" aria-current=move || current("/reference")>
        <ModernIcon kind=MIcon::Book />
        <span>{move || t("shell.knowledge")}</span>
      </a>
      <a class="md-dock-item" href="/tools" aria-current=move || current("/tools")>
        <ModernIcon kind=MIcon::Tools />
        <span>{move || t("shell.tools")}</span>
      </a>
      <button
        type="button"
        class="md-dock-item"
        aria-expanded=move || menu_open.get().to_string()
        on:click=move |_| menu_open.update(|v| *v = !*v)
      >
        <ModernIcon kind=MIcon::Menu />
        <span>{move || t("shell.tab-more")}</span>
      </button>
    </nav>
  }
}
