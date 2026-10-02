//! 知识库要点列表区块。

use leptos::prelude::*;

use crate::data;
use crate::i18n::t;

/// 要点列表：带项目符号的 `&str` 列表。
#[component]
pub fn BulletSection(#[prop(into)] title: String, items: &'static [&'static str]) -> impl IntoView {
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
                <span>{move || { data::track_knowledge(); data::kt(note) }}</span>
              </li>
            }
          })
          .collect_view()}
      </ul>
    </section>
  }
}
