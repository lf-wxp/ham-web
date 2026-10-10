//! 易错知识点：错题归因图谱 —— 一级分类（域）× 知识点（二级分类）的热力图。
//!
//! 域来自 `categories::TOP_CATEGORIES`（与浏览页同一棵分类树），颜色越深错得越多；
//! 点知识点格子直达该知识点的专项练习（`/practice?sub=`），点域名练整个域（`/practice?topic=`）。

use ham_web_core::categories::top_category;
use ham_web_core::mistake_book::{heat_level, mistake_domains};
use leptos::prelude::*;

use crate::i18n::{t, tf, tp};
use crate::study;
use crate::util::set_title;

/// 热力档位 → 域色的透明度（8 位十六进制，拼在 `#rrggbb` 后面）。
fn heat_alpha(level: u8) -> &'static str {
  match level {
    0 => "00",
    1 => "1F",
    2 => "38",
    3 => "54",
    4 => "73",
    _ => "00",
  }
}

/// 域色（`#rrggbb`）→ 热力格子的背景色。
fn heat_bg(level: u8, color: &str) -> String {
  let color = color.trim_start_matches('#');
  format!("background-color:#{color}{}", heat_alpha(level))
}

#[component]
pub fn MistakeTopicsPage() -> impl IntoView {
  set_title("易错知识点");
  let book = study::load_book();
  let domains = mistake_domains(&book);
  let stats = study::load_stats();
  // 热力以「全局最热的那个知识点」为满档，域行的小条用「最热的那个域」为满档。
  let global_max = domains
    .iter()
    .flat_map(|d| d.topics.iter())
    .map(|t| t.mistakes)
    .max()
    .unwrap_or(0);
  let domain_max = domains.iter().map(|d| d.mistakes).max().unwrap_or(0);

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("exam.error-prone-topics")}</h1>
            <div class="text-xs text-muted-foreground">
              {move || t("exam.mistake-heatmap-subtitle")}
            </div>
          </div>
          <a href="/mistakes" class="text-xs text-primary underline-offset-4 hover:underline">
            {move || t("exam.all-mistakes")}
          </a>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-3 px-4 py-5">
        <p class="text-xs text-muted-foreground">
          {move || t("exam.mistake-heatmap-hint")}
        </p>

        {if domains.is_empty() {
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
          domains
            .into_iter()
            .map(|domain| {
              // 域行的热力条宽度（%）。
              let bar_pct = if domain_max == 0 {
                0
              } else {
                (domain.mistakes as f64 / domain_max as f64 * 100.0).round() as u32
              };
              let color = top_category(domain.top).map_or("#888888", |c| c.color);
              let name = top_category(domain.top).map_or(domain.top, |c| c.name);
              view! {
                <section class="rounded-xl border bg-card">
                  <a
                    href=format!("/practice?topic={}", domain.top)
                    class="flex flex-wrap items-center gap-3 px-4 py-3 transition-colors hover:bg-accent/50"
                  >
                    <span
                      class="size-2.5 shrink-0 rounded-full"
                      style=format!("background-color:{color}")
                      aria-hidden="true"
                    ></span>
                    <div class="mr-auto min-w-0">
                      <div class="text-sm font-medium">{name}</div>
                      <div class="mt-0.5 text-xs text-muted-foreground">
                        {tp("common.questions", domain.mistakes, &[&domain.mistakes.to_string()])}
                        " · "
                        {tp(
                          "common.wrong-answers-in-total",
                          domain.total_wrong,
                          &[&domain.total_wrong.to_string()],
                        )}
                      </div>
                      // 域行热力条：本域错题数与最热域的比。
                      <div class="mt-1 h-1.5 w-full max-w-56 overflow-hidden rounded-full bg-muted">
                        <div
                          class="h-full rounded-full"
                          style=format!("width:{bar_pct}%;background-color:{color}")
                        ></div>
                      </div>
                    </div>
                    <span class="shrink-0 text-xs text-primary">{move || t("exam.train-this-domain")}</span>
                  </a>

                  // 知识点热力格子：颜色深浅 = 该知识点错题数与全局最热的比。
                  <div class="flex flex-wrap gap-1.5 border-t px-4 py-3">
                    {domain.topics.into_iter().map(|topic| {
                      let level = heat_level(topic.mistakes, global_max);
                      let bg = heat_bg(level, color);
                      let rate = stats
                        .total
                        .subs
                        .get(topic.code)
                        .and_then(|s| s.rate())
                        .map(|r| (r * 100.0).round() as u32);
                      view! {
                        <a
                          href=format!("/practice?sub={}", topic.code)
                          title=format!(
                            "{} · {} · {}",
                            topic.name,
                            topic.code,
                            topic.mistakes
                          )
                          class="flex items-center gap-2 rounded-lg border px-2.5 py-1.5 text-xs transition-colors hover:border-primary/60"
                          style=bg
                        >
                          <span class="max-w-48 truncate font-medium">{topic.name}</span>
                          <span class="tabular-nums text-muted-foreground">{topic.mistakes}</span>
                          {rate.map(|r| view! {
                            <span class="tabular-nums text-muted-foreground">{tf("common.accuracy", &[&r.to_string()])}</span>
                          })}
                        </a>
                      }
                    }).collect_view()}
                  </div>
                </section>
              }
            })
            .collect_view()
            .into_any()
        }}
      </div>
    </div>
  }
}
