use leptos::prelude::*;

use crate::icons::{Icon, IconKind};
use crate::ui::TextValue;

/// 空状态：居中提示，可附带说明与操作按钮（`actions`）。
///
/// 两种密度（同一个组件，不另起一套 DOM —— 同一个「没有内容」语义在全站只应有一种写法）：
/// - 默认：自己是一张卡片（`pxl-card` + 4px/12px 内边距 + 弹出场），用于**页面级**空态；
/// - `plain`：无边框无底色、上下留白更小，用于**卡片内**的空列表
///   （放进卡片里的空态再套一层卡片，会变成双线框）。
#[component]
pub fn EmptyState(
  #[prop(into)] title: TextValue,
  #[prop(optional, into)] description: Option<TextValue>,
  #[prop(optional)] icon: Option<IconKind>,
  #[prop(optional, into)] actions: Option<ViewFn>,
  #[prop(optional)] plain: bool,
) -> impl IntoView {
  // `motion-pop`：空态是「结果出来了，只是没有内容」，用回弹出场强调这是一次响应，
  // 而不是页面本身没加载完。卡片内的 `plain` 不播入场：它跟着父卡片一起出现，
  // 再弹一次会让整块内容「抖两下」。
  let root = if plain {
    "flex flex-col items-center justify-center gap-1 px-2 py-6 text-center"
  } else {
    "motion-pop flex flex-col items-center justify-center gap-2 pxl-card border bg-card px-4 py-12 text-center"
  };
  let title_class = if plain {
    "text-sm text-muted-foreground"
  } else {
    "text-sm text-foreground"
  };
  view! {
    <div class=root>
      {icon.map(|k| view! { <Icon kind=k class="size-12 text-muted-foreground" /> })}
      <div class=title_class>{move || title.get()}</div>
      {description.map(|d| view! { <div class="text-xs text-muted-foreground">{move || d.get()}</div> })}
      {actions.map(|a| a.run())}
    </div>
  }
}
