//! 知识库要点列表区块。

use leptos::prelude::*;

use crate::data;
use crate::i18n::t;

/// 要点列表：带项目符号的 `&str` 列表。
///
/// `ui = true` 表示文案写在 `crates/app` 里，查界面词典 [`t`]；默认 `false` 表示来自
/// `crates/core`，查知识库词典 [`data::kt`]。两者混用会让译文永远命中不到，因此由调用方声明来源。
#[component]
pub fn BulletSection(
  #[prop(into)] title: String,
  items: &'static [&'static str],
  #[prop(optional)] ui: bool,
) -> impl IntoView {
  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t(&title)}</h2>
      <ul class="space-y-2 p-4">
        {items
          .iter()
          .map(|note| {
            view! {
              <li class="flex gap-2 text-sm text-muted-foreground">
                <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                <span>{move || translate(note, ui)}</span>
              </li>
            }
          })
          .collect_view()}
      </ul>
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
