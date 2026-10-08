use leptos::prelude::*;

use crate::i18n::tf;
use crate::ui::Progress;

/// 进度头部：可选左侧控件、右侧按钮、进度条与说明。
#[component]
pub fn QuestionProgressHeader(
  #[prop(into)] percent: Signal<i64>,
  #[prop(into)] right: ViewFn,
  #[prop(optional, into)] meta: Option<ViewFn>,
  #[prop(optional, into)] left: Option<ViewFn>,
) -> impl IntoView {
  let top_row = if left.is_some() {
    "flex flex-wrap items-center justify-between gap-x-4 gap-y-2"
  } else {
    "flex flex-wrap items-center justify-end gap-x-4 gap-y-2"
  };
  view! {
    <div class="space-y-2">
      <div class=top_row>
        {left.as_ref().map(|l| view! { <div class="flex flex-wrap items-center gap-2">{l.run()}</div> })}
        <div class="flex items-center gap-3">{right.run()}</div>
      </div>
      <div class="flex items-center gap-4 justify-between flex-wrap">
        <div class="flex items-center gap-4 w-full sm:w-auto">
          <div class="min-w-24 text-sm text-muted-foreground">
            {move || tf("learning.progress-2", &[&percent.get().to_string()])}
          </div>
          <Progress value=percent class="h-2 flex-1 sm:w-40 sm:flex-none" />
        </div>
        {meta.map(|m| view! { <div class="hidden sm:block text-sm text-muted-foreground">{m.run()}</div> })}
      </div>
    </div>
  }
}
