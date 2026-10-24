use ham_web_core::registry;
use leptos::prelude::*;

use crate::cn::cn;
use crate::i18n::t;
use crate::icons::{Icon, IconKind, icon_of};

/// 移动端菜单里的一大段（考试中心 / 知识库 / 工具）：可折叠，当前页所在的那段默认展开。
///
/// 为什么折叠：以前三大段全部平铺，上百个入口在手机上是一条几千像素的长滚动，找功能靠滑；
/// 桌面端却是「分组侧栏 + 条目区」，两端心智模型不一致。折叠后首屏只剩三个段标题 + 常用入口，
/// 当前所在的那段自动展开，一眼知道自己在哪。
///
/// 用原生 `<details>` / `<summary>`：键盘（Enter / Space）、读屏（展开 / 收起状态）都是浏览器给的，
/// 不用再写一套 `aria-expanded`。菜单每次打开都会重建，所以 `open` 只需在构建时算一次。
#[component]
pub(super) fn MobileNavSection(
  #[prop(into)] title: Signal<String>,
  /// 这一大段包含的子分组（`EXAM_GROUPS` 等）。
  groups: &'static [&'static str],
  #[prop(into)] pathname: Signal<String>,
  /// 点了某个入口之后：由调用方收起整个菜单。
  on_navigate: Callback<()>,
) -> impl IntoView {
  let current = pathname.get_untracked();
  let has_current = registry::MODULES
    .iter()
    .any(|m| m.group.is_some_and(|g| groups.contains(&g)) && m.path == current);

  view! {
    <details open=has_current class="group border-t-2 border-dashed border-ink/30 pt-1">
      <summary class="flex min-h-11 cursor-pointer list-none items-center justify-between px-3 pxl-label text-xs text-muted-foreground [&::-webkit-details-marker]:hidden">
        <span>{move || title.get()}</span>
        <Icon kind=IconKind::ChevronDown class="size-6 group-open:rotate-180" />
      </summary>
      <div class="grid gap-1 pb-2">
        {groups
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
                          "flex min-h-11 items-center gap-3 border-2 px-3 py-2.5 text-sm",
                          if pathname.get() == m.path {
                            "border-ink bg-accent text-accent-foreground"
                          } else {
                            "border-transparent text-muted-foreground hover:border-ink hover:bg-accent hover:text-accent-foreground"
                          },
                        ])
                      }
                      aria-current=move || (pathname.get() == m.path).then_some("page")
                      on:click=move |_| on_navigate.run(())
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
      </div>
    </details>
  }
}
