//! 知识库三列表格区块。

use leptos::prelude::*;

use crate::data;
use crate::i18n::t;

/// 表格单元格样式（与 `style/input.css` 的「卡片 + 内部网格线」一致：只写 `border`）。
const CELL: &str = "border px-3 py-2 text-left align-top";

/// 三列表格：第一列为条目标题（强调），后两列为说明（次要色）。
///
/// 知识库里这类「名称 / 含义 / 影响」表出现了十余次，原本每处都要手写一遍
/// `overflow-x-auto` + `<table>` + `<thead>` 骨架。`min_width` 用于给列多的表留出
/// 横向滚动空间，默认 640px。
#[component]
pub fn TableSection(
  #[prop(into)] title: String,
  headers: &'static [&'static str],
  rows: &'static [(&'static str, &'static str, &'static str)],
  #[prop(optional, default = 640)] min_width: u32,
) -> impl IntoView {
  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t(&title)}</h2>
      <div class="overflow-x-auto">
        <table
          class="w-full border-collapse text-sm"
          style=format!("min-width: {min_width}px")
        >
          <thead class="bg-muted/60 text-xs">
            <tr>
              {headers
                .iter()
                .map(|h| {
                  view! { <th class=CELL>{move || t(h)}</th> }
                })
                .collect_view()}
            </tr>
          </thead>
          <tbody>
            {rows
              .iter()
              .map(|(name, mid, last)| {
                view! {
                  <tr class="border-t transition-colors hover:bg-muted/40">
                    <td class=format!("{CELL} whitespace-nowrap font-medium")>{move || { data::track_knowledge(); data::kt(name) }}</td>
                    <td class=format!("{CELL} text-muted-foreground")>{move || { data::track_knowledge(); data::kt(mid) }}</td>
                    <td class=format!("{CELL} text-muted-foreground")>{move || { data::track_knowledge(); data::kt(last) }}</td>
                  </tr>
                }
              })
              .collect_view()}
          </tbody>
        </table>
      </div>
    </section>
  }
}
