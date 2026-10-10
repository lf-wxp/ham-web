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
use super::locale_toggle::LocaleToggle;
use super::theme_toggle::ThemeToggle;

use super::menu::MenuKind;
use crate::i18n::{self, Locale, t};

#[component]
pub fn Navigation() -> impl IntoView {
  let location = use_location();
  let menu_open = RwSignal::new(false);
  let open_menu = RwSignal::new(None::<MenuKind>);

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
    // 顶栏材质：磨砂 + 提饱和（`backdrop-saturate-150`，让背后的极光透出来而不是发灰）；
    // 模糊半径停在 `xl`：顶栏常驻且每帧滚动都要重算，半径翻倍代价也近乎翻倍；
    // `nav-elevate` 随滚动淡入一道下沿阴影；`nav-progress` 是整页滚动进度线。
    // 两者都是 CSS 滚动驱动动画，没有 JS，不支持的浏览器上整块隐形（见 style/input.css）。
    <nav data-nav aria-label=move || t("shell.main-navigation") class="nav-elevate sticky top-0 z-50 border-b border-border/70 bg-background/70 backdrop-blur-xl backdrop-saturate-150 supports-[backdrop-filter]:bg-background/55">
      <div class="pointer-events-none absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-primary/60 to-transparent"></div>
      <div
        aria-hidden="true"
        class="nav-progress pointer-events-none absolute inset-x-0 bottom-0 h-0.5 bg-gradient-to-r from-[var(--hero-a)] via-[var(--hero-b)] to-[var(--hero-c)]"
      ></div>
      <div class="container relative mx-auto px-4">
        <div class="flex h-16 items-center justify-between gap-3">
          // 品牌标识
          <a
            href="/"
            class="group flex shrink-0 items-center gap-2.5"
            on:click=move |_| menu_open.set(false)
          >
            <div class="brand-mark relative flex size-9 items-center justify-center overflow-hidden rounded-xl text-primary-foreground transition-transform duration-300 ease-[var(--ease-out-back)] group-hover:rotate-6 group-hover:scale-110">
              <span class="absolute inset-0 bg-gradient-to-b from-white/20 to-black/15"></span>
              <Icon kind=IconKind::Satellite class="relative h-5 w-5" />
            </div>
            <div class="hidden flex-col whitespace-nowrap leading-tight sm:flex">
              <span class="font-display text-[13px] font-semibold tracking-tight text-foreground">{move || t("shell.amateur-radio")}</span>
              <span class="text-[11px] text-muted-foreground">{move || t("shell.exams-knowledge-tools")}</span>
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
              <Icon kind=IconKind::Home class="h-4 w-4" />
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
                <Icon kind=IconKind::Timer class="h-4 w-4" />
                {move || t("shell.exam-center")}
                <Icon
                  kind=IconKind::ChevronDown
                  class=Signal::derive(move || {
                    cn(&[
                      "h-3.5 w-3.5 transition-transform duration-200",
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
                    "absolute left-0 top-full pt-1.5 transition duration-[var(--motion-normal)] ease-[var(--ease-out-expo)]",
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
                  class="motion-popover origin-top w-48 rounded-2xl border bg-popover p-1.5 shadow-xl"
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
                              "flex items-center gap-2 rounded-xl px-3 py-2 text-sm transition-colors",
                              if active(m.path) {
                                "bg-accent text-foreground"
                              } else {
                                "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
                              },
                            ])
                          }
                        >
                          <Icon kind=icon_of(m.icon) class="h-4 w-4" />
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
              <Icon kind=IconKind::Search class="h-5 w-5" />
            </Button>
            <LocaleToggle />
            <ThemeToggle />
            // 刻意的例外，见文件头：菜单开关要输出 `aria-expanded`（`Button` 暂时没有这个通道）。
            <button
              type="button"
              data-slot="button"
              class=button_class(Variant::Ghost, Size::Icon, "xl:hidden")
              aria-label=move || if menu_open.get() { t("shell.close-menu") } else { t("shell.open-menu") }
              aria-expanded=move || menu_open.get().to_string()
              on:click=move |_| menu_open.update(|v| *v = !*v)
            >
              {move || {
                if menu_open.get() {
                  view! { <Icon kind=IconKind::X class="h-5 w-5" /> }
                } else {
                  view! { <Icon kind=IconKind::Menu class="h-5 w-5" /> }
                }
              }}
            </button>
          </div>
        </div>

        // 移动端下拉菜单
        {move || {
          menu_open.get().then(|| {
            view! {
              <div class="absolute inset-x-0 top-full z-40 max-h-[calc(100vh-4rem)] overflow-y-auto border-b bg-background/95 shadow-xl xl:hidden animate-in slide-in-from-top-2 fade-in duration-200">
                <nav aria-label=move || t("shell.mobile-navigation") class="container mx-auto grid grid-cols-1 gap-1 px-4 py-3">
                  <a
                    href="/"
                    class=move || {
                      cn(&[
                        "flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors",
                        if active("/") {
                          "bg-accent text-foreground"
                        } else {
                          "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
                        },
                      ])
                    }
                    on:click=move |_| menu_open.set(false)
                  >
                    <Icon kind=IconKind::Home class="h-5 w-5" />
                    {move || t("shell.home")}
                  </a>

                  <div class="px-3 pt-2 text-xs font-semibold text-muted-foreground">{move || t("shell.language")}</div>
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
                              "rounded-full border px-3.5 py-1.5 text-sm transition-colors",
                              if i18n::locale().get() == l {
                                "border-primary bg-primary text-primary-foreground shadow-sm shadow-primary/30"
                              } else {
                                "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
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

                  <div class="px-3 pt-2 text-xs font-semibold text-muted-foreground">{move || t("shell.exam-center")}</div>
                  {registry::MODULES
                    .iter()
                    .filter(|m| m.group == Some(GROUP_EXAM))
                    .map(|m| {
                      view! {
                        <a
                          href=m.nav_href()
                          class=move || {
                            cn(&[
                              "flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors",
                              if active(m.path) {
                                "bg-accent text-foreground"
                              } else {
                                "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
                              },
                            ])
                          }
                          on:click=move |_| menu_open.set(false)
                        >
                          <Icon kind=icon_of(m.icon) class="h-5 w-5" />
                          {move || t(m.title)}
                        </a>
                      }
                    })
                    .collect_view()}

                  <div class="px-3 pt-2 text-xs font-semibold text-muted-foreground">{move || t("shell.knowledge")}</div>
                  {KNOWLEDGE_GROUPS
                    .iter()
                    .map(|g| {
                      view! {
                        <div class="px-3 pt-1 text-xs font-medium text-muted-foreground">{move || t(g)}</div>
                        {registry::MODULES
                          .iter()
                          .filter(|m| m.group == Some(*g))
                          .map(|m| {
                            view! {
                              <a
                                href=m.nav_href()
                                class=move || {
                                  cn(&[
                                    "flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors",
                                    if active(m.path) {
                                      "bg-accent text-foreground"
                                    } else {
                                      "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
                                    },
                                  ])
                                }
                                on:click=move |_| menu_open.set(false)
                              >
                                <Icon kind=icon_of(m.icon) class="h-5 w-5" />
                                {move || t(m.title)}
                              </a>
                            }
                          })
                          .collect_view()}
                      }
                    })
                    .collect_view()}

                  <div class="px-3 pt-2 text-xs font-semibold text-muted-foreground">{move || t("shell.tools")}</div>
                  {TOOL_GROUPS
                    .iter()
                    .map(|g| {
                      view! {
                        <div class="px-3 pt-1 text-xs font-medium text-muted-foreground">{move || t(g)}</div>
                        {registry::MODULES
                          .iter()
                          .filter(|m| m.group == Some(*g))
                          .map(|m| {
                            view! {
                              <a
                                href=m.nav_href()
                                class=move || {
                                  cn(&[
                                    "flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors",
                                    if active(m.path) {
                                      "bg-accent text-foreground"
                                    } else {
                                      "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
                                    },
                                  ])
                                }
                                on:click=move |_| menu_open.set(false)
                              >
                                <Icon kind=icon_of(m.icon) class="h-5 w-5" />
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
    </nav>
  }
}
