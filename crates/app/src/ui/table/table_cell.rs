//! [`TableCell`]：数据单元格（`<td>`）。

use leptos::prelude::*;

use crate::cn::cn;
use crate::ui::TextValue;

/// 数据单元格。对齐 / 字号 / 颜色等修饰写在 `class`（后写胜出）。
/// 跨列空态行（「暂无数据」）传 `colspan`；装饰性单元格传 `aria_hidden=true`。
///
/// `numeric=true` 给数字列：右对齐 + 等宽数字（`tabular-nums`）。
/// 以前这件事靠调用点记得写 `text-right tabular-nums`，全仓只有 4 处写了 ——
/// 分数 / 距离 / 次数这类列在默认左对齐下各位不对齐，扫读时对不上行。
#[component]
pub fn TableCell(
  #[prop(optional, into)] class: String,
  #[prop(optional)] colspan: Option<u32>,
  #[prop(optional)] aria_hidden: bool,
  /// 数字列：右对齐并使用等宽数字。
  #[prop(optional)]
  numeric: bool,
  #[prop(optional, into)] title: Option<TextValue>,
  children: Children,
) -> impl IntoView {
  let class = if numeric {
    cn(&["text-right tabular-nums", &class])
  } else {
    class
  };
  view! {
    <td
      data-slot="table-cell"
      class=class
      colspan=colspan
      aria-hidden=aria_hidden.then_some("true")
      title=move || title.as_ref().map(TextValue::get)
    >
      {children()}
    </td>
  }
}
