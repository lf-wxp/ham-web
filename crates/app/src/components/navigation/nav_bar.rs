//! 顶部导航栏。
//!
//! 结构照现代风顶栏（`kit_modern/nav.rs`）来：品牌在左 → 导航项紧跟其后 → 动作组用
//! `ml-auto` 顶到最右。区别只在皮肤与收纳方式（像素风是三个分组下拉，现代风是一串文字链接）。
//!
//! 顶栏里**没有单独的「首页」项**：左边那枚站标就是首页链接。导航里也没有语言 / 明暗 / 备份 ——
//! 这些配置统一收进齿轮打开的设置对话框（`settings_dialog.rs`），顶栏只留
//! 「搜索 + 设置 + HUD」三个常驻入口。顶栏多摆两个切换按钮，就要在 1024–1280 那档与 HUD 抢位置，
//! 而这些操作在设置里本来就有完整形态（尤其「跟随系统」只有三选一给得出来）。
//!
//! 导航项本身只剩三个分组下拉，开关与面板都在 `group_menu.rs`（那里保留原生 `<button>` 的理由
//! 见 `docs/ui/pixel.md` §10.2）；本文件的汉堡、搜索、齿轮都是 [`Button`]。

use leptos::ev;
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;

use ham_web_core::registry::{EXAM_GROUPS, KNOWLEDGE_GROUPS, TOOL_GROUPS};

use crate::components::search_dialog::SearchOpen;
use crate::icons::{Icon, IconKind};
use crate::ui::{Button, Size, Variant};

use super::group_menu::{GroupMenu, MenuAlign};
use super::hud_badge::HudBadge;
use super::settings_dialog::SettingsDialog;

use super::menu::MenuKind;
use super::mobile_menu::{MOBILE_NAV_PANEL_ID, use_mobile_menu};
use super::mobile_nav_section::MobileNavSection;
use crate::i18n::t;

