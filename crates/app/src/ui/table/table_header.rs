//! [`TableHeader`]：表头单元格（`<th>`）。

use leptos::prelude::*;

/// 这一列的排序状态（对应 `aria-sort`）。
///
/// 只表达语义，不负责排序本身：点表头要做什么由调用方决定，组件只保证
/// 读屏能念出「这一列按升序 / 降序排」—— 全仓曾经没有任何 `aria-sort`，
/// 排序按钮点了之后只有箭头图标在变，读屏用户完全不知道当前顺序。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
  Ascending,
  Descending,
  /// 可排序但当前不是排序列（`aria-sort="none"`）。
  None,
}

impl SortOrder {
  const fn as_str(self) -> &'static str {
    match self {
      Self::Ascending => "ascending",
      Self::Descending => "descending",
      Self::None => "none",
    }
  }
}

/// 表头单元格。首列作为行标题时传 `scope="row"`，列标题是默认（CSS 里
/// `.pxl-table :is(th, td)` 统一左对齐，列标题要居中另写 `class`）。
/// 分组标题跨多列时传 `colspan`；这一列可排序时传 `sort`。
#[component]
pub fn TableHeader(
  #[prop(optional)] scope: Option<&'static str>,
  #[prop(optional)] colspan: Option<u32>,
  /// 排序状态（见 [`SortOrder`]）。只输出 `aria-sort`，排序逻辑与箭头图标由调用方给。
  #[prop(optional)]
  sort: Option<SortOrder>,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  view! {
    <th
      data-slot="table-header"
      scope=scope
      colspan=colspan
      aria-sort=sort.map(SortOrder::as_str)
      class=class
    >
      {children()}
    </th>
  }
}
