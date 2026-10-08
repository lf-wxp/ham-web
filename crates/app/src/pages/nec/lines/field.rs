//! `field`：从 `lines.rs` 拆出的视图构造函数（一个组件一个文件）。

use leptos::prelude::*;

use crate::ui::{ControlSize, Input};

use super::CELL;

/// 单元格：一个文本框（`ControlSize::Sm` 让密集表格里的控件与全站同高同圆角）。
pub(super) fn field(
  label: Signal<String>,
  value: Signal<String>,
  on_input: Callback<String, ()>,
  width: &'static str,
) -> impl IntoView {
  view! {
    <td class=CELL>
      <Input
        value=value
        on_change=on_input
        size=ControlSize::Sm
        aria_label=label
        class=width
      />
    </td>
  }
}
