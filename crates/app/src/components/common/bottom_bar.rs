use leptos::prelude::*;

/// 固定在底部的操作栏（桌面与移动端分别布局）。
#[component]
pub fn BottomBar(
  #[prop(into)] stats: ViewFn,
  #[prop(into)] left: ViewFn,
  #[prop(into)] right: ViewFn,
  #[prop(into)] mobile_top: ViewFn,
  #[prop(optional, into)] mobile_bottom: Option<ViewFn>,
) -> impl IntoView {
  view! {
    <div
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
          {mobile_bottom.map(|m| view! { <div>{m.run()}</div> })}
        </div>
      </div>
    </div>
  }
}
