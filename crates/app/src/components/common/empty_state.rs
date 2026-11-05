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
  /// 「下一步」跳转按钮的文字（配 `action_href`）。只写出口、不用自定义视图时用它 ——
  /// 两套皮肤的按钮类名不同，让每个空态各写一遍必然漂移。
  #[prop(optional, into)]
  action_text: Option<TextValue>,
  #[prop(optional, into)] action_href: Option<String>,
  #[prop(optional)] plain: bool,
) -> impl IntoView {
  // `motion-pop`：空态是「结果出来了，只是没有内容」，用回弹出场强调这是一次响应，
  // 而不是页面本身没加载完。卡片内的 `plain` 不播入场：它跟着父卡片一起出现，
  // 再弹一次会让整块内容「抖两下」。
  let modern = crate::kit::is_modern_at_mount();
  let action = match (action_href, action_text) {
    (Some(href), Some(text)) => {
      let class = if modern {
        "md-btn md-btn-accent mt-1"
      } else {
        "pxl-btn pxl-btn-primary mt-1"
      };
      Some(view! { <a href=href class=class>{move || text.get()}</a> })
    }
    _ => None,
  };
  // 现代风：不画像素图标（那是一套 16×16 点阵字形），改用一圈虚线 + 淡色副标题。
  if modern {
    return view! {
      <div class=if plain { "md-empty-plain" } else { "md-empty-card" }>
        <span class="md-empty-mark" aria-hidden="true"></span>
        <div class="md-empty-title">{move || title.get()}</div>
        {description
          .map(|d| view! { <div class="md-muted md-sm">{move || d.get()}</div> })}
        {action}
        {actions.map(|a| a.run())}
      </div>
    }
    .into_any();
  }
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
      {action}
      {actions.map(|a| a.run())}
    </div>
  }
  .into_any()
}
