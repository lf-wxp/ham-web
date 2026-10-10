//! 法规原文库：条文原文 + 题目 ↔ 条款双向关联。
//!
//! 条文来自 `ham_web_core::radio_law`（数据文件里带出处与核对日期，**原文照录**）；
//! 「引用这条的题目」按人工维护的映射渲染（题干与知识点照抄题库，一致性由 app 侧
//! 单测逐字节比对钉住），反向链接到 `/practice?sub=<知识点>` 的专项练习。

use ham_web_core::radio_law::{MappedQuestion, article, chapters, laws, questions_for};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::{t, tf};
use crate::util::set_title;

/// 题干显示截断（悬停看全题）。
fn short_stem(stem: &str) -> String {
  if stem.chars().count() > 42 {
    let cut: String = stem.chars().take(42).collect();
    format!("{cut}…")
  } else {
    stem.to_owned()
  }
}

#[component]
pub fn RadioLawPage() -> impl IntoView {
  set_title("knowledge.radio-law");

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=Signal::derive(move || t("knowledge.radio-law"))
        subtitle=Signal::derive(move || t("knowledge.radio-law-subtitle"))
      />

      <PageContainer>
        <p class="text-xs text-muted-foreground">{move || t("knowledge.radio-law-note")}</p>

        {laws()
          .iter()
          .map(|law| {
            view! {
              <section class="rounded-xl border bg-card">
                <div class="flex flex-wrap items-baseline gap-2 border-b px-4 py-3">
                  <h2 class="text-sm font-semibold">{law.title}</h2>
                  <span class="text-xs text-muted-foreground">
                    {move || format!("{} · {} {}", law.order, t("knowledge.radio-law-effective"), law.effective)}
                  </span>
                  <a
                    href=law.source_url
                    target="_blank"
                    rel="noreferrer"
                    class="text-xs underline underline-offset-2 hover:text-foreground"
                  >
                    {move || t("knowledge.radio-law-source")}
                  </a>
                </div>

                {chapters(law.key)
                  .into_iter()
                  .map(|(chapter, numbers)| {
                    view! {
                      <div class="border-b px-4 py-3 last:border-b-0">
                        <h3 class="mb-2 text-xs font-semibold text-primary">{chapter}</h3>
                        <div class="space-y-1">
                          {numbers
                            .into_iter()
                            .map(|number| {
                              let text = article(law.key, number)
                                .expect("章内条号必有条文（核心有单测）")
                                .text
                                .clone();
                              let questions: Vec<MappedQuestion> = questions_for(law.key, number).to_vec();
                              let n_questions = questions.len();
                              view! {
                                <div
                                  // 锚点带法律 key：办法与条例都有「第30条」，不带会撞。
                                  id=format!("{}-{number}", law.key)
                                  class="scroll-mt-20 rounded-lg px-3 py-2 transition-colors hover:bg-muted/40"
                                >
                                  <div class="flex flex-wrap items-baseline gap-2">
                                    <span class="text-sm font-semibold">
                                      {move || tf("knowledge.radio-law-article-n", &[&number.to_string()])}
                                    </span>
                                    {(n_questions > 0)
                                      .then(|| {
                                        view! {
                                          <span class="text-[11px] text-muted-foreground">
                                            {move || {
                                              tf(
                                                "knowledge.radio-law-n-questions",
                                                &[&n_questions.to_string()],
                                              )
                                            }}
                                          </span>
                                        }
                                      })}
                                  </div>
                                  <p class="mt-1 whitespace-pre-line text-sm leading-6 text-muted-foreground">
                                    {text}
                                  </p>
                                  // 反向关联：题库里引用这条的题目（人工维护的映射）。
                                  {(n_questions > 0)
                                    .then(|| {
                                      view! {
                                        <div class="mt-2 border-t pt-2">
                                          <p class="text-[11px] text-muted-foreground">
                                            {move || t("knowledge.radio-law-citing-questions")}
                                          </p>
                                          <ul class="mt-1 space-y-1">
                                            {questions.into_iter().map(|q| {
                                              view! {
                                                <li class="flex flex-wrap items-baseline gap-2 text-xs">
                                                  <a
                                                    href=format!("/practice?sub={}", q.p_code)
                                                    title=q.stem
                                                    class="text-primary underline-offset-2 hover:underline"
                                                  >
                                                    <span class="font-mono text-muted-foreground">
                                                      {q.id}
                                                    </span>
                                                    " "
                                                    {short_stem(q.stem)}
                                                  </a>
                                                </li>
                                              }
                                            }).collect_view()}
                                          </ul>
                                        </div>
                                      }
                                    })}
                                </div>
                              }
                            })
                            .collect_view()}
                        </div>
                      </div>
                    }
                  })
                  .collect_view()}
              </section>
            }
          })
          .collect_view()}
      </PageContainer>
    </div>
  }
}
