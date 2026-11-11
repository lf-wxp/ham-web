//! [`Table`]：表格根（滚动容器 + `<table>`）。

use leptos::prelude::*;

use crate::cn::cn;
use crate::ui::TextValue;

use super::scroll_overflow::use_scroll_overflow;
use super::shared::{TableDensity, table_class};

/// 表格根：外层 `overflow-x-auto` 滚动容器（移动端列宽保不住的兜底），内层 `<table>`。
///
/// - 列多了就在 `class` 里写 `min-w-[…]`（作用在 `<table>` 上，与滚动容器配套）；
///   列放得下就不写，滚动容器没有溢出时就是普通块。
/// - 没有可见标题的表格传 `aria_label`（读屏的表格名，e2e 也按它定位）。
/// - `borderless=true` 去掉单元格网格线（极简行列表，如训练记录）。
/// - 需要限制表格区域高度（如 `max-h-[28rem]`）时写在 `wrapper_class`（作用在滚动容器上；
///   `overflow-x-auto` 会让纵向溢出自动变为可滚动，无需再写 `overflow-auto`）。
/// - **键盘可达是默认行为**：容器溢出（`scrollWidth > clientWidth`）时自动加
///   `tabindex="0"` 与 `role="region"`（配 `aria_label` 命名该区域），不溢出时不进 Tab 序 ——
///   以前这两个属性是可选 prop，等于「让调用方记得」，实际几乎没人传，宽表在窄屏上
///   键盘用户就没法横向滚动（WCAG 2.1.1）。调用方仍可显式覆盖。
#[component]
pub fn Table(
  #[prop(optional, into)] aria_label: Option<TextValue>,
  #[prop(optional)] borderless: bool,
  #[prop(optional, into)] class: String,
  #[prop(optional, into)] wrapper_class: String,
  #[prop(optional)] tabindex: Option<&'static str>,
  #[prop(optional)] role: Option<&'static str>,
  /// 密度：默认标准（8/12 内距）；行数 ≥ 15 的表用 [`TableDensity::Compact`]（判据与例外见
  /// `docs/ui/modern.md` §4.3）。
  #[prop(optional)]
  density: TableDensity,
  children: Children,
) -> impl IntoView {
  let aria_label = aria_label.filter(|v| !v.is_empty());
  let region_label = aria_label.clone();
  let container = NodeRef::<leptos::html::Div>::new();
  // 溢出才可聚焦：检测与观察器的生命周期见 `use_scroll_overflow`。
  let scrollable = use_scroll_overflow(container);

  let wrapper_role = move || role.or(scrollable.get().then_some("region"));
  let wrapper_tabindex = move || tabindex.or(scrollable.get().then_some("0"));
  // `aria-label` 在无 role 的 `div` 上不被支持（axe: aria-prohibited-attr）。
  // 只有容器确有区域语义（溢出可滚，或调用方明确要 region）时才把名字挂上去，名字跟着 role 走；
  // 与 `ScrollRegion` 同一口径。不溢出时名字只留给内层 `<table>`。
  let wrapper_label =
    move || wrapper_role().and_then(|_| region_label.as_ref().map(TextValue::get));
  view! {
    <div
      node_ref=container
      class=cn(&["relative w-full overflow-x-auto", &wrapper_class])
      data-slot="table-container"
      tabindex=wrapper_tabindex
      role=wrapper_role
      aria-label=wrapper_label
    >
      <table
        data-slot="table"
        aria-label=move || aria_label.as_ref().map(TextValue::get)
        class=table_class(borderless, density, &class)
      >
        {children()}
      </table>
    </div>
  }
}
