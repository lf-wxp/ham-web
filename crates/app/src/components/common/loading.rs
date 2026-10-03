use leptos::prelude::*;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};

/// 加载状态：spinner + 可选文案。默认大间距居中，可用 `class` 覆盖间距。
///
/// 短等待（切换筛选、提交表单）用 [`Loading`]；首屏整块内容还没到时改用 [`Skeleton`]，
/// 骨架屏把「大概会长什么样」先画出来，比一个孤零零的 spinner 更少引起布局跳动。
#[component]
pub fn Loading(
  #[prop(optional, into)] label: Option<String>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  view! {
    <div class=cn(&[
      "motion-pop flex items-center justify-center gap-2 py-20 text-sm text-muted-foreground",
      &class,
    ])>
      <Icon kind=IconKind::Loader2 class="h-5 w-5 animate-spin" />
      {label.map(|l| view! { <span>{l}</span> })}
    </div>
  }
}

/// 骨架占位块：流光扫过的灰色条，尺寸由 `class` 决定（`h-4 w-32` 这类）。
///
/// 用作 [`Loading`] 的替代：多行列表加载时铺几行即可，避免 spinner 消失瞬间的高度跳变。
#[component]
pub fn Skeleton(#[prop(optional, into)] class: String) -> impl IntoView {
  view! { <div aria-hidden="true" class=cn(&["skeleton", &class])></div> }
}
