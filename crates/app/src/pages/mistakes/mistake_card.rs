use ham_web_core::categories::sub_category;
use ham_web_core::mistakes::Mistake;
use leptos::prelude::*;

/// 一道错题视图（只读）。
#[component]
pub(super) fn MistakeCard(mistake: Mistake) -> impl IntoView {
  let j = mistake
    .question
    .j_code()
    .map(str::to_owned)
    .unwrap_or_else(|| "—".to_owned());
  let topic = mistake
    .question
    .p_code()
    .and_then(sub_category)
    .map(|s| s.name)
    .unwrap_or("未分类");
  view! {
    <div class="rounded-xl border bg-card p-4">
      <div class="mb-2 flex items-baseline gap-2">
        <span class="shrink-0 rounded bg-muted px-1.5 py-0.5 font-mono text-xs">{j}</span>
        <p class="text-sm font-medium leading-snug">{mistake.question.question.clone()}</p>
      </div>
      <div class="space-y-1">
        {mistake
          .question
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
      <div class="mt-3 flex flex-wrap gap-x-5 gap-y-1 text-xs">
        <span class="text-red-600 dark:text-red-400">
          "我的答案："
          <span class="font-mono font-semibold">{mistake.my_answer.join("、")}</span>
        </span>
        <span class="text-emerald-600 dark:text-emerald-400">
          "正确答案："
          <span class="font-mono font-semibold">{mistake.correct.join("、")}</span>
        </span>
        <span class="text-muted-foreground">
          "考点："
          <span class="font-medium text-foreground">{topic}</span>
        </span>
      </div>
    </div>
  }
}
