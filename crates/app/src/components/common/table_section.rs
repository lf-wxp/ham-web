//! 知识库三列表格区块。

use leptos::prelude::*;

use super::section_shell::{SectionShell, section_text};
use crate::i18n::t;
use crate::ui::{ScrollRegion, Table, TableBody, TableCell, TableHead, TableHeader, TableRow};

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
  // 直接 move 会让组件体闭包退化（见 docs/ui/pixel.md §8 坑 1）。
  let title = StoredValue::new(title);
  let body = if crate::kit::is_modern_at_mount() {
    view! {
      // 现代风的裸表格版式由 `.md-panel table:not(.md-table)` 承担，不走 `Table`；
      // 溢出时键盘可达（`tabindex` / `role=region`）交给与 `Table` 同一套检测的 `ScrollRegion`。
      <ScrollRegion
        class="md-table-scroll"
        aria_label=Signal::derive(move || t(&title.get_value()))
      >
        <table class=class aria-label=move || t(&title.get_value())>
          <thead>
            <tr>
              {headers
                .iter()
                .map(|h| view! { <th>{move || t(h)}</th> })
                .collect_view()}
            </tr>
          </thead>
          <tbody>
            {rows
              .iter()
              .map(|(name, mid, last)| {
                view! {
                  <tr>
                    <th scope="row">{move || section_text(name, false)}</th>
                    <td>{move || section_text(mid, false)}</td>
                    <td class="md-muted">{move || section_text(last, false)}</td>
                  </tr>
                }
              })
              .collect_view()}
          </tbody>
        </table>
      </ScrollRegion>
    }
    .into_any()
  } else {
    view! {
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
                    {move || section_text(name, false)}
                  </TableCell>
                  <TableCell class="align-top text-muted-foreground">
                    {move || section_text(mid, false)}
                  </TableCell>
                  <TableCell class="align-top text-muted-foreground">
                    {move || section_text(last, false)}
                  </TableCell>
                </TableRow>
              }
            })
            .collect_view()}
        </TableBody>
      </Table>
    }
    .into_any()
  };
  view! { <SectionShell title=title.get_value()>{body}</SectionShell> }
}
