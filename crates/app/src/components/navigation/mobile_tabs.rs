//! 移动端底部 Tab：首页 / 练习 / 考试 / 错题。
//!
//! 文案用**白话**而不是关卡隐喻（基地 / 地图 / 图鉴）：底栏是找入口的地方，文字负责让人一眼找到
//! 「错题本」这类功能。每格一颗指示灯（当前项亮），是电台面板的读法；不画图标（`docs/ui/pixel.md` §3.4）。
//!
//! 只在小屏（`<md`，手机）显示：平板起顶栏里已经有三个分组下拉（`nav_bar.rs`），
//! 底栏再摆一份同样的入口就是两套导航。答题类页面（练习 / 考试 / 回合战）会设
//! `body.no-site-footer` —— 它们自己的固定操作栏就贴在底部，Tab 让位（见 `surfaces.css`）。
//!
//! 底栏里**没有「更多」**：全站菜单的唯一入口是顶栏右端的汉堡按钮（`nav_bar.rs`）。
//! 两处都能开同一个面板时，用户得先猜哪个「更多」是想找的那个；而底栏在答题类页面上会被
//! 整条隐藏，只有顶栏那颗在每种页面上都在。

use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::cn::cn;
use crate::i18n::t;
use crate::ui::{Led, LedState, LedTone};

use super::mobile_menu::use_mobile_menu;

/// 一个 Tab。
#[derive(Clone, Copy)]
struct Tab {
  href: &'static str,
  /// 词条 key（`shell.tab-*` / `shell.home` 等）。
  label: &'static str,
}

const TABS: &[Tab] = &[
  Tab {
    href: "/",
    label: "shell.home",
  },
  Tab {
    href: "/map",
    label: "shell.practice",
  },
  Tab {
    href: "/exam",
    label: "shell.tab-exam",
  },
  Tab {
    href: "/bestiary",
    label: "shell.tab-mistakes",
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
  // 顶栏的移动菜单可能正开着（点汉堡打开后没关）：底栏跳转时顺手收起它，
  // 否则新页面已经画出来了，面板还盖在上面。
  let menu = use_mobile_menu();
  view! {
    // `data-fixed-bottom`：窗口滚动条（`ui::ScrollBar`）据此让位，条件见 `docs/ui/pixel.md` §滚动模型。
    <nav
      aria-label=move || t("shell.mobile-tabs")
      data-fixed-bottom
      class="mobile-tabs fixed inset-x-0 bottom-0 z-[var(--z-bar)] border-t-2 border-ink bg-card md:hidden"
      style="padding-bottom: env(safe-area-inset-bottom);"
    >
      <ul class="grid grid-cols-4">
        {TABS
          .iter()
          .copied()
          .map(|tab| {
            // `tab` 是 `Copy`，两个闭包各持一份；`Location::pathname` 也是 `Copy` 的。
            let pathname = location.pathname;
            let active = move || is_active(&pathname.get(), tab);
            let label = move || t(tab.label);
            // 类名与 `aria-current` 都要读 `active`，多造一份闭包比共享引用简单。
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
                <a
                  href=tab.href
                  class=class
                  aria-current=move || is_current().then_some("page")
                  on:click=move |_| menu.set(false)
                >
                  {lamp()}
                  <span>{label}</span>
                </a>
              </li>
            }
          })
          .collect_view()}
      </ul>
    </nav>
  }
}
