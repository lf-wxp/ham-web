use leptos::prelude::*;

use crate::cn::cn;

/// 对话框的内容区：矮视口下由这里滚动 —— `Dialog` 面板是 `max-h-[…] overflow-hidden`，
/// 内容不放进它就会被面板直接裁掉。
///
/// `-m-1.5 p-1.5`：滚动容器两个方向都裁剪，贴边控件的焦点框（2px 虚线 + 2–3px 偏移）会被切掉外沿；
/// 可视区往外让出 6px（落在面板自己的内边距里），内容位置不变。`class` 只放内容的排布。
#[component]
pub fn DialogBody(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="dialog-body"
      class=cn(&["-m-1.5 min-h-0 overflow-y-auto overscroll-contain p-1.5", &class])
    >
      {children()}
    </div>
  }
}
