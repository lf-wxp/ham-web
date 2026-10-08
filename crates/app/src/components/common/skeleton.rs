//! 骨架占位块：加载态的「轮廓先到位」版本。

use leptos::prelude::*;

use crate::cn::cn;

/// 骨架占位块：流光扫过的灰色条，尺寸由 `class` 决定（`h-4 w-32` 这类）。
///
/// 用作 [`crate::components::common::Loading`] 的替代：多行列表加载时铺几行即可，
/// 避免 spinner 消失瞬间的高度跳变。
#[component]
pub fn Skeleton(#[prop(optional, into)] class: String) -> impl IntoView {
  view! { <div aria-hidden="true" class=cn(&["skeleton", &class])></div> }
}
