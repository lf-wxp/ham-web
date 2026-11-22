//! 移动端菜单里的一段：分组小标题 + 该组的入口链接。
//!
//! 段内不再折叠（以前每段是原生 `<details>`）：三大段由标签栏切换（见 `mobile_nav_panel.rs`），
//! 段内直接铺开 —— 再多一层折叠，等于在手机上点两次才能看见一个入口。
//!
//! 分组按「小标题 + 网格」成块渲染，而不是把 116 个条目平铺成一张大网格：知识库有 7 个分组，
//! 平铺后条目之间没有分界，用户只能靠记住顺序找。每个分组自己是一个 `<section>`，
//! 组标题（`h3`）天然跨满整行，网格只管条目。

use ham_web_core::registry;
use leptos::prelude::*;

use crate::cn::cn;
use crate::i18n::t;
use crate::icons::{Icon, icon_of};

use super::nav_sections::NavSection;

#[component]
pub(super) fn MobileNavSection(
  section: &'static NavSection,
  /// 当前路由：用于给所在条目打 `aria-current`（也是 e2e 的定位契约）。
  #[prop(into)]
  pathname: Signal<String>,
  /// 点了某个入口之后：由调用方收起整个菜单。
  on_navigate: Callback<()>,
) -> impl IntoView {
  view! {
    <div class="space-y-4">
      {section
        .groups()
        .iter()
        .map(|&group| {
          view! {
            <section>
              <h3 class="pxl-sub-head mb-1.5">{move || t(group)}</h3>
              // 手机一列；平板两列；1024 起三列 —— 面板在 `xl` 消失，所以不需要更宽的档。
              // `gap-1` 保持条目之间只隔 4px：它们是一张列表里的行，不是一组按钮。
              <div class="grid grid-cols-1 gap-1 sm:grid-cols-2 lg:grid-cols-3">
                {registry::in_group(group)
                  .into_iter()
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
                        <Icon kind=icon_of(m.icon) class="size-6 shrink-0" />
                        // `break-words`：西语长标题在平板上两列时会被截断成省略号，
                        // 换行最多多占一行（条目区自身有滚动兜底）。
                        <span class="min-w-0 break-words">{move || t(m.title)}</span>
                      </a>
                    }
                  })
                  .collect_view()}
              </div>
            </section>
          }
        })
        .collect_view()}
    </div>
  }
}
