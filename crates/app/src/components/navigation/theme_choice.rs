//! 设置对话框里的一个主题单选项：单选框 + 文字，点文字也能选中。

use leptos::prelude::*;

use crate::ui::RadioGroupItem;

/// 主题单选项：圆点 + 文字。
#[component]
pub(super) fn ThemeChoice(
  value: &'static str,
  #[prop(into)] label: Signal<String>,
) -> impl IntoView {
  let id = format!("settings-theme-{value}");
  let for_id = id.clone();
  view! {
    <label for=for_id class="flex cursor-pointer items-center gap-2 text-sm">
      <RadioGroupItem value=value id=id />
      {move || label.get()}
    </label>
  }
}
