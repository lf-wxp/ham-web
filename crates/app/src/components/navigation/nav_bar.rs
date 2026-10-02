use leptos::ev;
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};
use crate::ui::{Size, Variant, button_class};

use super::locale_toggle::LocaleToggle;
use super::theme_toggle::ThemeToggle;

use super::menu::{EXAM_ITEMS, KNOWLEDGE_GROUPS, MenuKind, TOOL_GROUPS};
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
  let exam_active = move || EXAM_ITEMS.iter().any(|i| active(i.href));
  let knowledge_active = move || {
    KNOWLEDGE_GROUPS
      .iter()
      .any(|g| g.items.iter().any(|i| active(i.href)))
  };
  let tool_active = move || {
    TOOL_GROUPS
      .iter()
      .any(|g| g.items.iter().any(|i| active(i.href)))
  };

  view! {
    <nav data-nav aria-label=move || t("主导航") class="sticky top-0 z-50 border-b bg-background/80 backdrop-blur-xl supports-[backdrop-filter]:bg-background/60">
      <div class="pointer-events-none absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-primary/50 to-transparent"></div>
      <div class="container mx-auto px-4">
        <div class="flex h-16 items-center justify-between gap-3">
          // 品牌标识
          <a
            href="/"
            class="group flex shrink-0 items-center gap-2.5"
            on:click=move |_| menu_open.set(false)
          >
            <div class="relative flex size-9 items-center justify-center overflow-hidden rounded-lg bg-primary text-primary-foreground shadow-xs shadow-primary/30 transition-transform duration-300 group-hover:rotate-6 group-hover:scale-110">
              <span class="absolute inset-0 bg-gradient-to-b from-white/20 to-black/15"></span>
              <Icon kind=IconKind::Satellite class="relative h-5 w-5" />
            </div>
            <div class="hidden flex-col whitespace-nowrap leading-tight sm:flex">
              <span class="text-sm font-semibold text-foreground">{move || t("业余无线电")}</span>
              <span class="text-[11px] text-muted-foreground">{move || t("题库 · 知识 · 工具")}</span>
            </div>
          </a>

          // 桌面端导航
          <div class="hidden items-center gap-1 xl:flex">
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
              {move || t("首页")}
            </a>

            // 考试中心下拉
            <div class="group relative">
              <button
                type="button"
                data-slot="button"
                aria-expanded=move || open_menu.get() == Some(MenuKind::Exam)
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
                {move || t("考试中心")}
                <Icon kind=IconKind::ChevronDown class="h-3.5 w-3.5" />
              </button>
              <div class=move || {
                cn(&[
                  "absolute left-0 top-full pt-1.5 transition",
                  if open_menu.get() == Some(MenuKind::Exam) {
                    "visible opacity-100"
                  } else {
                    "invisible opacity-0"
                  },
                ])
              }>
                <div class="w-44 rounded-lg border bg-popover p-1 shadow-md">
                  {EXAM_ITEMS
                    .iter()
                    .map(|item| {
                      view! {
                        <a
                          href=item.href
                          on:click=move |_| open_menu.set(None)
                          class=move || {
                            cn(&[
                              "flex items-center gap-2 rounded-md px-3 py-2 text-sm transition-colors",
                              if active(item.href) {
                                "bg-accent text-foreground"
                              } else {
                                "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
                              },
                            ])
                          }
                        >
                          <Icon kind=item.icon class="h-4 w-4" />
                          {move || t(item.label)}
                        </a>
                      }
                    })
                    .collect_view()}
                </div>
              </div>
            </div>

            // 知识库分组菜单
            <div class="group relative">
              <button
                type="button"
                data-slot="button"
                aria-expanded=move || open_menu.get() == Some(MenuKind::Knowledge)
                class=move || {
                  button_class(
                    if knowledge_active() { Variant::Default } else { Variant::Ghost },
                    Size::Sm,
                    "inline-flex items-center gap-2 whitespace-nowrap",
                  )
                }
                on:click=move |_| {
                  open_menu.update(|m| {
                    *m = if *m == Some(MenuKind::Knowledge) {
                      None
                    } else {
                      Some(MenuKind::Knowledge)
                    };
                  });
                }
              >
                <Icon kind=IconKind::BookOpen class="h-4 w-4" />
                {move || t("知识库")}
                <Icon kind=IconKind::ChevronDown class="h-3.5 w-3.5" />
              </button>
              <div class=move || {
                cn(&[
                  "absolute left-1/2 top-full -translate-x-1/2 pt-1.5 transition",
                  if open_menu.get() == Some(MenuKind::Knowledge) {
                    "visible opacity-100"
                  } else {
                    "invisible opacity-0"
                  },
                ])
              }>
                <div class="grid w-[900px] max-w-[calc(100vw-2rem)] grid-cols-2 gap-x-4 gap-y-3 rounded-lg border bg-popover p-4 shadow-md md:grid-cols-3 lg:grid-cols-5">
                  {KNOWLEDGE_GROUPS
                    .iter()
                    .map(|g| {
                      view! {
                        <div>
                          <div class="mb-1.5 flex items-center gap-1.5 px-2 text-xs font-semibold text-muted-foreground">
                            <Icon kind=g.icon class="h-3.5 w-3.5" />
                            <span>{move || t(g.label)}</span>
                          </div>
                          <div class="space-y-0.5">
                            {g
                              .items
                              .iter()
                              .map(|item| {
                                view! {
                                  <a
                                    href=item.href
                                    on:click=move |_| open_menu.set(None)
                                    class=move || {
                                      cn(&[
                                        "flex items-center gap-2 rounded-md px-2.5 py-1.5 text-sm transition-colors",
                                        if active(item.href) {
                                          "bg-accent text-foreground"
                                        } else {
                                          "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
                                        },
                                      ])
                                    }
                                  >
                                    <Icon kind=item.icon class="h-4 w-4 shrink-0" />
                                    <span class="min-w-0 leading-snug">{move || t(item.label)}</span>
                                  </a>
                                }
                              })
                              .collect_view()}
                          </div>
                        </div>
                      }
                    })
                    .collect_view()}
                </div>
              </div>
            </div>

            // 工具下拉
            <div class="group relative">
              <button
                type="button"
                data-slot="button"
                aria-expanded=move || open_menu.get() == Some(MenuKind::Tools)
                class=move || {
                  button_class(
                    if tool_active() { Variant::Default } else { Variant::Ghost },
                    Size::Sm,
                    "inline-flex items-center gap-2 whitespace-nowrap",
                  )
                }
                on:click=move |_| {
                  open_menu.update(|m| {
                    *m = if *m == Some(MenuKind::Tools) { None } else { Some(MenuKind::Tools) };
                  });
                }
              >
                <Icon kind=IconKind::Calculator class="h-4 w-4" />
                {move || t("工具")}
                <Icon kind=IconKind::ChevronDown class="h-3.5 w-3.5" />
              </button>
              <div class=move || {
                cn(&[
                  "absolute right-0 top-full pt-1.5 transition",
                  if open_menu.get() == Some(MenuKind::Tools) {
                    "visible opacity-100"
                  } else {
                    "invisible opacity-0"
                  },
                ])
              }>
                <div class="grid w-[720px] max-w-[calc(100vw-2rem)] grid-cols-2 gap-x-4 gap-y-3 rounded-lg border bg-popover p-4 shadow-md md:grid-cols-3 lg:grid-cols-5">
                  {TOOL_GROUPS
                    .iter()
                    .map(|g| {
                      view! {
                        <div>
                          <div class="mb-1.5 flex items-center gap-1.5 px-2 text-xs font-semibold text-muted-foreground">
                            <Icon kind=g.icon class="h-3.5 w-3.5" />
                            <span>{move || t(g.label)}</span>
                          </div>
                          <div class="space-y-0.5">
                            {g
                              .items
                              .iter()
                              .map(|item| {
                                view! {
                                  <a
                                    href=item.href
                                    on:click=move |_| open_menu.set(None)
                                    class=move || {
                                      cn(&[
                                        "flex items-center gap-2 rounded-md px-2.5 py-1.5 text-sm transition-colors",
                                        if active(item.href) {
                                          "bg-accent text-foreground"
                                        } else {
                                          "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
                                        },
                                      ])
                                    }
                                  >
                                    <Icon kind=item.icon class="h-4 w-4 shrink-0" />
                                    <span class="min-w-0 leading-snug">{move || t(item.label)}</span>
                                  </a>
                                }
                              })
                              .collect_view()}
                          </div>
                        </div>
                      }
                    })
                    .collect_view()}
                </div>
              </div>
            </div>
          </div>

          // 右侧：搜索 + 主题切换 + 移动端菜单按钮
          <div class="flex items-center gap-1">
            <button
              type="button"
              data-slot="button"
              class=button_class(Variant::Ghost, Size::Icon, "")
              aria-label=move || t("搜索")
              title=move || t("搜索（/）")
              on:click=move |_| search_open.set(true)
            >
              <Icon kind=IconKind::Search class="h-5 w-5" />
            </button>
            <LocaleToggle />
            <ThemeToggle />
            <button
              type="button"
              data-slot="button"
              class=button_class(Variant::Ghost, Size::Icon, "xl:hidden")
              aria-label=move || if menu_open.get() { t("关闭菜单") } else { t("打开菜单") }
              aria-expanded=move || menu_open.get()
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
              <div class="absolute inset-x-0 top-full z-40 max-h-[calc(100vh-4rem)] overflow-y-auto border-b bg-background shadow-md xl:hidden animate-in slide-in-from-top-2 fade-in duration-200">
                <nav aria-label=move || t("移动端导航") class="container mx-auto grid grid-cols-1 gap-1 px-4 py-3">
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
                    {move || t("首页")}
                  </a>

                  <div class="px-3 pt-2 text-xs font-semibold text-muted-foreground">{move || t("语言")}</div>
                  <div class="flex flex-wrap gap-1.5 px-3">
                    {Locale::ALL
                      .iter()
                      .map(|&l| view! {
                        <button
                          type="button"
                          class=move || {
                            cn(&[
                              "rounded-md border px-3 py-1.5 text-sm transition-colors",
                              if i18n::locale().get() == l {
                                "border-primary bg-primary text-primary-foreground"
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

                  <div class="px-3 pt-2 text-xs font-semibold text-muted-foreground">{move || t("考试中心")}</div>
                  {EXAM_ITEMS
                    .iter()
                    .map(|item| {
                      view! {
                        <a
                          href=item.href
                          class=move || {
                            cn(&[
                              "flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors",
                              if active(item.href) {
                                "bg-accent text-foreground"
                              } else {
                                "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
                              },
                            ])
                          }
                          on:click=move |_| menu_open.set(false)
                        >
                          <Icon kind=item.icon class="h-5 w-5" />
                          {move || t(item.label)}
                        </a>
                      }
                    })
                    .collect_view()}

                  <div class="px-3 pt-2 text-xs font-semibold text-muted-foreground">{move || t("知识库")}</div>
                  {KNOWLEDGE_GROUPS
                    .iter()
                    .map(|g| {
                      view! {
                        <div class="px-3 pt-1 text-xs font-medium text-muted-foreground">{move || t(g.label)}</div>
                        {g
                          .items
                          .iter()
                          .map(|item| {
                            view! {
                              <a
                                href=item.href
                                class=move || {
                                  cn(&[
                                    "flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors",
                                    if active(item.href) {
                                      "bg-accent text-foreground"
                                    } else {
                                      "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
                                    },
                                  ])
                                }
                                on:click=move |_| menu_open.set(false)
                              >
                                <Icon kind=item.icon class="h-5 w-5" />
                                {move || t(item.label)}
                              </a>
                            }
                          })
                          .collect_view()}
                      }
                    })
                    .collect_view()}

                  <div class="px-3 pt-2 text-xs font-semibold text-muted-foreground">{move || t("工具")}</div>
                  {TOOL_GROUPS
                    .iter()
                    .map(|g| {
                      view! {
                        <div class="px-3 pt-1 text-xs font-medium text-muted-foreground">{move || t(g.label)}</div>
                        {g
                          .items
                          .iter()
                          .map(|item| {
                            view! {
                              <a
                                href=item.href
                                class=move || {
                                  cn(&[
                                    "flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors",
                                    if active(item.href) {
                                      "bg-accent text-foreground"
                                    } else {
                                      "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
                                    },
                                  ])
                                }
                                on:click=move |_| menu_open.set(false)
                              >
                                <Icon kind=item.icon class="h-5 w-5" />
                                {move || t(item.label)}
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
