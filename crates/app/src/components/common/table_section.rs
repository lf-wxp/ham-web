//! 知识库三列表格区块。

use leptos::prelude::*;

use crate::data;
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};

/// 三列表格：第一列为条目标题（强调），后两列为说明（次要色）。
///
/// 知识库里这类「名称 / 含义 / 影响」表出现了十余次，原本每处都要手写一遍
/// `overflow-x-auto` + `<table>` + `<thead>` 骨架。表格本体交给 [`Table`]；
/// `class` 透传给它（作用在 `<table>` 上），列多的表写 `min-w-[720px]` 留出
/// 横向滚动空间，缺省 `min-w-[640px]`。`min_w` 用类名字面量而不是数字：
/// Tailwind 只扫描完整类名，动态拼 `min-w-{n}px` 扫不到。
#[component]
pub fn TableSection(
  #[prop(into)] title: String,
  headers: &'static [&'static str],
  rows: &'static [(&'static str, &'static str, &'static str)],
  #[prop(optional, into, default = "min-w-[640px]".into())] class: String,
) -> impl IntoView {
  // `StoredValue`（Copy）：标题要进两个 `'static` 属性闭包（h2 文案与表格无障碍名），
  // 直接 move 会让组件体闭包退化（见 docs/ui-components.md §8 坑 1）。
  let title = StoredValue::new(title);
  view! {
    <section class="pxl-card border bg-card">
      <h2 class="pxl-section-title">{move || t(&title.get_value())}</h2>
      <Table aria_label=Signal::derive(move || t(&title.get_value())) class=class>
        <TableHead>
          <TableRow>
            {headers
              .iter()
              .map(|h| {
                view! { <TableHeader>{move || t(h)}</TableHeader> }
              })
              .collect_view()}
          </TableRow>
        </TableHead>
        <TableBody>
          {rows
            .iter()
            .map(|(name, mid, last)| {
              view! {
                <TableRow>
                  <TableCell class="align-top whitespace-nowrap">
                    {move || {
                      data::track_knowledge();
                      data::kt(name)
                    }}
                  </TableCell>
                  <TableCell class="align-top text-muted-foreground">
                    {move || {
                      data::track_knowledge();
                      data::kt(mid)
                    }}
                  </TableCell>
                  <TableCell class="align-top text-muted-foreground">
                    {move || {
                      data::track_knowledge();
                      data::kt(last)
                    }}
                  </TableCell>
                </TableRow>
              }
            })
            .collect_view()}
        </TableBody>
      </Table>
    </section>
  }
}
