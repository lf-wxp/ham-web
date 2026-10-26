//! 顶部导航栏。
//!
//! 本文件里的**导航项与分组下拉开关**保留原生元素 + [`button_class`]：高亮态要随当前
//! 路由切换 `variant`，而 [`Button`] / [`ButtonLink`] 的 `variant` 是静态 prop
//! （见 `docs/ui-components.md` 的「常见坑」）。`aria-expanded` / `aria-controls`
//! 已经有通道，不再是保留理由 —— 汉堡菜单开关因此收编回 [`Button`]。同一个文件里的
//! 搜索、语言切换、主题切换等按钮都已经用组件，**不要顺手把剩下的导航项清掉**。
//! 「考试中心」「知识库」「工具」三个分组下拉的开关与面板是同一套路数，已拆到 `group_menu.rs`。

use leptos::ev;
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;

use ham_web_core::registry::{EXAM_GROUPS, KNOWLEDGE_GROUPS, TOOL_GROUPS};

use crate::cn::cn;
use crate::icons::{Icon, IconKind};
use crate::ui::{Button, ButtonLink, Size, Variant, button_class};

use super::group_menu::{GroupMenu, MenuAlign};
use super::hud_badge::HudBadge;
use super::locale_toggle::LocaleToggle;
use super::settings_dialog::SettingsDialog;
use super::theme_toggle::ThemeToggle;

use super::menu::MenuKind;
use super::mobile_menu::{MOBILE_NAV_PANEL_ID, use_mobile_menu};
use super::mobile_nav_section::MobileNavSection;
use crate::i18n::{self, Locale, t};

