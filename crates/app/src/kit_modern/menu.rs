//! 全站目录（「更多」面板）：把 `registry` 里的**全部页面**按分组铺出来，并带即时过滤。
//!
//! 之前的面板只有 9 个手写入口，170+ 页面根本摸不到；现在直接渲染 registry：
//! 新增页面只要登记进 registry，就自动出现在这里（和调谐台同一份数据源）。

use ham_web_core::registry::{self, EXAM_GROUPS, KNOWLEDGE_GROUPS, Module, TOOL_GROUPS};
use leptos::portal::Portal;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;

use crate::after_paint::{after_next_paint, is_plain_click};
use crate::components::search_dialog::SearchOpen;
use crate::data::core_text_tracked;
use crate::i18n::{self, Locale, t};

use super::backdrop::Backdrop;
use super::menu_item::menu_item;
use super::{MIcon, ModernIcon};
use crate::focus::use_escape_close;
use crate::presence::use_presence;

/// 三大板块：考试中心 / 知识库 / 工具。
const SECTIONS: &[(&str, &[&str])] = &[
  ("shell.exam-center", EXAM_GROUPS),
  ("shell.knowledge", KNOWLEDGE_GROUPS),
  ("shell.tools", TOOL_GROUPS),
];

/// 入口按块挂载，每块的个数：手机上面板首屏大约放得下这么多。
///
/// 一口气挂 180 个入口（每个一枚 SVG 图标 + 一条译文）在中端手机上要近百毫秒，
/// 面板要等它们全部建完才出第一帧。现在打开时只挂第一块，其余在面板滑入的过程中
/// 每帧补一块（`after_paint::each_frame`）。关闭时整块卸载 —— 「关了就不在 DOM 里」是
/// 现有用例与焦点管理的约定，所以没有做成常驻。
const CHUNK: usize = 24;

#[component]
pub fn ModernMenu(open: RwSignal<bool>, settings_open: RwSignal<bool>) -> impl IntoView {
  let presence = use_presence(open);
  let backdrop = Backdrop::new(open);
  use_escape_close(open);
  crate::overlay::use_scroll_lock(open);
  crate::focus::use_modal_focus(open);
  let SearchOpen(search_open) = expect_context::<SearchOpen>();
  let query = RwSignal::new(String::new());
  // 点目录里的链接：先收起面板，等这一帧画出来再导航（见 `after_paint`）。
  //
  // 不这样做的话，新页面同步渲染的那几百毫秒里面板一动不动，看起来像「点了没反应」。
  // 导航开始后主线程会被新页面占住，抽屉的退场动画跟着停在某一帧上 —— 所以退场曲线
  // 必须是「先快后慢」（`kit.css` 的 `.md-sheet[data-state="closed"]`）：第一帧就降到很淡，
  // 被冻住的也只是一层残影，而不是新旧两张页面半透明地叠在一起。
  // 不等退场播完再导航：那样新页面会整整晚 `--md-dur`，实测「可读」反而比原来晚 100–200ms。
  let navigate = use_navigate();
  let follow = Callback::new(move |(ev, href): (web_sys::MouseEvent, &'static str)| {
    open.set(false);
    if !is_plain_click(&ev) {
      return;
    }
    ev.prevent_default();
    let navigate = navigate.clone();
    after_next_paint(move || navigate(href, NavigateOptions::default()));
  });

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

  // 已放行的块数（见 [`CHUNK`]）。每次打开从 1 开始：关闭时归位，下次打开的第一帧只建第一块。
  let chunk_limit = RwSignal::new(1usize);
  // 当前目录一共几块（过滤后会变），由渲染时写入。
  let chunk_total = StoredValue::new(0usize);
  // 补齐循环的代际号：每起一个新循环加一，旧循环在下一帧发现代号变了就自行结束。
  // 不靠「有没有循环在跑」的布尔值：循环的结束条件（关闭、补齐、被顶替）散在好几处，
  // 代际号只有一个判断，也不会出现两个循环各补各的。
  let fill_gen = StoredValue::new(0u64);
  let start_fill = move || {
    let Some(mine) = fill_gen.try_update_value(|g| {
      *g += 1;
      *g
    }) else {
      return;
    };
    crate::after_paint::each_frame(move || {
      // 被更新的循环顶替、面板在补齐途中被关掉、或组件被卸载：停下。
      if fill_gen.try_get_value() != Some(mine) || !open.try_get_untracked().unwrap_or(false) {
        return false;
      }
      let (Some(current), Some(total)) =
        (chunk_limit.try_get_untracked(), chunk_total.try_get_value())
      else {
        return false;
      };
      if current >= total {
        return false;
      }
      chunk_limit.set(current + 1);
      current + 1 < total
    });
  };
  // 依赖「在不在 DOM 里」与「开没开」两个量：退场动画（约 210ms）里重新打开时，
  // `visible` 一直是真、只有 `open` 在变，只盯 `visible` 就不会重新起循环，补到一半的目录会停在半截。
  Effect::new(move |prev: Option<(bool, bool)>| {
    let visible = presence.visible.get();
    let is_open = open.get();
    if !visible {
      chunk_limit.set(1);
    } else if is_open && prev.is_none_or(|(was_visible, was_open)| !(was_visible && was_open)) {
      start_fill();
    }
    (visible, is_open)
  });

  view! {
    {move || {
      presence.visible.get().then(|| {
        view! {
          <Portal>
            <div
              class="md-sheet"
              data-anchored="true"
              data-state=presence.state()
              inert=presence.inert()
              aria-hidden=presence.aria_hidden()
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
                      chunk_total.set_value(0);
                      return view! {
                        <p class="md-palette-empty">{move || t("exam.no-matches")}</p>
                      }
                        .into_any();
                    }
                    // 全局块序号：跨板块连续编号，第 k 块在 `chunk_limit > k` 时才建。
                    let mut next_chunk = 0usize;
                    let view = list
                      .into_iter()
                      .map(|(label, items)| {
                        let count = items.len();
                        let chunks = items
                          .chunks(CHUNK)
                          .map(|chunk| {
                            let k = next_chunk;
                            next_chunk += 1;
                            // `Memo`：每放行一块，只有刚跨过门槛的那一块会重建，已建好的不动。
                            let ready = Memo::new(move |_| chunk_limit.get() > k);
                            let chunk = chunk.to_vec();
                            move || {
                              ready.get().then(|| {
                                chunk.iter().map(|&m| menu_item(m, follow)).collect_view()
                              })
                            }
                          })
                          .collect_view();
                        view! {
                          <section class="md-menu-section">
                            <p class="md-group-title">
                              {move || t(label)}
                              <span class="md-menu-count">{count}</span>
                            </p>
                            <div class="md-menu-grid">{chunks}</div>
                          </section>
                        }
                      })
                      .collect_view()
                      .into_any();
                    chunk_total.set_value(next_chunk);
                    // 目录变长（过滤框清空）而已放行的块数不够时，之前的补齐循环早就结束了，要再起一个；
                    // 否则清空过滤框之后只剩前几块，要关了重开才补全。
                    if chunk_limit.get_untracked() < next_chunk && open.get_untracked() {
                      start_fill();
                    }
                    view
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
                    <a
                      class="md-btn md-btn-ghost"
                      href="/backup"
                      on:click=move |ev| follow.run((ev, "/backup"))
                    >
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
