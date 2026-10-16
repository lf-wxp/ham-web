//! [`Table`]：表格根（滚动容器 + `<table>`）。

use leptos::prelude::*;

use crate::cn::cn;
use crate::ui::TextValue;

use super::shared::table_class;

/// 表格根：外层 `overflow-x-auto` 滚动容器（移动端列宽保不住的兜底），内层 `<table>`。
///
/// - 列多了就在 `class` 里写 `min-w-[…]`（作用在 `<table>` 上，与滚动容器配套）；
///   列放得下就不写，滚动容器没有溢出时就是普通块。
/// - 没有可见标题的表格传 `aria_label`（读屏的表格名，e2e 也按它定位）。
/// - `borderless=true` 去掉单元格网格线（极简行列表，如训练记录）。
/// - 需要限制表格区域高度（如 `max-h-[28rem]`）时写在 `wrapper_class`（作用在滚动容器上；
///   `overflow-x-auto` 会让纵向溢出自动变为可滚动，无需再写 `overflow-auto`）。
/// - 宽表格想让键盘用户能滚动整个区域时，给滚动容器传 `tabindex="0"` + `role="region"`
///   （配合 `aria_label` 命名该区域）。
#[component]
pub fn Table(
  #[prop(optional, into)] aria_label: Option<TextValue>,
  #[prop(optional)] borderless: bool,
  #[prop(optional, into)] class: String,
  #[prop(optional, into)] wrapper_class: String,
  #[prop(optional)] tabindex: Option<&'static str>,
  #[prop(optional)] role: Option<&'static str>,
  children: Children,
) -> impl IntoView {
  let aria_label = aria_label.filter(|v| !v.is_empty());
  let region_label = aria_label.clone();
  view! {
    <div
      class=cn(&["relative w-full overflow-x-auto", &wrapper_class])
      data-slot="table-container"
      tabindex=tabindex
      role=role
      aria-label=move || region_label.as_ref().map(TextValue::get)
    >
      <table
        data-slot="table"
        aria-label=move || aria_label.as_ref().map(TextValue::get)
        class=table_class(borderless, &class)
      >
        {children()}
      </table>
    </div>
  }
}
