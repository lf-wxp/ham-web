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

        // 移动端下拉菜单
        {move || {
          menu_open.get().then(|| {
            view! {
              <div
                id=MOBILE_NAV_PANEL_ID
                class="motion-pop absolute inset-x-0 top-full z-40 max-h-[calc(100svh-4rem)] overflow-y-auto overscroll-contain border-b-2 border-ink bg-card shadow-[0_4px_0_0_var(--pxl-shadow)] xl:hidden"
              >
                <nav aria-label=move || t("shell.mobile-navigation") class="container mx-auto grid grid-cols-1 gap-1 px-4 py-3">
                  <a
                    href="/"
                    class=move || {
                      cn(&[
                        "flex items-center gap-3 border-2 px-3 py-2.5 text-sm",
                        if active("/") {
                          "border-ink bg-accent text-accent-foreground"
                        } else {
                          "border-transparent text-muted-foreground hover:border-ink hover:bg-accent hover:text-accent-foreground"
                        },
                      ])
                    }
                    on:click=move |_| menu_open.set(false)
                  >
                    <Icon kind=IconKind::Home class="size-6" />
                    {move || t("shell.home")}
                  </a>

                  <div class="px-3 pt-2 pxl-label text-xs text-muted-foreground">{move || t("shell.language")}</div>
                  <div
                    class="flex flex-wrap gap-1.5 px-3"
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
                              "border-2 px-3 py-1.5 text-sm",
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

                  // 手机顶栏没有齿轮：「像素动效 / 易读字体 / 主题」都从这里进。
                  <div class="space-y-2 px-3 pt-2">
                    <Button
                      variant=Variant::Outline
                      size=Size::Default
                      class="w-full justify-start gap-3"
                      on_click=Callback::new(move |_| {
                        menu_open.set(false);
                        settings_open.set(true);
                      })
                    >
                      <Icon kind=IconKind::Settings class="size-6" />
                      {move || t("shell.settings")}
                    </Button>
                    // 备份不藏在工具页：手机上也给一个直达入口。
                    <ButtonLink
                      href="/backup"
                      variant=Variant::Outline
                      class="w-full justify-start gap-3"
                      on_click=Callback::new(move |_| menu_open.set(false))
                    >
                      <Icon kind=IconKind::Download class="size-6" />
                      {move || t("tools.data-backup")}
                    </ButtonLink>
                  </div>

                  // 三大段可折叠、当前页所在的那段默认展开（见 `MobileNavSection`）。
                  <MobileNavSection
                    title=Signal::derive(move || t("shell.exam-center"))
                    groups=EXAM_GROUPS
                    pathname=Signal::derive(move || location.pathname.get())
                    on_navigate=Callback::new(move |()| menu_open.set(false))
                  />
                  <MobileNavSection
                    title=Signal::derive(move || t("shell.knowledge"))
                    groups=KNOWLEDGE_GROUPS
                    pathname=Signal::derive(move || location.pathname.get())
                    on_navigate=Callback::new(move |()| menu_open.set(false))
                  />
                  <MobileNavSection
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
      </div>
      <SettingsDialog open=settings_open />
    </nav>
  }
}
