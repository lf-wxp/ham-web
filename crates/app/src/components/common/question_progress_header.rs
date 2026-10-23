use leptos::prelude::*;

use crate::i18n::tf;
use crate::ui::Progress;

/// 进度头部：可选左侧控件、右侧按钮、进度条与说明。
#[component]
pub fn QuestionProgressHeader(
  #[prop(into)] percent: Signal<i64>,
  #[prop(into)] right: ViewFn,
  #[prop(optional, into)] meta: Option<ViewFn>,
  /// 进度文字的替代文案（如「已作答 1 / 683」）。不传时显示 `进度 N%`。
  ///
  /// 为什么要能换：683 题的题库里答 1 题是 0.146%，百分比会一直显示 0%；
  /// 用「已作答 N / 总数」这种绝对数才能看出每一步。
  #[prop(optional, into)]
  label: Option<Signal<String>>,
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
          <div class="min-w-24 text-sm tabular-nums text-muted-foreground">
            {move || {
              label
                .as_ref()
                .map_or_else(
                  || tf("learning.progress-percent", &[&percent.get().to_string()]),
                  |l| l.get(),
                )
            }}
          </div>
          <Progress value=percent class="h-2 flex-1 sm:w-40 sm:flex-none" />
        </div>
        {meta.map(|m| view! { <div class="hidden sm:block text-sm text-muted-foreground">{m.run()}</div> })}
      </div>
    </div>
  }
}
