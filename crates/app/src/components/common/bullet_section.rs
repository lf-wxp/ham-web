//! 知识库要点列表区块。

use leptos::prelude::*;

use super::section_shell::{SectionShell, section_text};

/// 要点列表：带项目符号的 `&str` 列表。
///
/// `ui = true` 表示文案写在 `crates/app` 里，查界面词典 [`crate::i18n::t`]；默认 `false`
/// 表示来自 `crates/core`，查知识库词典 [`crate::data::kt`]。两者混用会让译文永远命中不到，
/// 因此由调用方声明来源。
///
/// 项目符号两套皮肤不同：现代风是 CSS 画的圆点（`.md-bullet-dot`），装饰性的，对读屏隐藏；
/// 像素风是文本 `•`，保持与页面里手写的像素风要点列表一致。
#[component]
pub fn BulletSection(
  #[prop(into)] title: String,
  items: &'static [&'static str],
  #[prop(optional)] ui: bool,
) -> impl IntoView {
  let body = if crate::kit::is_modern_at_mount() {
    view! {
      <ul class="md-bullets">
        {items
          .iter()
          .map(|note| {
            view! {
              <li>
                <span class="md-bullet-dot" aria-hidden="true"></span>
                <span>{move || section_text(note, ui)}</span>
              </li>
            }
          })
          .collect_view()}
      </ul>
    }
    .into_any()
  } else {
    view! {
      <ul class="space-y-2 p-4">
        {items
          .iter()
          .map(|note| {
            view! {
              <li class="flex gap-2 text-sm text-muted-foreground">
                <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                <span>{move || section_text(note, ui)}</span>
              </li>
            }
          })
          .collect_view()}
      </ul>
    }
    .into_any()
  };
  view! { <SectionShell title=title>{body}</SectionShell> }
}
