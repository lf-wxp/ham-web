//! 移动端菜单面板（顶栏右端的汉堡按钮打开 / 关闭）。
//!
//! 结构是「段标签栏 + 段内容」，不再是三段折叠：
//!
//! - **换段不必回到顶部**：折叠列表滚两屏之后段标题就被卷出视口，只剩底部 Tab 还能开合；
//!   标签栏常驻，随时能换段。
//! - **段内也不再折叠**：手机上「先展开段、再展开分组」要点两次才看得见一个入口。
//!   分组改成小标题 + 网格（`mobile_nav_section.rs`）：手机一列、`sm` 两列、`lg` 三列。
//! - **限高扣掉贴底层**：面板从导航下沿铺到「底部 Tab 的上沿」，最后一行入口不会被
//!   底栏压住（`--fixed-bottom-h` 是 `ui::ScrollBar` 实测写下的同一个值；答题类页面把
//!   底栏换成操作栏时它也自动跟随）。
//!
//! 面板**不锁背景滚动**：它不是模态（没有遮罩、焦点可以正常离开），锁了会让菜单里的
//! 长列表滚不动 —— 这里恰恰要靠滚动才看得完 116 个知识库入口。
//!
//! 每次打开都回到当前页所在的段：菜单的默认视图必须回答「我在哪、附近还有什么」，
//! 上次浏览点过哪里不该跨次生效（与桌面下拉 `GroupMenu` 同一条口径）。
//!
//! 面板里**只有导航**：语言 / 外观在顶栏齿轮的设置对话框里，站内搜索是顶栏那颗常驻图标按钮 ——
//! 同一个动作在菜单里再摆一份，等于给用户两条要走一次才知道区别的路。

use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::i18n::t;
use crate::ui::{Tabs, TabsList, TabsPanel, TabsTrigger};

use super::mobile_menu::{MOBILE_NAV_PANEL_ID, use_mobile_menu};
use super::mobile_nav_section::MobileNavSection;
use super::nav_sections::{NAV_SECTIONS, section_of};

/// 移动端菜单面板。
///
/// 只在菜单打开时挂载（`nav_bar.rs` 里 `menu_open.get().then(…)`）：关掉即卸载，
/// 面板里的百来个条目不会在每次进页面时也建一遍。
#[component]
pub(super) fn MobileNavPanel() -> impl IntoView {
  let location = use_location();
  let menu_open = use_mobile_menu().open;

  // 当前页所在的段：面板「关了就卸载」，所以这里算出来的就是本次打开时的值，
  // 不需要额外跟踪路由 —— 点条目会先收起菜单。
  let tab = RwSignal::new(
    section_of(&location.pathname.get_untracked())
      .slug
      .to_owned(),
  );
  let pathname = Signal::derive(move || location.pathname.get());

  view! {
    // `nav` 保留成面板根：它是「导航」这个地标（读屏按地标跳，e2e 也按这个名字定位）。
    <nav
      id=MOBILE_NAV_PANEL_ID
      aria-label=move || t("shell.mobile-navigation")
      class="motion-pop absolute inset-x-0 top-full z-[var(--z-bar)] flex max-h-[calc(100svh-var(--nav-h,66px)-var(--fixed-bottom-h,0px))] flex-col border-b-2 border-ink bg-card shadow-[0_4px_0_0_var(--pxl-shadow)] md:hidden"
    >
      <Tabs
        value=Signal::derive(move || tab.get())
        on_change=Callback::new(move |v: String| tab.set(v))
        class="flex min-h-0 flex-1 flex-col"
      >
        // 标签栏与页内 Tabs 同一套记号（`.pxl-tablist` / `.pxl-tab`）：抬起的板 + 主色下沿。
        // `flex-nowrap` 覆盖 `.pxl-tablist` 的折行：320px 下西语「Centro de exámenes」放不下，
        // 宁可让标签栏横滑，也不要折成两行 —— 折行会把下面的内容区挤掉一整条。
        // 这里**不传 `aria_label`**：外层 `<nav>` 已经叫「移动端导航」，标签栏再叫同一个名字
        // 会让读屏连报两次同一个地标名。三个段的文字本身就是这组标签的名称。
        <TabsList class="shrink-0 flex-nowrap overflow-x-auto px-4 pt-2">
          {NAV_SECTIONS
            .iter()
            .map(|s| {
              view! {
                <TabsTrigger value=s.slug class="flex-1">
                  {move || t(s.label)}
                </TabsTrigger>
              }
            })
            .collect_view()}
        </TabsList>

        // 每段自己滚：切段时新面板从顶部开始，不继承上一段滚了多远。
        {NAV_SECTIONS
          .iter()
          .map(|s| {
            view! {
              <TabsPanel
                value=s.slug
                class="min-h-0 flex-1 overflow-y-auto overscroll-contain px-4 py-3"
              >
                <MobileNavSection
                  section=s
                  pathname=pathname
                  on_navigate=Callback::new(move |()| menu_open.set(false))
                />
              </TabsPanel>
            }
          })
          .collect_view()}
      </Tabs>
    </nav>
  }
}