#[component]
pub fn Navigation() -> impl IntoView {
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
  let search_open = expect_context::<RwSignal<bool>>();
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

  let active = move |href: &'static str| location.pathname.get() == href;
  view! {
    // 顶栏是一条实心的「HUD 栏」：底色用 `--card`（不透明）+ 2px 墨色下沿 + 硬偏移投影，
    // 不模糊、不渐变；吸顶时下面滚过的内容被它实实在在地盖住，不需要任何磨砂兜底。
    <nav data-nav aria-label=move || t("shell.main-navigation") class="sticky top-0 z-50 border-b-2 border-ink bg-card shadow-[0_2px_0_0_var(--pxl-shadow)]">
      <div class="container relative mx-auto px-1 xs:px-2 sm:px-4">
        <div class="flex h-16 items-center justify-between gap-2 sm:gap-3">
          // 品牌标识
          <a
            href="/"
            class="group flex shrink-0 items-center gap-2.5"
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

          // 桌面端导航
          // `self-stretch`：撑满 h-16 的导航行，让内部下拉的 `top-full` 能落在导航下沿。
          <div class="hidden items-center gap-1 self-stretch xl:flex">
            // 刻意的例外，见文件头：高亮要切 `variant`，`ButtonLink` 的 `variant` 是静态 prop。
            <a
              href="/"
              data-slot="button"
              class=move || {
                button_class(
                  if active("/") { Variant::Default } else { Variant::Ghost },
                  Size::Sm,
                  "inline-flex items-center gap-2 whitespace-nowrap",
                )
              }
            >
              <Icon kind=IconKind::Home class="size-6" />
              {move || t("shell.home")}
            </a>

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

          // 右侧：搜索 + 主题切换 + 移动端菜单按钮
          <div class="flex items-center gap-1">
            <Button
              variant=Variant::Ghost
              size=Size::Icon
              aria_label=Signal::derive(move || t("shell.search"))
              title=Signal::derive(move || t("shell.search-with-shortcut"))
              on_click=Callback::new(move |_| search_open.set(true))
            >
              <Icon kind=IconKind::Search class="size-6" />
            </Button>
            <LocaleToggle />
            // <sm（手机）放不下：这两个挪进移动菜单（「外观与动效」对话框里也有主题）。
            <div class="hidden items-center gap-1 sm:flex">
              <ThemeToggle />
              <Button
                variant=Variant::Ghost
                size=Size::Icon
                aria_label=Signal::derive(move || t("shell.settings"))
                title=Signal::derive(move || t("shell.settings"))
                on_click=Callback::new(move |_| settings_open.set(true))
              >
                <Icon kind=IconKind::Settings class="size-6" />
              </Button>
            </div>
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

      // 移动端下拉菜单。挂在 `<nav>` 上、不放进那层 `container` 里：`container` 在 400–639px
      // 之间是**定宽居中**的（`max-w` 400），面板跟着它走就会在左右各空出一条，露出下面的页面。
      // 内容宽度仍由面板里的 `container` 对齐：两侧边界与顶栏 `container` 同宽（同一 max-w），
      // 内边距是面板自己的 `px-4`（菜单需要缩进，不跟顶栏的 `px-1/2/4` 逐档一致）。
      {move || {
        menu_open.get().then(|| {
          view! {
            <div
              id=MOBILE_NAV_PANEL_ID
              class="motion-pop absolute inset-x-0 top-full z-40 max-h-[calc(100svh-4rem)] overflow-y-auto overscroll-contain border-b-2 border-ink bg-card shadow-[0_4px_0_0_var(--pxl-shadow)] xl:hidden"
            >
              // 布局与顺序：**先导航、后设置**，两种宽度共用同一份结构。
              // - `<sm`（手机竖屏）一列：三段折叠在上，虚线之下是语言 / 外观与动效 / 备份；
              // - `sm`–`xl`（横屏手机 / 平板 / 窄桌面）三列：三段各占一列，虚线之下仍是那排工具。
              // 以前把「首页 / 语言 / 设置 / 备份」放在最上面，点开菜单第一眼看到的是三个跟导航
              // 无关的控件，像误入设置页；首页也不必放 —— 底部 Tab 与顶栏 brand 都直达（见
              // `mobile_tabs.rs`）。单列铺到 640–1279px 还会拉成通栏长列表（900px 下约 370px 高）。
              //
              // 分隔虚线由这里画（一列时是三条，三列时只有一条横跨整行），不放在
              // `MobileNavSection` 里 —— 三列并排时每段自带的 `border-t` 会断成三段短虚线。
              <nav
                aria-label=move || t("shell.mobile-navigation")
                class="container mx-auto grid grid-cols-1 gap-1 px-4 py-3 sm:grid-cols-3 sm:items-start sm:gap-x-4"
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

                // 工具区与导航之间的那条线：一列时也画（三列时横跨整行）。
                <div class="border-t-2 border-dashed border-ink/30 sm:col-span-3"></div>

                // 「语言」与两个按钮排在同一行、竖直居中对齐：标签单独占一行时，左边两行、
                // 右边一行的落差看起来就像没对齐。可见的标签只是给眼睛的，读屏名字由 `role=group`
                // 的 `aria-label` 提供，所以 `aria-hidden` 掉，免得重复念一遍。
                // `hidden md:inline`：`sm`（640）那档每列只有 192px，图标 + 三个语言按钮塞不进一行，
                // 标签会把这一格顶成三行；那里只留按钮本身（中文 / English / Español 自解释）。
                <div class="flex flex-wrap items-center gap-2 px-3 pt-1 sm:self-center">
                  <span class="hidden pxl-label text-xs text-muted-foreground md:inline" aria-hidden="true">
                    {move || t("shell.language")}
                  </span>
                  <div
                    class="flex flex-wrap gap-1.5"
                    role="group"
                    aria-label=move || t("shell.language")
                  >
                    {Locale::ALL
                      .iter()
                      .map(|&l| view! {
                        // 刻意的例外，见文件头：选中态要重算 `class`（`Button.class` 是静态串），
                        // 顺带用 `aria-pressed` 让读屏用户也能分辨当前语言（不只是靠颜色）。
                        <button
                          type="button"
                          aria-pressed=move || (i18n::locale().get() == l).to_string()
                          class=move || {
                            cn(&[
                              // `px-2`：三个语言按钮要和「语言」同处一行，`px-3` 时
                              // 在这排三列（每列 234px）里放不下、会折到第二行。
                              "border-2 px-2 py-1.5 text-sm",
                              if i18n::locale().get() == l {
                                "border-ink bg-primary text-primary-foreground"
                              } else {
                                "border-input text-muted-foreground hover:border-ink hover:bg-accent hover:text-accent-foreground"
                              },
                            ])
                          }
                          on:click=move |_| i18n::set_locale(l)
                        >
                          {l.label()}
                        </button>
                      })
                      .collect_view()}
                  </div>
                </div>

                // 手机顶栏没有齿轮：「像素动效 / 易读字体 / 主题」都从这里进。
                // 一列时两个入口并排（省下一整行高度）；`sm:contents` 让外层消失、两个 `px-3`
                // 包装各占一格 —— 那圈内边距让按钮左缘与段标题的图标（`px-3`）齐平，
                // 否则按钮会顶到列边，比上面那排段标题偏出 12px。
                <div class="grid grid-cols-2 gap-1 sm:contents">
                  // `sm:self-center`：640px 那档语言按钮会折成两行、把这一格撑高，
                  // 不加就让按钮贴在这一格的顶边（整排看起来又是一高一低）。
                  <div class="px-3 sm:self-center">
                    <Button
                      variant=Variant::Outline
                      size=Size::Default
                      // `h-auto` + `whitespace-normal`：西语的「Aparición y movimiento」在窄格里
                      // 放不下一行（`.pxl-btn` 默认 nowrap + 固定 h-10），允许折行而不是溢出边框。
                      class="h-auto min-h-10 w-full justify-start whitespace-normal py-2 text-left"
                      on_click=Callback::new(move |_| {
                        menu_open.set(false);
                        settings_open.set(true);
                      })
                    >
                      <Icon kind=IconKind::Settings class="size-6" />
                      {move || t("shell.settings")}
                    </Button>
                  </div>
                  // 备份不藏在工具页：手机上也给一个直达入口。
                  <div class="px-3 sm:self-center">
                    <ButtonLink
                      href="/backup"
                      variant=Variant::Outline
                      class="h-auto min-h-10 w-full justify-start whitespace-normal py-2 text-left"
                      on_click=Callback::new(move |_| menu_open.set(false))
                    >
                      <Icon kind=IconKind::Download class="size-6" />
                      {move || t("tools.data-backup")}
                    </ButtonLink>
                  </div>
                </div>
              </nav>
            </div>
          }
        })
      }}
      <SettingsDialog open=settings_open />
    </nav>
  }
}
