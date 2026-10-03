//! 知识库「术语 / 概念」网格区块。

use leptos::prelude::*;

use crate::data;
use crate::i18n::t;

/// 术语 / 概念网格：两列 `(名称, 说明)` 卡片。
///
/// `inline = true` 时名称与说明排在同一行（基线对齐），适合名称很短、说明也短的条目；
/// 默认上下两行，说明较长时更易读。
///
/// `ui = true` 表示文案写在 `crates/app` 里，查界面词典 [`t`]；默认 `false` 表示来自
/// `crates/core`，查知识库词典 [`data::kt`]。
#[component]
pub fn ConceptsSection(
  #[prop(into)] title: String,
  items: &'static [(&'static str, &'static str)],
  #[prop(optional)] inline: bool,
  #[prop(optional)] ui: bool,
) -> impl IntoView {
  let row = if inline {
    "flex items-baseline gap-2 rounded-lg px-3 py-2"
  } else {
    "flex flex-col gap-1 rounded-lg px-3 py-2"
  };
  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t(&title)}</h2>
      <div class="grid gap-1 p-4 sm:grid-cols-2">
        {items
          .iter()
          .map(|(term, desc)| {
            view! {
              <div class=row>
                <span class="text-sm font-medium">{move || translate(term, ui)}</span>
                <span class="text-sm text-muted-foreground">{move || translate(desc, ui)}</span>
              </div>
            }
          })
          .collect_view()}
      </div>
    </section>
  }
}

/// 按文案来源选词典：`ui` 为真走界面词典，否则走知识库词典。
fn translate(text: &str, ui: bool) -> String {
  if ui {
    return t(text);
  }
  data::track_knowledge();
  data::kt(text)
}
