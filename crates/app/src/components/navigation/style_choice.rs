//! 设置对话框里的一个界面风格单选项：单选框 + 文字，点文字也能选中。
//!
//! 与 `ThemeChoice` 分开是因为两者的 `id` 前缀与语义不同（主题是明暗，风格是整套设计系统）；
//! 合并成一个组件会让 id 生成规则与可访问名混在一起。

use leptos::prelude::*;

use crate::ui::RadioGroupItem;

#[component]
pub(super) fn StyleChoice(
  value: &'static str,
  #[prop(into)] label: Signal<String>,
) -> impl IntoView {
  let id = format!("settings-ui-style-{value}");
  let for_id = id.clone();
  view! {
    <label for=for_id class=format!("{} text-sm", crate::kit::check_row_class())>
      <RadioGroupItem value=value id=id />
      {move || label.get()}
    </label>
  }
}
