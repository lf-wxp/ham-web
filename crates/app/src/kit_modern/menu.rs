//! 全站目录（「更多」面板）：把 `registry` 里的**全部页面**按分组铺出来，并带即时过滤。
//!
//! 之前的面板只有 9 个手写入口，170+ 页面根本摸不到；现在直接渲染 registry：
//! 新增页面只要登记进 registry，就自动出现在这里（和调谐台同一份数据源）。

use ham_web_core::registry::{self, EXAM_GROUPS, KNOWLEDGE_GROUPS, Module, TOOL_GROUPS};
use leptos::portal::Portal;
use leptos::prelude::*;

use crate::components::search_dialog::SearchOpen;
use crate::data::core_text_tracked;
use crate::i18n::{self, Locale, t};

use super::backdrop::Backdrop;
use super::module_icon::module_icon;
use super::{MIcon, ModernIcon};
use crate::focus::use_escape_close;
use crate::presence::use_presence;

/// 三大板块：考试中心 / 知识库 / 工具。
const SECTIONS: &[(&str, &[&str])] = &[
  ("shell.exam-center", EXAM_GROUPS),
  ("shell.knowledge", KNOWLEDGE_GROUPS),
  ("shell.tools", TOOL_GROUPS),
];

#[component]
pub fn ModernMenu(open: RwSignal<bool>, settings_open: RwSignal<bool>) -> impl IntoView {
  let presence = use_presence(open);
  let backdrop = Backdrop::new(open);
  use_escape_close(open);
  crate::overlay::use_scroll_lock(open);
  crate::focus::use_modal_focus(open);
  let SearchOpen(search_open) = expect_context::<SearchOpen>();
  let query = RwSignal::new(String::new());

  let sections = Memo::new(move |_| {
    let q = query.get().trim().to_lowercase();
    let mut out: Vec<(&'static str, Vec<&'static Module>)> = Vec::new();
    for (label, groups) in SECTIONS {
      let items: Vec<&'static Module> = groups
        .iter()
        .flat_map(|g| registry::in_group(g))
        // 译文与原文都匹配：非中文界面里用户输入的是看得见的译文（同调谐台）。
        .filter(|m| {
          q.is_empty()
            || m.nav_href().contains(&q)
            || m.title.to_lowercase().contains(&q)
            || core_text_tracked(m.title).to_lowercase().contains(&q)
        })
        .collect();
      if !items.is_empty() {
        out.push((label, items));
      }
    }
    out
  });

  view! {
    {move || {
      presence.visible.get().then(|| {
        view! {
          <Portal>
            <div
              class="md-sheet"
              data-anchored="true"
              data-state=move || { if presence.closing.get() { "closed" } else { "open" } }
              inert=move || presence.closing.get().then_some("")
              aria-hidden=move || presence.closing.get().then_some("true")
              // 只在「点遮罩」时关闭，不在面板上 `stop_propagation`：后者会把路由的全局
              // click 拦截一起挡掉，面板里的链接就退化成整页跳转（见 `backdrop.rs`）。
              on:pointerdown=backdrop.on_pointerdown()
              on:click=backdrop.on_click()
            >
              <div
                class="md-sheet-panel md-menu-panel"
                data-modal-root=""
                tabindex="-1"
                role="dialog"
                aria-modal="true"
                aria-label=move || t("shell.main-navigation")
              >
                <div class="md-sheet-head">
                  <span class="md-h2">{move || t("shell.main-navigation")}</span>
                  <button
                    type="button"
                    class="md-icon-btn"
                    aria-label=move || t("shell.close-menu")
                    on:click=move |_| open.set(false)
                  >
                    <ModernIcon kind=MIcon::Close />
                  </button>
                </div>

                <div class="md-menu-search">
                  <ModernIcon kind=MIcon::Search />
                  // 刻意的例外：`.md-menu-search` 是「图标 + 输入 + 站内搜索按钮」一行，
                  // 包 `md-input-wrap` 会多一层盒模型（登记在 `docs/ui/pixel.md` §10）。
                  <input
                    class="md-menu-input"
                    type="search"
                    placeholder=move || t("shell.search")
                    aria-label=move || t("shell.search")
                    prop:value=move || query.get()
                    on:input=move |e| query.set(event_target_value(&e))
                  />
                  // 目录里的搜索框只过滤这份目录（三大板块），「全站搜索」另开弹层：
                  // 触屏没有 ⌘K，这一颗就是手机上真正的入口。
                  <button
                    type="button"
                    class="md-chip"
                    title=move || t("shell.site-search")
                    on:click=move |_| {
                      open.set(false);
                      search_open.set(true);
                    }
                  >
                    {move || t("shell.site-search")}
                  </button>
                </div>

                <div class="md-sheet-body md-menu-body">
                  {move || {
                    let list = sections.get();
                    if list.is_empty() {
                      return view! {
                        <p class="md-palette-empty">{move || t("exam.no-matches")}</p>
                      }
                        .into_any();
                    }
                    list
                      .into_iter()
                      .map(|(label, items)| {
                        view! {
                          <section class="md-menu-section">
                            <p class="md-group-title">
                              {move || t(label)}
                              <span class="md-menu-count">{items.len()}</span>
                            </p>
                            <div class="md-menu-grid">
                              {items
                                .into_iter()
                                .map(|m| {
                                  view! {
                                    <a
                                      class="md-menu-item"
                                      href=m.nav_href()
                                      on:click=move |_| open.set(false)
                                    >
                                      <ModernIcon
                                        kind=module_icon(m)
                                        class="md-menu-icon-sm"
                                      />
                                      {core_text_tracked(m.title)}
                                    </a>
                                  }
                                })
                                .collect_view()}
                            </div>
                          </section>
                        }
                      })
                      .collect_view()
                      .into_any()
                  }}
                </div>

                <div class="md-sheet-foot">
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="md-stat-label">{move || t("shell.language")}</span>
                    {Locale::ALL
                      .iter()
                      .map(|&l| {
                        view! {
                          <button
                            type="button"
                            class="md-chip"
                            aria-pressed=move || (i18n::locale().get() == l).to_string()
                            on:click=move |_| i18n::set_locale(l)
                          >
                            {l.label()}
                          </button>
                        }
                      })
                      .collect_view()}
                  </div>
                  <div class="flex flex-wrap items-center gap-2">
                    <button
                      type="button"
                      class="md-btn md-btn-quiet"
                      on:click=move |_| {
                        open.set(false);
                        settings_open.set(true);
                      }
                    >
                      <ModernIcon kind=MIcon::Settings />
                      {move || t("shell.settings")}
                    </button>
                    <a class="md-btn md-btn-ghost" href="/backup" on:click=move |_| open.set(false)>
                      {move || t("tools.data-backup")}
                    </a>
                  </div>
                </div>
              </div>
            </div>
          </Portal>
        }
      })
    }}
  }
}
