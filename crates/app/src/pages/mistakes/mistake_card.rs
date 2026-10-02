use crate::i18n::{t, tf};
use ham_web_core::Bank;
use ham_web_core::categories::{sub_category, top_pages};
use ham_web_core::mistake_book::{MistakeRecord, WRONG_CAUSES};
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
  let sub = q.p_code().and_then(sub_category);
  let topic = sub.map_or("未分类", |s| s.name);
  // 考点对应的一级分类专题页（取首个），用于点击直达补知识点。
  let topic_href = sub
    .and_then(|s| top_pages(s.top).first())
    .map(|&(href, _)| href);
  // 该一级分类的正确率（用于展示薄弱程度）。
  let topic_rate = sub.and_then(|s| {
    crate::study::load_stats()
      .total
      .categories
      .get(s.top)
      .and_then(|t| t.rate())
      .map(|r| (r * 100.0).round() as u32)
  });
  // 同类题再练：按二级分类（考点）筛选该题库下的同类题目。
  let same_href = sub.map(|s| format!("/browse?bank={}&sub={}", Bank::of_id(q.id_str()), s.name));
  let due = if record.is_due(now_ms) {
    t("待复习")
  } else {
    let days = ((record.due_ms - now_ms) as f64 / DAY_MS as f64).ceil() as i64;
    tf("{} 天后复习", &[&days.to_string()])
  };
  // 掌握度：难度系数越小，间隔增长越慢、越难掌握。
  let difficulty = if record.ease < 1.8 {
    (t("难"), "text-red-700 dark:text-red-400")
  } else if record.ease < 2.3 {
    (t("中"), "text-amber-700 dark:text-amber-400")
  } else {
    (t("易"), "text-emerald-700 dark:text-emerald-400")
  };
  let my_answer = if record.my_answer.is_empty() {
    t("（闪卡自评不会）")
  } else {
    record.my_answer.join("、")
  };
  let key = record.key.clone();
  let cause = RwSignal::new(record.cause.clone());
  let set_cause = {
    let key = key.clone();
    move |c: &'static str| {
      cause.set(if c.is_empty() {
        None
      } else {
        Some(c.to_owned())
      });
      crate::study::set_mistake_cause(&key, c);
    }
  };
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
          {move || t("我的答案：")} <span class="font-mono font-semibold">{my_answer}</span>
        </span>
        <span class="text-emerald-700 dark:text-emerald-400">
          {move || t("正确答案：")} <span class="font-mono font-semibold">{q.answer_keys.join("、")}</span>
        </span>
        <span class="text-muted-foreground">
          {move || t("考点：")}
          {match topic_href {
            Some(href) => view! {
              <a href=href class="font-medium text-foreground underline-offset-4 hover:underline">{move || t(topic)}</a>
            }
            .into_any(),
            None => view! { <span class="font-medium text-foreground">{move || t(topic)}</span> }.into_any(),
          }}
          {topic_rate.map(|r| view! { <span class="ml-1 tabular-nums">{tf("（正确率 {}%）", &[&r.to_string()])}</span> })}
          {same_href.map(|href| view! {
            <a href=href class="ml-1 font-medium text-primary underline-offset-4 hover:underline">
              {move || t("同类题再练 →")}
            </a>
          })}
        </span>
      </div>
      <div class="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
        <span>{move || t("答错")} <span class="font-semibold text-foreground tabular-nums">{record.wrong_count}</span> {move || t("次")}</span>
        <span>{tf("连续答对 {} / {}", &[&record.streak.to_string(), &record.target_streak().to_string()])}</span>
        <span>
          {move || t("难度")} <span class=format!("font-semibold {}", difficulty.1)>{difficulty.0}</span>
        </span>
        <span>{tf("间隔 {} 天", &[&format!("{:.0}", record.interval_days)])}</span>
        <span>{due}</span>
        <button
          type="button"
          class="ml-auto rounded px-2 py-0.5 transition-colors hover:bg-accent hover:text-foreground"
          on:click=move |_| on_remove.run(key.clone())
        >
          {move || t("移出")}
        </button>
      </div>
      <div class="mt-2 flex flex-wrap items-center gap-1.5 border-t pt-2 text-xs">
        <span class="text-muted-foreground">{move || t("错因")}</span>
        {WRONG_CAUSES
          .iter()
          .map(|&(ck, cn)| {
            let sc = set_cause.clone();
            view! {
              <button
                type="button"
                class=move || if cause.get().as_deref() == Some(ck) {
                  "rounded-full bg-primary/15 px-2.5 py-0.5 font-medium text-primary"
                } else {
                  "rounded-full border px-2.5 py-0.5 text-muted-foreground transition-colors hover:bg-accent"
                }
                aria-pressed=move || (cause.get().as_deref() == Some(ck)).to_string()
                on:click=move |_| {
                  let clear = cause.get_untracked().as_deref() == Some(ck);
                  sc(if clear { "" } else { ck });
                }
              >
                {move || t(cn)}
              </button>
            }
          })
          .collect_view()}
      </div>
    </div>
  }
}
