use leptos::prelude::*;

use crate::cn::cn;

/// 对话框的内容区：矮视口下由这里滚动 —— `Dialog` 面板是 `max-h-[…] overflow-hidden`，
/// 内容不放进它就会被面板直接裁掉。
///
/// `-m-1.5 p-1.5`：滚动容器两个方向都裁剪，贴边控件的焦点框（2px 虚线 + 2px 偏移 = 盒外 4px）
/// 会被切掉外沿；可视区往外让出 6px（落在面板自己的内边距里），内容位置不变。
/// **父级必须自带 ≥6px 内距**（`Dialog` 是 `p-4` / `p-6`），否则负外边距会越出面板。
///
/// 这条「让 6px」的规则是全站唯一落点：弹层以外的滚动区（抽屉 / 下拉面板）父级内距不同，
/// 要按各自的内距换成 `-m-(内距-目标)` 的组合，不要套用本组件。
#[component]
pub fn DialogBody(
  #[prop(optional, into)] class: String,
  /// 内容溢出时需要键盘滚动：置真会补 `tabindex="0"`（axe 的 `scrollable-region-focusable`）。
  #[prop(optional, default = false)]
  focusable: bool,
  children: Children,
) -> impl IntoView {
  view! {
    <div
      data-slot="dialog-body"
      tabindex=move || focusable.then_some("0")
      class=cn(&["-m-1.5 min-h-0 overflow-y-auto overscroll-contain p-1.5", &class])
    >
      {children()}
    </div>
  }
}
