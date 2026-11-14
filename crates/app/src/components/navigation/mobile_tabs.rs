//! 移动端底部 Tab：首页 / 练习 / 考试 / 错题 / 更多。
//!
//! 文案用**白话**而不是关卡隐喻（基地 / 地图 / 图鉴）：底栏是找入口的地方，文字负责让人一眼找到
//! 「错题本」这类功能。每格一颗指示灯（当前项亮），是电台面板的读法；不画图标（`docs/ui/pixel.md` §3.4）。
//!
//! 只在小屏（`<xl`，与顶栏移动菜单同一断点）显示。答题类页面（练习 / 考试 / 回合战）
//! 会设 `body.no-site-footer` —— 它们自己的固定操作栏就贴在底部，Tab 让位（见 `surfaces.css`）。

use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::cn::cn;
use crate::i18n::t;
use crate::ui::{Led, LedState, LedTone};

use super::mobile_menu::{MOBILE_NAV_PANEL_ID, use_mobile_menu};

/// 一个 Tab。
#[derive(Clone, Copy)]
struct Tab {
  href: &'static str,
  /// 词条 key（`shell.tab-*`）。
  label: &'static str,
  /// `true` 时该 Tab 只切换顶栏的移动菜单，不导航。
  opens_menu: bool,
}

const TABS: &[Tab] = &[
  Tab {
    href: "/",
    label: "shell.home",
    opens_menu: false,
  },
  Tab {
    href: "/map",
    label: "shell.practice",
    opens_menu: false,
  },
  Tab {
    href: "/exam",
    label: "shell.tab-exam",
    opens_menu: false,
  },
  Tab {
    href: "/bestiary",
    label: "shell.tab-mistakes",
    opens_menu: false,
  },
  Tab {
    href: "/tools",
    label: "shell.tab-more",
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
    // `data-fixed-bottom`：窗口滚动条（`ui::ScrollBar`）据此让位，条件见 `docs/ui/pixel.md` §滚动模型。
    <nav
      aria-label=move || t("shell.mobile-tabs")
      data-fixed-bottom
      class="mobile-tabs fixed inset-x-0 bottom-0 z-[var(--z-bar)] border-t-2 border-ink bg-card xl:hidden"
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
                "flex w-full flex-col items-center gap-1 border-t-4 pt-2.5 pb-2 text-xs",
                // 激活态 = 波段键按进去：顶部 4px 主色灯条 + 凹槽底 + 灯亮（形状与灯两个通道，不只靠字色；
                // Game Boy / CGA 这类强约束方案里 primary 与 muted 的字色差很小）。不画图标：
                // 入口靠白话文字（`docs/ui/pixel.md` §3.4），灯是电台面板的读法。
                if active() {
                  "border-primary bg-inset text-foreground"
                } else {
                  "border-transparent text-muted-foreground"
                },
              ])
            };
            let lamp = move || view! { <Led tone=LedTone::Rx state=Signal::derive(move || LedState::lit(active())) small=true /> };
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
                      {lamp()}
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
                      {lamp()}
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
