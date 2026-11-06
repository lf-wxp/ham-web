//! 设置对话框里的一行单选：单选圆点 + 可选前置内容 + 文字，点文字也能选中。

use leptos::prelude::*;

use crate::ui::RadioGroupItem;

/// 设置里的一行单选。
///
/// 界面风格、主题、配色方案三组选项的 DOM 结构与无障碍接线相同：DOM id 取
/// `{id_prefix}-{value}`，`<label for>` 指向它，点文字也能选中。id 规则与皮肤类名
/// （`check_row_class`）只在这里生成，调用方只给前缀、值和文字。
///
/// - `label`：选项文字。传 `Signal` 而不是 `String`，切语言时只有这一行重渲染。
/// - 子节点（可选）：放在文字前面的装饰，如配色方案的色块。随明暗模式变化的内容在子节点里
///   写成闭包（`{swatch}`），它自己就是响应式的。
#[component]
pub(super) fn ChoiceRow(
  /// id 前缀，例如 `settings-ui-style`。
  id_prefix: &'static str,
  /// 选项值（进 `RadioGroup` 的 `value`）。
  value: &'static str,
  #[prop(into)] label: Signal<String>,
  #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
  let id = format!("{id_prefix}-{value}");
  let for_id = id.clone();
  view! {
    <label for=for_id class=format!("{} text-sm", crate::kit::check_row_class())>
      <RadioGroupItem value=value id=id />
      {children.map(|leading| leading())}
      {move || label.get()}
    </label>
  }
}
