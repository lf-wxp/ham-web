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
use super::view_transition::with_view_transition;
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
      list
        .iter()
        // 同时匹配中文原文与当前界面语言的译文：英文 / 西语界面里用户看到的是译文，
        // 只匹配原文的话输入 "Glossary" 什么都搜不到。译文在 Memo 内取，切语言会重算。
        .filter(|c| {
          q.is_empty()
            || c.href.contains(&q)
            || c.title.to_lowercase().contains(&q)
            || core_text_tracked(c.title).to_lowercase().contains(&q)
        })
        .take(40)
        .copied()
        .collect::<Vec<_>>()
    })
  });

  // ↑↓ 移动选中项时把它滚进列表可视区（最多 40 项，超出面板高度后选中项会跑到视野外）。
  // 只在列表容器内部改 `scroll_top`，页面不跟着动，见 `popover::scroll_into_view_local`。
  Effect::new(move |_| {
    let index = cursor.get();
    request_animation_frame(move || {
      if let Some(list) = crate::util::document().get_element_by_id("md-palette-list") {
        crate::ui::popover::scroll_into_view_local(&list, &format!("#md-palette-item-{index}"));
      }
    });
  });

  // `Callback`（Copy）：要进多个 `on:click` 闭包，普通闭包会被第一个 move 掉。
  let go = Callback::new(move |href: String| {
    // `use_navigate()` 返回的是 Clone 而非 Copy 的闭包，先在回调外克隆一份再进过渡回调。
    let navigate = navigate.clone();
    with_view_transition(move || navigate(&href, Default::default()));
    open.set(false);
    query.set(String::new());
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
                    items
                      .into_iter()
                      .enumerate()
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
                            <span class="md-palette-href">{ch.href}</span>
                            <ModernIcon kind=MIcon::ArrowRight class="md-palette-go" />
                          </button>
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
