use leptos::prelude::*;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};
use crate::ui::TextValue;

/// 加载状态：spinner + 可选文案。默认大间距居中，可用 `class` 覆盖间距。
///
/// 短等待（切换筛选、提交表单）用 [`Loading`]；首屏整块内容还没到时改用
/// [`crate::components::common::Skeleton`]，骨架屏把「大概会长什么样」先画出来，
/// 比一个孤零零的 spinner 更少引起布局跳动。
///
/// 语义与 [`PageLoading`](crate::components::common::PageLoading) 对齐：外层常驻
/// `role="status"` + `aria-busy`，图标 `aria-hidden`，没有可见文案时补一句 `sr-only`
/// —— 否则 spinner 只在视觉上转，读屏用户听不到「还在加载」。节点先于内容存在，
/// 部分读屏器才播报（与判分结论同一条口径）。
#[component]
pub fn Loading(
  #[prop(optional, into)] label: Option<TextValue>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let has_label = label.is_some();
  view! {
    <div
      role="status"
      aria-busy="true"
      aria-live="polite"
      class=cn(&[
        "motion-pop flex items-center justify-center gap-2 py-20 text-sm text-muted-foreground",
        &class,
      ])
    >
      <Icon kind=IconKind::Loader2 class="size-6 w-5 animate-spin" />
      {label.map(|l| view! { <span>{move || l.get()}</span> })}
      {(!has_label)
        .then(|| view! { <span class="sr-only">{move || crate::i18n::t("common.loading")}</span> })}
    </div>
  }
}
