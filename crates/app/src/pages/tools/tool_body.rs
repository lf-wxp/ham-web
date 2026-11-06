use leptos::prelude::*;

/// 计算器主体：`ready` 之前先占位，之后挂上真正的内容（分批节奏见 `tool_mount`）。
///
/// 占位高度取 22rem：手机宽度下 60 个计算器主体的中位高度约 420px，桌面约 260px ——
/// 宁可略高，滚动条在挂载过程中只会往回缩一点，不会先短后长地跳。
#[component]
pub(super) fn ToolBody(ready: Memo<bool>, children: Children) -> impl IntoView {
  let mut children = Some(children);
  // 这个闭包只应该订阅 `ready`：`ready` 只会从 false 变成 true 一次，之后不再重跑，`children` 也就只取一次。
  // `children()` 构造计算器时可能读了别的信号（比如读存储里的上次输入）；放在 `untrack` 里，
  // 这些读取不会变成这个闭包的依赖。否则那个信号一变闭包就重跑，而 `children` 早已被取走，
  // 整个计算器就渲染成一片空白。
  move || {
    if ready.get() {
      untrack(|| children.take().map(|c| c())).into_any()
    } else {
      view! { <div aria-hidden="true" style="min-height: 22rem"></div> }.into_any()
    }
  }
}
