use leptos::portal::Portal;
use leptos::prelude::*;

use crate::i18n::t;

/// 固定在底部的操作栏（桌面与移动端分别布局）。
///
/// 经 `Portal` 挂到 `body`，而不是留在页面里：路由内容容器 `#route-content` 切换时
/// 会重放 `motion-page` 入场动画（带 `transform`），而**祖先只要有 transform，其后代
/// 的 `position: fixed` 就会以该容器（而非视口）为包含块** —— 操作栏会跟着内容走、
/// 不再贴底（渲染在题目卡片下方，下方留一大片空白）。弹窗同样用 Portal 规避这一点。
///
/// 挂到 `body` 后不在 `main` 里了，无障碍检查会报「内容不在地标内」（axe `region`），
/// 因此显式声明为带名称的 `region` 地标。
#[component]
pub fn BottomBar(
  #[prop(into)] stats: ViewFn,
  #[prop(into)] left: ViewFn,
  #[prop(into)] right: ViewFn,
  #[prop(into)] mobile_top: ViewFn,
  #[prop(optional, into)] mobile_bottom: Option<ViewFn>,
) -> impl IntoView {
  view! {
    <Portal>
      <div
        role="region"
        aria-label=move || t("操作栏")
        class="fixed left-0 right-0 bottom-0 z-40 border-t bg-background/85 backdrop-blur supports-[backdrop-filter]:bg-background/60"
        style="padding-bottom: env(safe-area-inset-bottom);"
      >
        <div class="container mx-auto max-w-4xl px-4 py-2 space-y-2">
          <div class="text-sm text-muted-foreground text-center">{stats.run()}</div>
          <div class="hidden sm:flex items-center justify-between gap-2">
            <div>{left.run()}</div>
            <div class="flex items-center gap-2"></div>
            <div class="flex items-center gap-2">{right.run()}</div>
          </div>
          <div class="sm:hidden space-y-2">
            <div>{mobile_top.run()}</div>
            {mobile_bottom.as_ref().map(|m| view! { <div>{m.run()}</div> })}
          </div>
        </div>
      </div>
    </Portal>
  }
}
