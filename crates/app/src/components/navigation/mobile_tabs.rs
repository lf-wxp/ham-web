//! 移动端底部 Tab：基地 / 地图 / 考试 / 图鉴 / 更多。
//!
//! 只在小屏（`<xl`，与顶栏移动菜单同一断点）显示。答题类页面（练习 / 考试 / 回合战）
//! 会设 `body.no-site-footer` —— 它们自己的固定操作栏就贴在底部，Tab 让位（见 `surfaces.css`）。

use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::cn::cn;
use crate::i18n::t;
use crate::icons::{Icon, IconKind};

use super::mobile_menu::{MOBILE_NAV_PANEL_ID, use_mobile_menu};

/// 一个 Tab。
#[derive(Clone, Copy)]
struct Tab {
  href: &'static str,
  /// 词条 key（`shell.tab-*`）。
  label: &'static str,
  icon: IconKind,
  /// `true` 时该 Tab 只切换顶栏的移动菜单，不导航。
  opens_menu: bool,
}

const TABS: &[Tab] = &[
  Tab {
    href: "/",
    label: "shell.tab-base",
    icon: IconKind::Home,
    opens_menu: false,
  },
  Tab {
    href: "/map",
    label: "shell.tab-map",
    icon: IconKind::Map,
    opens_menu: false,
  },
  Tab {
    href: "/exam",
    label: "shell.tab-exam",
    icon: IconKind::Timer,
    opens_menu: false,
  },
  Tab {
    href: "/bestiary",
    label: "shell.tab-bestiary",
    icon: IconKind::BookMarked,
    opens_menu: false,
  },
  Tab {
    href: "/tools",
    label: "shell.tab-more",
    icon: IconKind::Menu,
    opens_menu: true,
  },
];

/// 某个 Tab 是否为当前页。
fn is_active(pathname: &str, tab: Tab) -> bool {
  if tab.href == "/" {
    return pathname == "/";
  }
  pathname == tab.href || pathname.starts_with(&format!("{}/", tab.href))
}

/// 底部 Tab 栏。
#[component]
pub fn MobileTabs() -> impl IntoView {
  let location = use_location();
  let menu = use_mobile_menu();
  view! {
    <nav
      aria-label=move || t("shell.mobile-tabs")
      class="mobile-tabs fixed inset-x-0 bottom-0 z-40 border-t-2 border-ink bg-card xl:hidden"
      style="padding-bottom: env(safe-area-inset-bottom);"
    >
      <ul class="grid grid-cols-5">
        {TABS
          .iter()
          .copied()
          .map(|tab| {
            // `tab` 是 `Copy`，两个闭包各持一份；`Location::pathname` 也是 `Copy` 的。
            let pathname = location.pathname;
            let active = move || is_active(&pathname.get(), tab);
            let label = move || t(tab.label);
            // 类名与 `aria-current` 都要读 `active`，多造一份闭包比共享引用简单。
            // `w-full`：`<a>` 是块级 flex 会自动撑满 `li`，而 `<button>` 保持内容宽
            // （fit-content），不撑满的话「更多」的图标会贴在格子左缘，五个图标间距不齐。
            let class = move || {
              cn(&[
                "flex w-full flex-col items-center gap-0.5 py-2 text-xs",
                if active() { "text-primary" } else { "text-muted-foreground" },
              ])
            };
            let is_current = move || active();
            view! {
              <li>
                {if tab.opens_menu {
                  view! {
                    <button
                      type="button"
                      class=class
                      aria-expanded=move || menu.open.get().to_string()
                      aria-controls=MOBILE_NAV_PANEL_ID
                      on:click=move |_| menu.set(true)
                    >
                      <Icon kind=tab.icon class="size-6" />
                      <span>{label}</span>
                    </button>
                  }
                    .into_any()
                } else {
                  view! {
                    <a
                      href=tab.href
                      class=class
                      aria-current=move || is_current().then_some("page")
                      on:click=move |_| menu.set(false)
                    >
                      <Icon kind=tab.icon class="size-6" />
                      <span>{label}</span>
                    </a>
                  }
                    .into_any()
                }}
              </li>
            }
          })
          .collect_view()}
      </ul>
    </nav>
  }
}