#[component]
pub(crate) fn PixelNav() -> impl IntoView {
  let location = use_location();
  // 移动端菜单状态与底部 Tab 共享：Tab 的「更多」打开的就是这个面板。
  let menu_open = use_mobile_menu().open;
  let open_menu = RwSignal::new(None::<MenuKind>);
  let settings_open = RwSignal::new(false);

  // 点击导航外部时关闭桌面端下拉。
  let outside_handle = window_event_listener(ev::click, move |e| {
    if let Some(el) = e
      .target()
      .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    {
      let inside = el.closest("[data-nav]").ok().flatten().is_some();
      if !inside {
        open_menu.set(None);
      }
    }
  });
  let outside_handle = SendWrapper::new(Some(outside_handle));
  on_cleanup(move || {
    if let Some(h) = outside_handle.take() {
      h.remove();
    }
  });

  // 全局「/」快捷键：非输入框内按下打开站内搜索面板。
  let SearchOpen(search_open) = expect_context::<SearchOpen>();
  let search_handle = window_event_listener(ev::keydown, move |e| {
    if let Some(el) = e
      .target()
      .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok())
    {
      let tag = el.tag_name().to_lowercase();
      if tag == "input" || tag == "textarea" || el.is_content_editable() {
        return;
      }
    }
    if e.key() == "/" {
      e.prevent_default();
      search_open.set(true);
    }
  });
  let search_handle = SendWrapper::new(Some(search_handle));
  on_cleanup(move || {
    if let Some(h) = search_handle.take() {
      h.remove();
    }
  });

  view! {
    // 顶栏是一条实心的「HUD 栏」：底色用 `--card`（不透明）+ 2px 墨色下沿 + 硬偏移投影，
    // 不模糊、不渐变；吸顶时下面滚过的内容被它实实在在地盖住，不需要任何磨砂兜底。
    <nav data-nav aria-label=move || t("shell.main-navigation") class="sticky top-0 z-[var(--z-nav)] border-b-2 border-ink bg-card shadow-[0_2px_0_0_var(--pxl-shadow)]">
      // 内容层与正文同一个版心：`max-w-5xl`（值 = `--page-max`，见 `style/input.css`）+ `px-4`，
      // 与 `PageContainer` / `PageHeader` 逐字一致，所以顶栏两端与正文边缘永远对齐。
      // 以前这里用 `container`（宽度随断点长到 1280）且 <xl 铺满整宽，于是顶栏比正文宽出一截；
      // 现在桌面导航只剩三段下拉、右侧只剩两个图标，1200 的版心在三种语言下都放得下
      // （最长的西语整行约 996px）。`relative` 留给导航项的下拉定位。
      <div class="relative mx-auto max-w-5xl px-4">
        // 排布对齐现代风：品牌在最左，导航项紧跟其后（不是居中），右侧动作组用 `ml-auto` 顶到
        // 最右。三块之间不再用 `justify-between` —— 那样导航项会随两侧宽度浮动，换语言或
        // HUD 数字变长时整组横向漂移。
        <div class="flex h-16 items-center gap-2 sm:gap-3 xl:gap-4">
          // 品牌标识
          <a
            href="/"
            class="group flex shrink-0 items-center gap-2.5 pointer-coarse:min-h-11"
            on:click=move |_| menu_open.set(false)
          >
            <div class="brand-mark flex size-10 items-center justify-center text-primary-foreground group-hover:-translate-y-0.5">
              <Icon kind=IconKind::Satellite class="size-6" />
            </div>
            <div class="hidden flex-col whitespace-nowrap leading-tight sm:flex">
              <span class="pxl-title text-xs text-foreground">{move || t("shell.amateur-radio")}</span>
              // 副标题是装饰，宽度让位于功能：点阵标题字体比正文宽，西语副标题又最长（≈ 240px），
              // 640–768px 与 xl–2xl（桌面导航整行展开、右侧还有 HUD）两段都会把顶栏撑出屏幕。
              <span class="hidden text-xs text-muted-foreground lg:block xl:hidden 2xl:block">
                {move || t("shell.exams-knowledge-tools")}
              </span>
            </div>
          </a>

          // 桌面端导航。没有单独的「首页」项：左边那枚站标本身就是首页链接
          // （与现代风的 `.md-brand` 同一套语义），再摆一个「首页」按钮等于同一目标两个入口。
          // `self-stretch`：撑满 h-16 的导航行，让内部下拉的 `top-full` 能落在导航下沿。
          <div class="hidden items-center gap-1 self-stretch xl:flex">
            // 考试中心下拉：17 条平铺会拉成一条长列表，与知识库 / 工具共用
            // 「分组侧栏 + 条目区」面板（见 `group_menu.rs`）。
            <GroupMenu
              kind=MenuKind::Exam
              groups=EXAM_GROUPS
              label=Signal::derive(move || t("shell.exam-center"))
              icon=IconKind::Timer
              align=MenuAlign::Left
              open_menu=open_menu
            />

            // 知识库下拉：分组侧栏 + 条目区（上百个专题平铺会顶出视口，
            // 结构与高度约束见 `group_menu.rs`）
            <GroupMenu
              kind=MenuKind::Knowledge
              groups=KNOWLEDGE_GROUPS
              label=Signal::derive(move || t("shell.knowledge"))
              icon=IconKind::BookOpen
              align=MenuAlign::Center
              open_menu=open_menu
            />

            // 工具下拉：与知识库同一套面板，只是相对导航容器右对齐
            <GroupMenu
              kind=MenuKind::Tools
              groups=TOOL_GROUPS
              label=Signal::derive(move || t("shell.tools"))
              icon=IconKind::Calculator
              align=MenuAlign::Right
              open_menu=open_menu
            />
          </div>

          // 右侧：搜索 + 设置 + HUD + 移动端菜单按钮。
          // `ml-auto` 把它顶到最右（布局对齐现代风，见上）。
          // `min-w-0`：flex 子项的最小宽度默认取内容的 min-content，不写的话 320px 的西语界面里这一组
          // （搜索 + HUD + 菜单按钮）比可用宽度多 1px，把页面横向撑出 1px。
          // 齿轮不分宽度常驻：它是语言与全部外观设置的唯一入口，手机档也不能少。
          <div class="ml-auto flex min-w-0 items-center gap-1">
            <Button
              variant=Variant::Ghost
              size=Size::Icon
              aria_label=Signal::derive(move || t("shell.search"))
              title=Signal::derive(move || t("shell.search-with-shortcut"))
              on_click=Callback::new(move |_| search_open.set(true))
            >
              <Icon kind=IconKind::Search class="size-6" />
            </Button>
            <Button
              variant=Variant::Ghost
              size=Size::Icon
              aria_label=Signal::derive(move || t("shell.settings"))
              title=Signal::derive(move || t("shell.settings"))
              on_click=Callback::new(move |_| settings_open.set(true))
            >
              <Icon kind=IconKind::Settings class="size-6" />
            </Button>
            <HudBadge />
            <Button
              variant=Variant::Ghost
              size=Size::Icon
              class="xl:hidden"
              aria_label=Signal::derive(move || {
                if menu_open.get() { t("shell.close-menu") } else { t("shell.open-menu") }
              })
              aria_expanded=Signal::derive(move || menu_open.get())
              aria_controls=MOBILE_NAV_PANEL_ID
              on_click=Callback::new(move |_| menu_open.update(|v| *v = !*v))
            >
              {move || {
                if menu_open.get() {
                  view! { <Icon kind=IconKind::X class="size-6" /> }
                } else {
                  view! { <Icon kind=IconKind::Menu class="size-6" /> }
                }
              }}
            </Button>
          </div>
        </div>

      </div>

      // 移动端下拉菜单。挂在 `<nav>` 上、不放进上面那层内容 div 里：那层是 `container`
      // （宽度随断点定死），面板以它为定位祖先，`inset-x-0` 就会被那条 `max-w` 收窄，
      // 左右各空出一条、露出下面的页面。挂在 `<nav>` 下（`sticky` 即定位祖先）铺满整宽，
      // 内容内边距由面板自己的 `px-4` 给（菜单需要缩进，不跟顶栏的 `px-1/2/4` 逐档一致）。
      {move || {
        menu_open.get().then(|| {
          view! {
            <div
              id=MOBILE_NAV_PANEL_ID
              class="motion-pop absolute inset-x-0 top-full z-[var(--z-bar)] max-h-[calc(100svh-4rem)] overflow-y-auto overscroll-contain border-b-2 border-ink bg-card shadow-[0_4px_0_0_var(--pxl-shadow)] xl:hidden"
            >
              // 布局与顺序：**先导航、后一个页面直达入口**，两种宽度共用同一份结构。
              // - `<sm`（手机竖屏）一列：三段折叠依次向下；
              // - `sm`–`xl`（横屏手机 / 平板 / 窄桌面）三列：三段各占一列。
              // 面板里**只有导航**：语言与外观设置在顶栏齿轮里（各宽度都常驻），数据备份在设置
              // 对话框与页脚各有入口 —— 这里再摆一份只是同一条路多一个出口。
              // 首页也不必放：底部 Tab 与顶栏 brand 都直达（见 `mobile_tabs.rs`）。单列铺到
              // 640–1279px 会拉成一条长列表，三列后高度减半。
              //
              // 分隔虚线由这里画（一列时是两条，三列时不画），不放在 `MobileNavSection` 里 ——
              // 三列并排时每段自带的 `border-t` 会断成三段短虚线。
              <nav
                aria-label=move || t("shell.mobile-navigation")
                class="grid grid-cols-1 gap-1 px-4 py-3 sm:grid-cols-3 sm:items-start sm:gap-x-4"
              >
                // 三大段可折叠、当前页所在的那段默认展开（见 `MobileNavSection`）。
                <MobileNavSection
                  icon=IconKind::Timer
                  title=Signal::derive(move || t("shell.exam-center"))
                  groups=EXAM_GROUPS
                  pathname=Signal::derive(move || location.pathname.get())
                  on_navigate=Callback::new(move |()| menu_open.set(false))
                />
                <div class="border-t-2 border-dashed border-ink/30 sm:hidden"></div>
                <MobileNavSection
                  icon=IconKind::BookOpen
                  title=Signal::derive(move || t("shell.knowledge"))
                  groups=KNOWLEDGE_GROUPS
                  pathname=Signal::derive(move || location.pathname.get())
                  on_navigate=Callback::new(move |()| menu_open.set(false))
                />
                <div class="border-t-2 border-dashed border-ink/30 sm:hidden"></div>
                <MobileNavSection
                  icon=IconKind::Calculator
                  title=Signal::derive(move || t("shell.tools"))
                  groups=TOOL_GROUPS
                  pathname=Signal::derive(move || location.pathname.get())
                  on_navigate=Callback::new(move |()| menu_open.set(false))
                />
              </nav>
            </div>
          }
        })
      }}
      <SettingsDialog open=settings_open />
    </nav>
  }
}
