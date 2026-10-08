//! 易错知识点：按知识点（二级分类）聚合错题，定位薄弱考点并一键同类题重练。

use ham_web_core::mistake_book::mistake_topics;
use leptos::prelude::*;

use crate::i18n::{t, tf, tp};
use crate::study;
use crate::util::set_title;

#[component]
pub fn MistakeTopicsPage() -> impl IntoView {
  set_title("易错知识点");
  let book = study::load_book();
  let topics = mistake_topics(&book);
  let stats = study::load_stats();

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("exam.error-prone-topics")}</h1>
            <div class="text-xs text-muted-foreground">
              {move || t("exam.mistakes-grouped-by-topic")}
            </div>
          </div>
          <a href="/mistakes" class="text-xs text-primary underline-offset-4 hover:underline">
            {move || t("exam.all-mistakes")}
          </a>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-3 px-4 py-5">
        {if topics.is_empty() {
          view! {
            <div class="rounded-xl border bg-card px-4 py-12 text-center">
              <div class="text-sm font-medium">{move || t("learning.no-mistakes-yet")}</div>
              <div class="mt-1 text-xs text-muted-foreground">
                {move || t("learning.after-answering-in-practice")}
              </div>
            </div>
          }
          .into_any()
        } else {
          topics
            .iter()
            .enumerate()
            .map(|(i, topic)| {
              let rate = stats
                .total
                .subs
                .get(topic.code)
                .and_then(|s| s.rate())
                .map(|r| (r * 100.0).round() as u32);
              view! {
                <a
                  href=format!("/browse?sub={}", topic.name)
                  class="flex items-center gap-3 rounded-xl border bg-card p-4 transition-colors hover:bg-accent/50"
                >
                  <span class="flex size-7 shrink-0 items-center justify-center rounded-full bg-primary/10 text-xs font-semibold tabular-nums text-primary">
                    {i + 1}
                  </span>
                  <div class="min-w-0 flex-1">
                    <div class="text-sm font-medium">{topic.name}</div>
                    <div class="mt-0.5 text-xs text-muted-foreground">
                      {topic.top_name} " · " <span class="font-mono">{topic.code}</span>
                    </div>
                  </div>
                  <div class="shrink-0 text-right text-xs">
                    <div class="font-semibold tabular-nums text-red-600">
                      {tp("common.questions", topic.mistakes, &[&topic.mistakes.to_string()])}
                    </div>
                    <div class="text-muted-foreground">
                      {tp("common.wrong-answers-in-total", topic.total_wrong, &[&topic.total_wrong.to_string()])}
                    </div>
                    {rate.map(|r| view! {
                      <div class="text-muted-foreground">
                        {tf("common.accuracy", &[&r.to_string()])}
                      </div>
                    })}
                  </div>
                  <span class="shrink-0 text-xs text-primary">{move || t("exam.similar-questions")}</span>
                </a>
              }
            })
            .collect_view()
            .into_any()
        }}
      </div>
    </div>
  }
}
