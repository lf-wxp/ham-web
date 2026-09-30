use ham_web_core::QuestionItem;
use leptos::prelude::*;

use crate::icons::{Icon, IconKind};

/// 一道收藏题卡片。
#[component]
pub(super) fn BookmarkCard(
  question: QuestionItem,
  on_remove: Callback<QuestionItem>,
) -> impl IntoView {
  let j = question
    .j_code()
    .map(str::to_owned)
    .unwrap_or_else(|| "—".to_owned());
  let q = question.clone();
  view! {
    <div class="rounded-xl border bg-card p-4">
      <div class="mb-2 flex items-start gap-2">
        <span class="mt-0.5 shrink-0 rounded bg-muted px-1.5 py-0.5 font-mono text-xs">{j}</span>
        <p class="flex-1 text-sm font-medium leading-snug">{question.question.clone()}</p>
        <button
          type="button"
          class="shrink-0 text-muted-foreground transition-colors hover:text-destructive"
          title="取消收藏"
          aria-label="取消收藏"
          on:click=move |_| on_remove.run(q.clone())
        >
          <Icon kind=IconKind::BookMarked class="h-4 w-4" />
        </button>
      </div>
      <div class="space-y-1">
        {question
          .options
          .iter()
          .map(|o| {
            view! {
              <div class="text-xs text-muted-foreground">
                <span class="font-mono">{o.key.clone()}</span> "　" {o.text.clone()}
              </div>
            }
          })
          .collect_view()}
      </div>
      <div class="mt-2 text-xs text-emerald-600 dark:text-emerald-400">
        "答案：" <span class="font-mono font-semibold">{question.answer_keys.join("、")}</span>
      </div>
    </div>
  }
}
