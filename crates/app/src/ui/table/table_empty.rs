//! [`TableEmpty`]：表格的空态行。

use leptos::prelude::*;

use crate::ui::{TableCell, TextValue};

/// 表格为空时的一行提示（跨列居中）。
///
/// 为什么不做成 `Table` 的内置能力：组件只拿到 `children`，判断不了
/// 「`<tbody>` 里有没有数据行」—— 那样要么再开一个 `is_empty` prop（调用点要算两遍），
/// 要么读 DOM（React/Leptos 里都是反模式）。这里只保证**外观只有一处**：
/// 以前 20 多个页面各写一行 `<tr><td colspan=…>暂无数据</td></tr>`，
/// 内距、字号、是否居中全不一样。
///
/// 卡片内的空列表也可以不用它，直接用 `EmptyState plain=true`（更大的空态）。
#[component]
pub fn TableEmpty(
  #[prop(into)] label: TextValue,
  /// 跨几列（与表头列数一致，否则右侧会空出一块没有底纹的区域）。
  #[prop(default = 1)]
  cols: u32,
  /// 追加类名（后写胜出），例如在等宽字体的表里改回正文字体。
  #[prop(optional, into)]
  class: String,
) -> impl IntoView {
  let class = crate::cn::cn(&["py-8 text-center text-sm text-muted-foreground", &class]);
  view! {
    <tr data-slot="table-empty">
      <TableCell colspan=cols class=class>
        {move || label.get()}
      </TableCell>
    </tr>
  }
}
