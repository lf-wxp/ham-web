//! 知识库「术语 / 概念」网格区块。

use leptos::prelude::*;

use super::section_shell::{SectionShell, section_text};

/// 术语 / 概念网格：两列 `(名称, 说明)` 卡片。
///
/// `inline = true` 时名称与说明排在同一行（基线对齐），适合名称很短、说明也短的条目；
/// 默认上下两行，说明较长时更易读。
///
/// `ui = true` 表示文案写在 `crates/app` 里，查界面词典 [`crate::i18n::t`]；默认 `false`
/// 表示来自 `crates/core`，查知识库词典 [`crate::data::kt`]。
#[component]
pub fn ConceptsSection(
  #[prop(into)] title: String,
  items: &'static [(&'static str, &'static str)],
  #[prop(optional)] inline: bool,
  #[prop(optional)] ui: bool,
) -> impl IntoView {
  // 现代风用 `<dl>`（术语 / 定义有语义），像素风用两列网格的 `<div>`：标签不同即语义不同，
  // 这里必须分叉；能共用的外壳与查词规则已经收在 `SectionShell` 里。
  let body = if crate::kit::is_modern_at_mount() {
    view! {
      <dl class="md-concepts">
        {items
          .iter()
          .map(|(term, desc)| {
            view! {
              <div class="md-concept">
                <dt>{move || section_text(term, ui)}</dt>
                <dd class="md-muted">{move || section_text(desc, ui)}</dd>
              </div>
            }
          })
          .collect_view()}
      </dl>
    }
    .into_any()
  } else {
    let row = if inline {
      "flex items-baseline gap-2 px-3 py-2"
    } else {
      "flex flex-col gap-1 px-3 py-2"
    };
    view! {
      <div class="grid gap-1 p-4 sm:grid-cols-2">
        {items
          .iter()
          .map(|(term, desc)| {
            view! {
              <div class=row>
                <span class="text-sm">{move || section_text(term, ui)}</span>
                <span class="text-sm text-muted-foreground">{move || section_text(desc, ui)}</span>
              </div>
            }
          })
          .collect_view()}
      </div>
    }
    .into_any()
  };
  view! { <SectionShell title=title>{body}</SectionShell> }
}
