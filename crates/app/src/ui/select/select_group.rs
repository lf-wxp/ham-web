use leptos::prelude::*;

/// 选项分组：一个不可选的组标题 + 一组 [`super::SelectItem`]。
///
/// `listbox` 的子节点只允许 `option` / `group`：组名挂在 `group` 的 `aria-label` 上读屏才会播报，
/// 标题本身是 `presentation`（不是一个裸文本行）。键盘导航读的是 DOM 里的 `[role="option"]` 顺序，
/// 分组只是多包了一层容器，不影响上下键的行号。
#[component]
pub fn SelectGroup(#[prop(into)] label: String, children: Children) -> impl IntoView {
  let heading = label.clone();
  // 现代风交给 `.md-option-group`（与 `.md-option` 的 12px 内距对齐）；像素风用工具类。
  let heading_class = if crate::kit::is_modern_at_mount() {
    "md-option-group"
  } else {
    "px-2 pt-2 pb-1 text-xs text-muted-foreground"
  };
  view! {
    <div role="group" aria-label=label>
      <div role="presentation" class=heading_class>
        {heading}
      </div>
      {children()}
    </div>
  }
}
