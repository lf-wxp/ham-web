//! 全局顶部导航：所有页面可见，含主导航、移动端菜单与主题切换。

use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};
use crate::theme::{Theme, use_theme};
use crate::ui::{Size, Variant, button_class};

struct NavItem {
  href: &'static str,
  label: &'static str,
  icon: IconKind,
}

const NAV_ITEMS: &[NavItem] = &[
  NavItem { href: "/", label: "首页", icon: IconKind::Home },
  NavItem { href: "/practice", label: "练习", icon: IconKind::ClipboardList },
  NavItem { href: "/exam", label: "考试", icon: IconKind::Timer },
  NavItem { href: "/browse", label: "分类浏览", icon: IconKind::LayoutGrid },
  NavItem { href: "/glossary", label: "术语表", icon: IconKind::BookOpen },
  NavItem { href: "/bands", label: "波段表", icon: IconKind::Satellite },
  NavItem { href: "/photo-processor", label: "照片处理", icon: IconKind::Camera },
];

#[component]
pub fn Navigation() -> impl IntoView {
  let location = use_location();
  let menu_open = RwSignal::new(false);

  let active = move |href: &'static str| location.pathname.get() == href;

  view! {
    <nav class="sticky top-0 z-50 border-b bg-background/80 backdrop-blur-xl supports-[backdrop-filter]:bg-background/60">
      <div class="container mx-auto px-4">
        <div class="flex h-16 items-center justify-between gap-3">
          // 品牌标识
          <a href="/" class="group flex items-center gap-2.5" on:click=move |_| menu_open.set(false)>
            <div class="flex size-9 items-center justify-center rounded-lg bg-primary text-primary-foreground shadow-xs transition-transform duration-300 group-hover:rotate-6 group-hover:scale-110">
              <Icon kind=IconKind::Satellite class="h-5 w-5" />
            </div>
            <div class="hidden flex-col leading-tight sm:flex">
              <span class="text-sm font-semibold text-foreground">"业余无线电考试"</span>
              <span class="text-[11px] text-muted-foreground">"题库 · 练习 · 模拟考试"</span>
            </div>
          </a>

          // 桌面端主导航
          <div class="hidden items-center gap-1 lg:flex">
            {NAV_ITEMS
              .iter()
              .map(|item| {
                view! {
                  <a
                    href=item.href
                    data-slot="button"
                    class=move || {
                      button_class(
                        if active(item.href) { Variant::Default } else { Variant::Ghost },
                        Size::Sm,
                        "inline-flex items-center gap-2",
                      )
                    }
                  >
                    <Icon kind=item.icon class="h-4 w-4" />
                    {item.label}
                  </a>
                }
              })
              .collect_view()}
          </div>

          // 右侧：主题切换 + 移动端菜单按钮
          <div class="flex items-center gap-1">
            <ThemeToggle />
            <button
              type="button"
              data-slot="button"
              class=button_class(Variant::Ghost, Size::Icon, "lg:hidden")
              aria-label=move || if menu_open.get() { "关闭菜单" } else { "打开菜单" }
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
              <div class="lg:hidden border-t animate-in slide-in-from-top-2 fade-in duration-200">
                <nav class="container mx-auto grid grid-cols-1 gap-1 px-4 py-3">
                  {NAV_ITEMS
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
                          {item.label}
                        </a>
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

/// 明暗主题切换按钮，可通过 `class` 追加定位等样式。
#[component]
pub fn ThemeToggle(#[prop(optional)] class: &'static str) -> impl IntoView {
  let theme = use_theme();
  let toggle_label = move || {
    if theme.is_dark() {
      "切换到浅色模式"
    } else {
      "切换到深色模式"
    }
  };
  view! {
    <button
      type="button"
      data-slot="button"
      class=button_class(Variant::Ghost, Size::Icon, class)
      aria-label=toggle_label
      title=toggle_label
      on:click=move |_| theme.set(if theme.is_dark() { Theme::Light } else { Theme::Dark })
    >
      {move || {
        if theme.is_dark() {
          view! { <Icon kind=IconKind::Sun class="h-4 w-4" /> }
        } else {
          view! { <Icon kind=IconKind::Moon class="h-4 w-4" /> }
        }
      }}
    </button>
  }
}
