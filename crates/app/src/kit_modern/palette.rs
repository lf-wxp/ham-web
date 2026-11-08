//! 调谐台（Command Palette）：`⌘K` / `Ctrl K` 唤起的**路由跳转**。
//!
//! 为什么在导航之外还要有它：现代风的顶栏只留得下 6 个一级入口，剩下的 170+ 页面
//! 靠层层菜单找，是最「上一代」的交互。这里把全站路由当频率刻度：键入几个字直接调台。
//!
//! 与站内搜索的分工：顶栏搜索按钮与 `/` 打开的是 `components/search_dialog.rs`（搜内容：
//! 题库 / 术语 / 工具 / 日志 / 页面），这里只按页面标题跳转 —— 两者都不再抢对方的键。
//!
//! 键盘优先：↑↓ 选择、Enter 跳转、Esc 退出；鼠标与触屏同样可用。

use ham_web_core::registry;
use leptos::ev;
use leptos::portal::Portal;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use send_wrapper::SendWrapper;

use crate::data::core_text_tracked;
use crate::i18n::t;

use super::backdrop::Backdrop;
use super::module_icon::module_icon;
use super::{MIcon, ModernIcon};
use crate::focus::use_escape_close;
use crate::presence::use_presence;

/// 调谐台里的一条「频道」。
#[derive(Clone, Copy, PartialEq, Eq)]
struct Channel {
  title: &'static str,
  href: &'static str,
  /// 保留模块本身：图标推断要按 title + group 走。
  module: &'static registry::Module,
}

fn channels() -> Vec<Channel> {
  let mut out: Vec<Channel> = Vec::new();
  for m in registry::MODULES {
    out.push(Channel {
      title: m.title,
      href: m.nav_href(),
      module: m,
    });
  }
  out
}

/// 分组在菜单里的先后（考试中心 → 知识库 → 工具，与目录面板同一套顺序）。
///
/// `registry::MODULES` 是按「页面归属」登记的，分组顺序是交错的；不排序直接切段，
/// 同一组会碎成好几段、段标题重复出现（实测「闯关 / 闯关 / 练题 / 模拟考 / 练题…」）。
/// `None`（不在导航里的 5 条）排最后。
fn group_rank(group: Option<&str>) -> usize {
  let Some(g) = group else {
    return usize::MAX;
  };
  if let Some(i) = registry::EXAM_GROUPS.iter().position(|x| *x == g) {
    return i;
  }
  if let Some(i) = registry::KNOWLEDGE_GROUPS.iter().position(|x| *x == g) {
    return registry::EXAM_GROUPS.len() + i;
  }
  if let Some(i) = registry::TOOL_GROUPS.iter().position(|x| *x == g) {
    return registry::EXAM_GROUPS.len() + registry::KNOWLEDGE_GROUPS.len() + i;
  }
  usize::MAX
}

/// 游戏化页面的**白话别名**（见 `docs/ui/modern.md` §4.4）。
///
/// `/map`（闯关地图）与 `/bestiary`（怪物图鉴）用的是世界观语言：对老用户是好记的，
/// 对「想找错题复习」的人则完全不可检索 —— 调谐台是全站唯一的「按概念找页面」入口，
/// 这里补一个功能名，显示在标题后面，并参与输入过滤。
fn page_alias(href: &str) -> Option<&'static str> {
  match href {
    "/map" => Some("shell.page-alias-map"),
    "/bestiary" => Some("shell.page-alias-bestiary"),
    _ => None,
  }
}

/// 关键词查询的匹配质量分档：0 标题命中（中文原文或当前界面语言的译文）→ 1 白话别名命中 →
/// 2 路由命中 → 3 其余。
///
/// 译文也算「标题命中」：英文 / 西语界面里用户看到并输入的就是译文，若只比中文原文，
/// 这两种语言下的标题命中会全部掉到最后一档，排序等于失效。
///
/// 只在 `query` 非空时参与排序（空查询按分组排，见 `results`）。分档内部仍保持
/// `registry` 顺序，所以同档平局不会因为排序而抖动。
fn match_rank(ch: &Channel, q: &str) -> usize {
  if ch.title.to_lowercase().contains(q) || core_text_tracked(ch.title).to_lowercase().contains(q) {
    0
  } else if page_alias(ch.href).is_some_and(|k| t(k).to_lowercase().contains(q)) {
    1
  } else if ch.href.contains(q) {
    2
  } else {
    3
  }
}

