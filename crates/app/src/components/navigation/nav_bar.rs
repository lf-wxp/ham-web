//! 顶部导航栏。
//!
//! 本文件里的**导航项与下拉 / 菜单开关刻意保留原生 `<a>` / `<button>` + [`button_class`]**：
//! 高亮态要随当前路由切换 `variant`，下拉开关还要输出 `aria-expanded` —— 这两个通道
//! [`Button`] 与 `ButtonLink` 目前都没有（见 `docs/ui-components.md` 的「常见坑」）。
//! 同一个文件里的搜索、语言切换、主题切换等按钮都已经用组件，**不要顺手把这些清掉**。
//! 「知识库」「工具」两个分组下拉的开关与面板是同一套路数，已拆到 `group_menu.rs`。

use leptos::ev;
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;

use ham_web_core::registry::{self, GROUP_EXAM, KNOWLEDGE_GROUPS, TOOL_GROUPS};

use crate::cn::cn;
use crate::icons::{Icon, IconKind, icon_of};
use crate::ui::{Button, Size, Variant, button_class};

use super::group_menu::{GroupMenu, MenuAlign};
use super::hud_badge::HudBadge;
use super::locale_toggle::LocaleToggle;
use super::settings_dialog::SettingsDialog;
use super::theme_toggle::ThemeToggle;

