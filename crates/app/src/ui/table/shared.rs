//! table 一族共用的样式 token。

use crate::cn::cn;

/// 表格基线。外观在 `style/pixel/controls.css` 的 `.pxl-table`（网格线、内边距、
/// 表头底纹、行悬停），这里拼的只是布局与宽度。
///
/// `border-collapse` 刻意写成工具类：`surfaces.css` 里「卡片内贴边表格去掉与卡片边框
/// 重合的最外圈双线」拿 `table.border-collapse` 当选择器，挪进 `.pxl-table` 会让
/// 那组规则失配，卡片里的表格重新出现双线。
pub(super) const TABLE_BASE: &str = "pxl-table w-full border-collapse";

/// 根组件类名：基线 + 调用方覆盖（`min-w-*`、字号等；后写胜出）。
pub(super) fn table_class(borderless: bool, extra: &str) -> String {
  let no_border = if borderless {
    "[&_:is(th,td)]:border-0"
  } else {
    ""
  };
  cn(&[TABLE_BASE, no_border, extra])
}