/// 一段结果：组名 + `(全局序号, 频道)`。
type Run = (Option<&'static str>, Vec<(usize, Channel)>);

/// 把结果切成「相邻同组」的段：181 条平铺时用户只能靠滚碰运气（见 `docs/ui/modern.md` §4.4）。
///
/// 只切段、不重排 —— 视觉顺序与 `results` 顺序必须一致，否则 `↑↓` 的游标编号
/// 会和 `aria-activedescendant` 指到的元素对不上。
/// 段标题直接用 registry 的分组名当 i18n key（与目录面板同一套词条），不新增文案。
fn group_runs(items: &[Channel]) -> Vec<Run> {
  let mut runs: Vec<Run> = Vec::new();
  for (index, ch) in items.iter().enumerate() {
    let group = ch.module.group;
    match runs.last_mut() {
      Some((g, rows)) if *g == group => rows.push((index, *ch)),
      _ => runs.push((group, vec![(index, *ch)])),
    }
  }
  runs
}

/// 关键词查询的结果：不分组，整体作为一段（无组名）。
fn flat_run(items: &[Channel]) -> Vec<Run> {
  if items.is_empty() {
    return Vec::new();
  }
  vec![(None, items.iter().copied().enumerate().collect())]
}

#[cfg(test)]
mod tests {
  use super::*;

  fn chans() -> Vec<Channel> {
    channels().into_iter().take(6).collect()
  }

  #[test]
  fn flat_run_keeps_global_index_and_has_no_group() {
    let items = chans();
    let runs = flat_run(&items);
    assert_eq!(runs.len(), 1);
    assert!(runs[0].0.is_none());
    let idx: Vec<usize> = runs[0].1.iter().map(|(i, _)| *i).collect();
    assert_eq!(idx, (0..items.len()).collect::<Vec<_>>());
    assert!(flat_run(&[]).is_empty());
  }

  #[test]
  fn group_runs_indices_are_global_and_contiguous() {
    let items = chans();
    let runs = group_runs(&items);
    let idx: Vec<usize> = runs
      .iter()
      .flat_map(|(_, r)| r.iter().map(|(i, _)| *i))
      .collect();
    assert_eq!(idx, (0..items.len()).collect::<Vec<_>>());
  }

  #[test]
  fn group_rank_puts_ungrouped_last() {
    assert_eq!(group_rank(None), usize::MAX);
    let first = registry::EXAM_GROUPS[0];
    assert_eq!(group_rank(Some(first)), 0);
  }
}

#[component]
pub fn ModernPalette(open: RwSignal<bool>) -> impl IntoView {
  let query = RwSignal::new(String::new());
  let cursor = RwSignal::new(0usize);
  let navigate = use_navigate();
  let presence = use_presence(open);
  let backdrop = Backdrop::new(open);
  crate::overlay::use_scroll_lock(open);
  crate::focus::use_modal_focus(open);
  // Esc 走全局监听而不是输入框的 `on:keydown`：Tab 到列表项之后焦点就不在输入框上了，
  // 挂在那里的 Esc 会失效（同 Kit 其它四个弹层都用这一份）。
  use_escape_close(open);
  let input_ref = NodeRef::<leptos::html::Input>::new();

  // 打开时把焦点交给输入框（`autofocus` 在 Portal 里不生效）。
  // rAF 回调已经脱离响应式上下文，读 ref 必须走 untracked，否则会刷「outside a tracking context」。
  Effect::new(move |_| {
    if open.get() {
      request_animation_frame(move || {
        if let Some(el) = input_ref.get_untracked() {
          let _ = el.focus();
        }
      });
    }
  });
  let all = StoredValue::new(channels());

  // 全局快捷键：⌘K / Ctrl+K 开合。`/` 留给站内搜索（见模块注释），这里不再抢。
  let handle = window_event_listener(ev::keydown, move |e| {
    let meta = e.meta_key() || e.ctrl_key();
    // 认物理键位 `KeyK`：非拉丁键盘布局（俄语、希腊语……）下 `key` 不是 "k"。
    // 按住不放会连发 keydown，不处理 `repeat`，否则调谐台会一开一合地闪。
    if meta && !e.repeat() && (e.code() == "KeyK" || e.key().eq_ignore_ascii_case("k")) {
      e.prevent_default();
      open.update(|v| *v = !*v);
      query.set(String::new());
      cursor.set(0);
    }
  });
  let handle = SendWrapper::new(Some(handle));
  on_cleanup(move || {
    if let Some(h) = handle.take() {
      h.remove();
    }
  });

  let results = Memo::new(move |_| {
    let q = query.get().trim().to_lowercase();
    all.with_value(|list| {
      let mut hits = list
        .iter()
        // 同时匹配中文原文与当前界面语言的译文：英文 / 西语界面里用户看到的是译文，
        // 只匹配原文的话输入 "Glossary" 什么都搜不到。译文在 Memo 内取，切语言会重算。
        .filter(|c| {
          q.is_empty()
            || c.href.contains(&q)
            || c.title.to_lowercase().contains(&q)
            || core_text_tracked(c.title).to_lowercase().contains(&q)
            // 别名也要能搜到：用户想找「错题复习」，不会知道那页叫「怪物图鉴」。
            || page_alias(c.href).is_some_and(|k| t(k).to_lowercase().contains(&q))
        })
        .copied()
        .collect::<Vec<_>>();
      // 排序而不是只切段：`registry::MODULES` 的分组是交错的，不排序时同一组会碎成几段、
      // 段标题重复。`sort_by_key` 稳定，组内保持 registry 的顺序。
      //
      // 有关键词时**先按匹配质量再按分组**：只按分组排会让 `group: None` 的页面（`/achievements`
      // 这类）恒排最后，随后被 `truncate(40)` 裁掉 —— 而搜名字时它们恰恰是最该命中的。
      if q.is_empty() {
        hits.sort_by_key(|c| group_rank(c.module.group));
      } else {
        // `match_rank` 要做小写化与译文查表，用缓存键让每条只算一次，而不是每次比较都算。
        hits.sort_by_cached_key(|c| (match_rank(c, &q), group_rank(c.module.group)));
      }
      hits.truncate(40);
      hits
    })
  });

  // ↑↓ 移动选中项时把它滚进列表可视区（最多 40 项，超出面板高度后选中项会跑到视野外）。
  // 只在列表容器内部改 `scroll_top`，页面不跟着动，见 `popover::scroll_into_view_local`。
  Effect::new(move |_| {
    let len = results.with(Vec::len);
    if len == 0 {
      return;
    }
    // 结果变少时 `cursor` 可能停在下标之外（除了 `on:input` 没有别处重置它）。
    // 与 `aria-activedescendant` 取同一个 `min(len - 1)`，两边不能指向不同的项。
    let index = cursor.get().min(len - 1);
    request_animation_frame(move || {
      if let Some(list) = crate::util::document().get_element_by_id("md-palette-list") {
        crate::ui::popover::scroll_into_view_local(&list, &format!("#md-palette-item-{index}"));
      }
    });
  });

  // `Callback`（Copy）：要进多个 `on:click` 闭包，普通闭包会被第一个 move 掉。
  let go = Callback::new(move |href: String| {
    // 先关面板、再换页，**不套 View Transition**。
    //
    // 曾经是 `with_view_transition(navigate)`（想给「跳页」加一次共享元素过渡），结果是
    // 点下去之后整页冻住数秒：VT 在动画开始前要先拍一张「旧状态」快照，而那一刻面板还开着，
    // 快照要把整屏弹层（`backdrop-filter: blur(26px)` + 181 行结果 + 全屏遮罩）一起光栅化。
    // 实测（同一台机器，点选后到新页面首帧）：套 VT ≈ 4.0 s，去掉 VT 21–74 ms。
    // 这与 `docs/ui/interaction.md` 的硬约束 1（高频操作禁用 VT）一致 —— 换页只保留
    // `#route-content` 的淡入，退出动效归 `presence`，一个表面只有一条动效源。
    open.set(false);
    query.set(String::new());
    navigate(&href, Default::default());
  });

  view! {
    {move || {
      presence.visible.get().then(|| {
        view! {
          <Portal>
            <div
              class="md-modal md-palette-backdrop"
              data-state=presence.state()
              inert=presence.inert()
              aria-hidden=presence.aria_hidden()
              on:pointerdown=backdrop.on_pointerdown()
              on:click=backdrop.on_click()
            >
              <div
                class="md-palette"
                data-modal-root=""
                tabindex="-1"
                role="dialog"
                aria-modal="true"
                aria-label=move || t("shell.search")
              >
                <div class="md-palette-input-row">
                  <ModernIcon kind=MIcon::Search />
                  // 刻意的例外：这一行是「图标 + 输入 + kbd」的 flex 行，
                  // `ModernInput` 的 `md-input-wrap` 与清除按钮会改掉它的布局
                  // （登记在 `docs/ui/pixel.md` §10 与 `check-ui` 的封闭清单里）。
                  <input
                    node_ref=input_ref
                    class="md-palette-input"
                    type="text"
                    role="combobox"
                    aria-expanded="true"
                    aria-controls="md-palette-list"
                    aria-autocomplete="list"
                    // 读屏只能通过 `aria-activedescendant` 知道「↑↓ 现在停在哪一条」；
                    // 列表为空时指向不存在的 id，所以那时返回 `None`（属性被移除）。
                    aria-activedescendant=move || {
                      let len = results.with(Vec::len);
                      (len > 0)
                        .then(|| format!("md-palette-item-{}", cursor.get().min(len - 1)))
                    }
                    placeholder=move || t("shell.search")
                    aria-label=move || t("shell.search")
                    prop:value=move || query.get()
                    on:input=move |e| {
                      query.set(event_target_value(&e));
                      cursor.set(0);
                    }
                    on:keydown=move |e| {
                      let len = results.with(Vec::len);
                      match e.key().as_str() {
                        "ArrowDown" => {
                          e.prevent_default();
                          cursor.update(|c| *c = if len == 0 { 0 } else { (*c + 1) % len });
                        }
                        "ArrowUp" => {
                          e.prevent_default();
                          cursor
                            .update(|c| {
                              *c = if len == 0 { 0 } else { (*c + len - 1) % len };
                            });
                        }
                        // 输入法组词中的 Enter 是「上屏」，不是「跳转」（keyCode 229 是
                        // 部分浏览器在组词结束那一下的兜底标记）。
                        "Enter" if !e.is_composing() && e.key_code() != 229 => {
                          if let Some(ch) = results.with(|r| r.get(cursor.get()).copied()) {
                            go.run(ch.href.to_owned());
                          }
                        }
                        _ => {}
                      }
                    }
                  />
                  <kbd class="md-kbd">"esc"</kbd>
                </div>

                // 空结果提示放在 listbox 外面：listbox 只允许包含 option。
                {move || {
                  results
                    .with(Vec::is_empty)
                    .then(|| {
                      view! {
                        <p class="md-palette-empty" role="status">
                          {move || t("exam.no-matches")}
                        </p>
                      }
                    })
                }}
                <div
                  class="md-palette-list"
                  id="md-palette-list"
                  role="listbox"
                  aria-label=move || t("shell.search")
                  hidden=move || results.with(Vec::is_empty)
                >
                  {move || {
                    let items = results.get();
                    // 按 registry 的子分组切段（`group_runs`）：段标题沿用目录面板的词条，
                    // 只在「换组」处出现一次，不重复贴在每一行上。
                    //
                    // 有关键词时按「匹配质量」排序，同一组会被拆散；此时分组标题只会重复出现，
                    // 所以合成一段、不画标题（序号仍是全局序号，游标与 `aria-activedescendant` 一致）。
                    let runs = if query.with(|q| q.trim().is_empty()) {
                      group_runs(&items)
                    } else {
                      flat_run(&items)
                    };
                    runs
                      .into_iter()
                      .map(|(group, rows)| {
                        let body = rows
                          .into_iter()
                          .map(|(i, ch)| {
                            let active = move || cursor.get() == i;
                            view! {
                              <button
                                type="button"
                                class="md-palette-item"
                                id=format!("md-palette-item-{i}")
                                role="option"
                                aria-selected=move || active().to_string()
                                data-active=move || active().to_string()
                                on:click=move |_| go.run(ch.href.to_owned())
                              >
                                <ModernIcon kind=module_icon(ch.module) class="md-menu-icon-sm" />
                                <span class="md-palette-title">{core_text_tracked(ch.title)}</span>
                                {page_alias(ch.href)
                                  .map(|k| view! { <span class="md-palette-alias">{move || t(k)}</span> })}
                                <span class="md-palette-href">{ch.href}</span>
                                <ModernIcon kind=MIcon::ArrowRight class="md-palette-go" />
                              </button>
                            }
                          })
                          .collect_view();
                        view! {
                          <div
                            class="md-palette-sec"
                            role="group"
                            aria-label=move || group.map(t)
                          >
                            {group
                              .map(|g| view! { <p class="md-palette-group">{move || t(g)}</p> })}
                            {body}
                          </div>
                        }
                      })
                      .collect_view()
                  }}
                </div>

                <div class="md-palette-foot">
                  <span>
                    <kbd class="md-kbd">"↑"</kbd> <kbd class="md-kbd">"↓"</kbd> " "
                    {move || t("shell.search")}
                  </span>
                  <span>
                    <kbd class="md-kbd">"enter"</kbd> " " {move || t("exam.go")}
                  </span>
                </div>
              </div>
            </div>
          </Portal>
        }
      })
    }}
  }
}