use super::menu::MenuKind;
use super::mobile_menu::{MOBILE_NAV_PANEL_ID, use_mobile_menu};
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
  // 某分组下是否有当前页面（用于给对应的一级菜单加高亮）。
  let group_active = move |group: &str| {
    registry::MODULES
      .iter()
      .filter(|m| m.group == Some(group))
      .any(|m| active(m.path))
  };
  let exam_active = move || group_active(GROUP_EXAM);

  view! {
    // 顶栏是一条实心的「HUD 栏」：底色用 `--card`（不透明）+ 2px 墨色下沿 + 硬偏移投影，
    // 不模糊、不渐变；吸顶时下面滚过的内容被它实实在在地盖住，不需要任何磨砂兜底。
    <nav data-nav aria-label=move || t("shell.main-navigation") class="sticky top-0 z-50 border-b-2 border-ink bg-card shadow-[0_2px_0_0_var(--pxl-shadow)]">
      <div class="container relative mx-auto px-1 min-[360px]:px-2 sm:px-4">
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

            // 考试中心下拉
            // `self-stretch`：把容器拉满导航行的高度（h-16），`top-full` 才是「导航栏下沿」。
            // 不加的话 `top-full` 会取按钮自身的高度，这个面板就比另两个高出十几个像素。
            <div class="group relative flex items-center self-stretch">
              // 刻意的例外，见文件头：下拉开关要切换 `variant` 并输出 `aria-expanded`。
              <button
                type="button"
                data-slot="button"
                aria-expanded=move || (open_menu.get() == Some(MenuKind::Exam)).to_string()
                class=move || {
                  button_class(
                    if exam_active() { Variant::Default } else { Variant::Ghost },
                    Size::Sm,
                    "inline-flex items-center gap-2 whitespace-nowrap",
                  )
                }
                on:click=move |_| {
                  open_menu.update(|m| {
                    *m = if *m == Some(MenuKind::Exam) { None } else { Some(MenuKind::Exam) };
                  });
                }
              >
                <Icon kind=IconKind::Timer class="size-6" />
                {move || t("shell.exam-center")}
                <Icon
                  kind=IconKind::ChevronDown
                  class=Signal::derive(move || {
                    cn(&[
                      "size-6",
                      if open_menu.get() == Some(MenuKind::Exam) { "rotate-180" } else { "" },
                    ])
                  })
                />
              </button>
              // 外层只管定位与显隐（隐藏后不再参与命中测试）；
              // `motion-popover` 挂在内层面板上做位移 + 缩放 —— 外层可能带
              // `-translate-x-1/2` 这类居中 transform，动效若也写 transform 会把它覆盖掉。
              <div
                class=move || {
                  cn(&[
                    "absolute left-0 top-full pt-2",
                    if open_menu.get() == Some(MenuKind::Exam) {
                      "visible opacity-100"
                    } else {
                      "invisible opacity-0"
                    },
                  ])
                }
              >
                <div
                  data-open=move || (open_menu.get() == Some(MenuKind::Exam)).to_string()
                  class="motion-popover pxl-popover origin-top w-52 p-1"
                >
                  {registry::MODULES
                    .iter()
                    .filter(|m| m.group == Some(GROUP_EXAM))
                    .map(|m| {
                      view! {
                        <a
                          href=m.nav_href()
                          on:click=move |_| open_menu.set(None)
                          class=move || {
                            cn(&[
                              "flex items-center gap-2 border-2 px-3 py-2 text-sm",
                              if active(m.path) {
                                "border-ink bg-accent text-accent-foreground"
                              } else {
                                "border-transparent text-muted-foreground hover:border-ink hover:bg-accent hover:text-accent-foreground"
                              },
                            ])
                          }
                        >
                          <Icon kind=icon_of(m.icon) class="size-6" />
                          {move || t(m.title)}
                        </a>
                      }
                    })
                    .collect_view()}
                </div>
              </div>
            </div>

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
              title=Signal::derive(move || t("shell.search-2"))
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
            // 刻意的例外，见文件头：菜单开关要输出 `aria-expanded`（`Button` 暂时没有这个通道）。
            <button
              type="button"
              data-slot="button"
              class=button_class(Variant::Ghost, Size::Icon, "xl:hidden")
              aria-label=move || if menu_open.get() { t("shell.close-menu") } else { t("shell.open-menu") }
              aria-expanded=move || menu_open.get().to_string()
              aria-controls=MOBILE_NAV_PANEL_ID
              on:click=move |_| menu_open.update(|v| *v = !*v)
            >
              {move || {
                if menu_open.get() {
                  view! { <Icon kind=IconKind::X class="size-6" /> }
                } else {
                  view! { <Icon kind=IconKind::Menu class="size-6" /> }
                }
              }}
            </button>
          </div>
        </div>

        // 移动端下拉菜单
        {move || {
          menu_open.get().then(|| {
            view! {
              <div
                id=MOBILE_NAV_PANEL_ID
                class="motion-pop absolute inset-x-0 top-full z-40 max-h-[calc(100vh-4rem)] overflow-y-auto border-b-2 border-ink bg-card shadow-[0_4px_0_0_var(--pxl-shadow)] xl:hidden"
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
                        <button
                          type="button"
                          // 选中态不能只靠颜色：读屏用户靠 `aria-pressed` 分辨当前语言。
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
                  <div class="px-3 pt-2">
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
                  </div>

                  <div class="px-3 pt-2 pxl-label text-xs text-muted-foreground">{move || t("shell.exam-center")}</div>
                  {registry::MODULES
                    .iter()
                    .filter(|m| m.group == Some(GROUP_EXAM))
                    .map(|m| {
                      view! {
                        <a
                          href=m.nav_href()
                          class=move || {
                            cn(&[
                              "flex items-center gap-3 border-2 px-3 py-2.5 text-sm",
                              if active(m.path) {
                                "border-ink bg-accent text-accent-foreground"
                              } else {
                                "border-transparent text-muted-foreground hover:border-ink hover:bg-accent hover:text-accent-foreground"
                              },
                            ])
                          }
                          on:click=move |_| menu_open.set(false)
                        >
                          <Icon kind=icon_of(m.icon) class="size-6" />
                          {move || t(m.title)}
                        </a>
                      }
                    })
                    .collect_view()}

                  <div class="px-3 pt-2 pxl-label text-xs text-muted-foreground">{move || t("shell.knowledge")}</div>
                  {KNOWLEDGE_GROUPS
                    .iter()
                    .map(|g| {
                      view! {
                        <div class="px-3 pt-1 text-xs text-muted-foreground">{move || t(g)}</div>
                        {registry::MODULES
                          .iter()
                          .filter(|m| m.group == Some(*g))
                          .map(|m| {
                            view! {
                              <a
                                href=m.nav_href()
                                class=move || {
                                  cn(&[
                                    "flex items-center gap-3 border-2 px-3 py-2.5 text-sm",
                                    if active(m.path) {
                                      "border-ink bg-accent text-accent-foreground"
                                    } else {
                                      "border-transparent text-muted-foreground hover:border-ink hover:bg-accent hover:text-accent-foreground"
                                    },
                                  ])
                                }
                                on:click=move |_| menu_open.set(false)
                              >
                                <Icon kind=icon_of(m.icon) class="size-6" />
                                {move || t(m.title)}
                              </a>
                            }
                          })
                          .collect_view()}
                      }
                    })
                    .collect_view()}

                  <div class="px-3 pt-2 pxl-label text-xs text-muted-foreground">{move || t("shell.tools")}</div>
                  {TOOL_GROUPS
                    .iter()
                    .map(|g| {
                      view! {
                        <div class="px-3 pt-1 text-xs text-muted-foreground">{move || t(g)}</div>
                        {registry::MODULES
                          .iter()
                          .filter(|m| m.group == Some(*g))
                          .map(|m| {
                            view! {
                              <a
                                href=m.nav_href()
                                class=move || {
                                  cn(&[
                                    "flex items-center gap-3 border-2 px-3 py-2.5 text-sm",
                                    if active(m.path) {
                                      "border-ink bg-accent text-accent-foreground"
                                    } else {
                                      "border-transparent text-muted-foreground hover:border-ink hover:bg-accent hover:text-accent-foreground"
                                    },
                                  ])
                                }
                                on:click=move |_| menu_open.set(false)
                              >
                                <Icon kind=icon_of(m.icon) class="size-6" />
                                {move || t(m.title)}
                              </a>
                            }
                          })
                          .collect_view()}
                      }
                    })
                    .collect_view()}
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
