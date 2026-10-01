use ham_web_core::categories::sub_category;
use ham_web_core::mistake_book::MistakeRecord;
use leptos::prelude::*;

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// 一道错题视图。
#[component]
pub(super) fn MistakeCard(
  record: MistakeRecord,
  now_ms: i64,
  on_remove: Callback<String>,
) -> impl IntoView {
  let q = &record.question;
  let j = q.j_code().map_or_else(|| "—".to_owned(), str::to_owned);
  let topic = q
    .p_code()
    .and_then(sub_category)
    .map_or("未分类", |s| s.name);
  let due = if record.is_due(now_ms) {
    "待复习".to_owned()
  } else {
    let days = ((record.due_ms - now_ms) as f64 / DAY_MS as f64).ceil() as i64;
    format!("{days} 天后复习")
  };
  let my_answer = if record.my_answer.is_empty() {
    "（闪卡自评不会）".to_owned()
  } else {
    record.my_answer.join("、")
  };
  let key = record.key.clone();
  view! {
    <div class="rounded-xl border bg-card p-4">
      <div class="mb-2 flex items-baseline gap-2">
        <span class="shrink-0 rounded bg-muted px-1.5 py-0.5 font-mono text-xs">{j}</span>
        <p class="whitespace-pre-line text-sm font-medium leading-snug">{q.question.clone()}</p>
      </div>
      <div class="space-y-1">
        {q
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
      <div class="mt-3 flex flex-wrap items-center gap-x-5 gap-y-1 text-xs">
        <span class="text-red-700 dark:text-red-400">
          "我的答案：" <span class="font-mono font-semibold">{my_answer}</span>
        </span>
        <span class="text-emerald-700 dark:text-emerald-400">
          "正确答案：" <span class="font-mono font-semibold">{q.answer_keys.join("、")}</span>
        </span>
        <span class="text-muted-foreground">
          "考点：" <span class="font-medium text-foreground">{topic}</span>
        </span>
      </div>
      <div class="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
        <span>"答错 " <span class="font-semibold text-foreground tabular-nums">{record.wrong_count}</span> " 次"</span>
        <span>{format!("连续答对 {} / {}", record.streak, record.target_streak())}</span>
        <span>{due}</span>
        <button
          type="button"
          class="ml-auto rounded px-2 py-0.5 transition-colors hover:bg-accent hover:text-foreground"
          on:click=move |_| on_remove.run(key.clone())
        >
          "移出"
        </button>
      </div>
    </div>
  }
}
