//! table 一族共用的样式 token。

use crate::cn::cn;

/// 表格基线：现代风 `md-table`（细线 + 无底色表头），像素风 `pxl-table`（网格线 / 表头底纹）。
///
/// `border-collapse` 刻意写成工具类：`surfaces.css` 里「卡片内贴边表格去掉与卡片边框
/// 重合的最外圈双线」拿 `table.border-collapse` 当选择器，挪进 `.pxl-table` 会让
/// 那组规则失配，卡片里的表格重新出现双线。
#[must_use]
pub(super) fn table_base() -> &'static str {
  if crate::kit::is_modern_at_mount() {
    "md-table w-full border-collapse"
  } else {
    "pxl-table w-full border-collapse"
  }
}

/// 表格密度：标准（8/12 内距）与紧凑（5/8 内距，行高更小）。
///
/// 判据：行数 ≥ 15 用紧凑档，否则标准档；`/tools` 下的参考表是已定的例外（历史上就是紧凑档）。
/// 判据、例外与量过的数据以 `docs/ui/modern.md` §4.3 为准，这里不再重复。
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum TableDensity {
  #[default]
  Standard,
  Compact,
}

impl TableDensity {
  /// 密度类。**只在现代风下产出**：像素表格的密度由它自己的 `px-3 py-2` 决定，
  /// 把 `md-table-compact` 挂到像素表格上会让现代风的层外规则漏进像素模式
  /// （表现为内距变成 6/8、左边框消失）。
  fn class(self) -> &'static str {
    match self {
      TableDensity::Standard => "",
      TableDensity::Compact => {
        if crate::kit::is_modern_at_mount() {
          "md-table-compact"
        } else {
          ""
        }
      }
    }
  }
}

/// 根组件类名：基线 + 密度 + 调用方覆盖（`min-w-*`、字号等；后写胜出）。
pub(super) fn table_class(borderless: bool, density: TableDensity, extra: &str) -> String {
  let no_border = if borderless {
    "[&_:is(th,td)]:border-0"
  } else {
    ""
  };
  cn(&[table_base(), density.class(), no_border, extra])
}
