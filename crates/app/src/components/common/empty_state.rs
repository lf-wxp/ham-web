use leptos::prelude::*;

use crate::icons::{Icon, IconKind};

/// 空状态：居中提示，可附带说明与操作按钮（`actions`）。
#[component]
pub fn EmptyState(
  #[prop(into)] title: String,
  #[prop(optional, into)] description: Option<String>,
  #[prop(optional)] icon: Option<IconKind>,
  #[prop(optional, into)] actions: Option<ViewFn>,
) -> impl IntoView {
  view! {
    // `motion-pop`：空态是「结果出来了，只是没有内容」，用回弹出场强调这是一次响应，
    // 而不是页面本身没加载完。
    <div class="motion-pop flex flex-col items-center justify-center gap-2 rounded-xl border bg-card px-4 py-12 text-center">
      {icon.map(|k| view! { <Icon kind=k class="h-8 w-8 text-muted-foreground/40" /> })}
      <div class="text-sm font-medium text-foreground">{title}</div>
      {description.map(|d| view! { <div class="text-xs text-muted-foreground">{d}</div> })}
      {actions.map(|a| a.run())}
    </div>
  }
}
