use ham_web_core::categories::{TOP_CATEGORIES, sub_category};
use ham_web_core::mistake_book::{
  MASTER_STREAK, MistakeRecord, RecordOutcome, WRONG_CAUSES, cause_stats, interval_buckets,
};
use ham_web_core::{Bank, QuestionItem};
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::cn::cn;
use crate::components::common::{ExplanationCard, NoteEditor, PageContainer, PageHeader};
use crate::study;
use crate::ui::{Button, ButtonLink, NativeSelect, SelectOption, Size, Variant};
use crate::util::{download_text, now_ms, set_title};

use super::mistake_card::MistakeCard;
use super::mistake_diagnosis::MistakeDiagnosis;
use crate::i18n::{t, tf, tp};

/// 题目所属一级分类（用于分类筛选），无分类码时归入「其他」。
pub(super) fn top_of(question: &QuestionItem) -> &'static str {
  question
    .p_code()
    .and_then(sub_category)
    .map_or("其他", |s| s.top)
}

/// 把错题导出为纯文本。
fn export_text(records: &[MistakeRecord]) -> String {
  let mut s = String::new();
  for (i, r) in records.iter().enumerate() {
    let j = r.question.j_code().unwrap_or("—");
    s.push_str(&format!("{}. [{}] {}\n", i + 1, j, r.question.question));
    for o in &r.question.options {
      s.push_str(&format!("   {}. {}\n", o.key, o.text));
    }
    s.push_str(&tp(
      "learning.correct-answer-wrong-times",
      r.wrong_count,
      &[
        &r.question.answer_keys.join("、"),
        &r.wrong_count.to_string(),
      ],
    ));
  }
  s
}

/// 选项选择态按钮。
fn option_class(selected: bool) -> String {
  cn(&[
    "w-full rounded-lg border px-3 py-2 text-left text-sm transition-colors",
    if selected {
      "border-primary bg-primary/10 text-foreground"
    } else {
      "hover:bg-accent"
    },
  ])
}

#[component]
pub fn MistakesPage() -> impl IntoView {
  set_title("shell.mistakes");

  let query = use_query_map();
  let bank = RwSignal::new(
    query
      .with_untracked(|q| q.get("bank"))
      .and_then(|b| b.parse::<Bank>().ok()),
  );
  let records = RwSignal::new(study::load_book().sorted());
  let refresh = move || records.set(study::load_book().sorted());
  let in_bank = move |m: &MistakeRecord| bank.get().is_none_or(|b| m.in_bank(b));
  let due_count = Memo::new(move |_| {
    let now = now_ms();
    records.with(|r| r.iter().filter(|m| m.is_due(now) && in_bank(m)).count())
  });
  let bank_count = Memo::new(move |_| records.with(|r| r.iter().filter(|m| in_bank(m)).count()));

  // 重练会话：开始时固定题目列表，作答结果实时写入错题本。
  let session = RwSignal::new(Vec::<MistakeRecord>::new());
  let practicing = RwSignal::new(false);
  let finished = RwSignal::new(false);
  let current = RwSignal::new(0usize);
  let selected = RwSignal::new(Vec::<String>::new());
  let feedback = RwSignal::new(None::<RecordOutcome>);
  let correct = RwSignal::new(0usize);
  let wrong = RwSignal::new(0usize);
  let mastered = RwSignal::new(0usize);
  let filter = RwSignal::new(String::new());
  let confirm_clear = RwSignal::new(false);
  // 闪卡模式：看题 → 自评会 / 不会，复用错题本的 SRS 进度。
  let carding = RwSignal::new(false);
  let card_revealed = RwSignal::new(false);
  let card_known = RwSignal::new(0usize);
  let card_unknown = RwSignal::new(0usize);

  let filtered = move || {
    let f = filter.get_untracked();
    let b = bank.get_untracked();
    records
      .get_untracked()
      .into_iter()
      .filter(|m| (f.is_empty() || top_of(&m.question) == f) && b.is_none_or(|b| m.in_bank(b)))
      .collect::<Vec<_>>()
  };

  let start_with = move |list: Vec<MistakeRecord>| {
    if list.is_empty() {
      return;
    }
    session.set(list);
    practicing.set(true);
    finished.set(false);
    current.set(0);
    selected.set(Vec::new());
    feedback.set(None);
    correct.set(0);
    wrong.set(0);
    mastered.set(0);
    crate::study::note_question_start();
  };
  let start_due = move || {
    let now = now_ms();
    let mut list: Vec<MistakeRecord> = filtered().into_iter().filter(|m| m.is_due(now)).collect();
    list.sort_by_key(|m| m.due_ms);
    start_with(list);
  };
  let start_all = move || start_with(filtered());

  let start_cards = move || {
    let list: Vec<MistakeRecord> = filtered();
    if list.is_empty() {
      return;
    }
    session.set(list);
    carding.set(true);
    current.set(0);
    card_revealed.set(false);
    card_known.set(0);
    card_unknown.set(0);
    crate::study::note_question_start();
  };

  let card_advance = move || {
    let total = session.with_untracked(Vec::len);
    if current.get_untracked() + 1 >= total {
      carding.set(false);
      refresh();
    } else {
      current.update(|c| *c += 1);
      card_revealed.set(false);
      crate::study::note_question_start();
    }
  };
  let card_mark = move |ok: bool| {
    let Some(m) = session.with_untracked(|s| s.get(current.get_untracked()).cloned()) else {
      return;
    };
    crate::study::record_self_assess(&m.question, ok);
    if ok {
      card_known.update(|c| *c += 1);
    } else {
      card_unknown.update(|c| *c += 1);
    }
    card_advance();
  };

  if query.with_untracked(|q| q.get("review").is_some()) {
    start_due();
  }

  let toggle = move |key: String| {
    let Some(m) = session.with_untracked(|s| s.get(current.get_untracked()).cloned()) else {
      return;
    };
    if m.question.is_multiple() {
      selected.update(|s| {
        if let Some(pos) = s.iter().position(|k| *k == key) {
          s.remove(pos);
        } else {
          s.push(key);
        }
      });
    } else {
      selected.set(vec![key]);
    }
  };

  let submit = move || {
    let Some(m) = session.with_untracked(|s| s.get(current.get_untracked()).cloned()) else {
      return;
    };
    let sel = selected.get_untracked();
    let Some(outcome) = study::record_answer(&m.question, &sel) else {
      return;
    };
    match outcome {
      RecordOutcome::Wrong => wrong.update(|c| *c += 1),
      RecordOutcome::Mastered => {
        correct.update(|c| *c += 1);
        mastered.update(|c| *c += 1);
      }
      RecordOutcome::Progressed { .. } | RecordOutcome::Correct => correct.update(|c| *c += 1),
    }
    feedback.set(Some(outcome));
  };

  let next = move || {
    let total = session.with_untracked(Vec::len);
    if current.get_untracked() + 1 >= total {
      finished.set(true);
      practicing.set(false);
      refresh();
    } else {
      current.update(|c| *c += 1);
      selected.set(Vec::new());
      feedback.set(None);
      crate::study::note_question_start();
    }
  };

  let exit = move || {
    practicing.set(false);
    finished.set(false);
    refresh();
  };

  let export = move || {
    download_text("mistakes.txt", &export_text(&filtered()), "text/plain");
  };

  let remove = Callback::new(move |key: String| {
    study::remove_mistake(&key);
    refresh();
  });

  let idle = move || !practicing.get() && !finished.get() && !records.with(Vec::is_empty);

  // 未来 7 天错题复习前瞻（owned 数据，静态渲染）。
  let review_now = now_ms();
  let review_timeline = study::load_book().due_timeline(review_now, 7);
  let review_max = review_timeline.iter().copied().max().unwrap_or(1).max(1);
  let review_total: usize = review_timeline.iter().sum();
  // 记忆巩固度分布（owned 数据，静态渲染）。
  let mastery = interval_buckets(&study::load_book());

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.mistakes")
        subtitle=tf("learning.questions-answered-wrong-in", &[&MASTER_STREAK.to_string()])
        actions=ViewFn::from(move || {
          view! {
            {move || {
              idle()
                .then(|| {
                  view! {
                    <Button
                      variant=Variant::Default
                      size=Size::Sm
                      disabled=Signal::derive(move || due_count.get() == 0)
                      on_click=Callback::new(move |_| start_due())
                    >
                      {move || tf("learning.due-today", &[&due_count.get().to_string()])}
                    </Button>
                    <Button
                      variant=Variant::Outline
                      size=Size::Sm
                      on_click=Callback::new(move |_| start_all())
                    >
                      {move || t("learning.retry-all")}
                    </Button>
                    <Button
                      variant=Variant::Outline
                      size=Size::Sm
                      on_click=Callback::new(move |_| start_cards())
                    >
                      {move || t("learning.flashcard-review")}
                    </Button>
                    <Button
                      variant=Variant::Outline
                      size=Size::Sm
                      on_click=Callback::new(move |_| export())
                    >
                      {move || t("learning.export")}
                    </Button>
                    <ButtonLink
                      href=Signal::derive(move || match bank.get() {
                        Some(b) => format!("/print?src=mistakes&bank={b}"),
                        None => "/print?src=mistakes".to_owned(),
                      })
                      variant=Variant::Outline
                      size=Size::Sm
                    >
                      {move || t("learning.print")}
                    </ButtonLink>
                    <Button
                      variant=Variant::Ghost
                      size=Size::Sm
                      class="text-muted-foreground"
                      on_click=Callback::new(move |_| confirm_clear.set(true))
                    >
                      {move || t("learning.clear")}
                    </Button>
                  }
                })
            }}
            <span class="rounded-full border px-3 py-1 text-xs text-muted-foreground">
              {move || match bank.get() {
                Some(b) => tp(
                  "learning.class-mistakes-total",
                  records.with(Vec::len) as u32,
                  &[
                    &b.to_string(),
                    &bank_count.get().to_string(),
                    &records.with(Vec::len).to_string(),
                  ],
                ),
                None => tp("learning.mistakes-total", records.with(Vec::len), &[&records.with(Vec::len).to_string()]),
              }}
            </span>
          }
        })
      >
        {move || {
          confirm_clear.get().then(|| {
            view! {
              <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-2 px-4 pb-3 text-sm">
                <span class="mr-auto text-red-700 dark:text-red-400">{move || t("learning.clear-all-mistakes-this")}</span>
                <Button
                  variant=Variant::Destructive
                  size=Size::Sm
                  on_click=Callback::new(move |_| {
                                    study::clear_book();
                                    confirm_clear.set(false);
                                    refresh();
                                  })
                >
                  {move || t("learning.clear")}
                </Button>
                <Button
                  variant=Variant::Outline
                  size=Size::Sm
                  on_click=Callback::new(move |_| confirm_clear.set(false))
                >
                  {move || t("exam.cancel")}
                </Button>
              </div>
            }
          })
        }}
      </PageHeader>

      <PageContainer class="space-y-4">
        {move || {
          if carding.get() {
            // 闪卡视图：看题 → 显示答案 → 自评会 / 不会
            let Some(m) = session.with_untracked(|s| s.get(current.get()).cloned()) else {
              return view! { <div></div> }.into_any();
            };
            let total = session.with_untracked(Vec::len);
            let q = m.question.clone();
            view! {
              <div class="rounded-xl border bg-card p-5">
                <div class="mb-3 flex items-center justify-between text-xs text-muted-foreground">
                  <span>{move || tf("exam.question", &[&(current.get() + 1).to_string(), &total.to_string()])}</span>
                  <span>
                    {move || t("learning.know")} <span class="font-semibold text-emerald-600">{card_known.get()}</span>
                    " · " {move || t("learning.don-t-know")} <span class="font-semibold text-red-600">{card_unknown.get()}</span>
                  </span>
                </div>
                <p class="whitespace-pre-line text-base font-medium leading-relaxed">{q.question.clone()}</p>
                {q.image().map(|src| view! { <img src=src.to_owned() alt=move || t("exam.question-image") class="max-h-64 rounded border" /> })}
                {move || {
                  if card_revealed.get() {
                    view! {
                      <div class="mt-4 space-y-1 border-t pt-3">
                        {q.options.iter().map(|o| {
                          let correct = q.answer_keys.contains(&o.key);
                          view! {
                            <div class=format!(
                              "text-sm {}",
                              if correct { "font-medium text-emerald-600 dark:text-emerald-400" } else { "text-muted-foreground" },
                            )>
                              <span class="font-mono">{o.key.clone()}</span> "　" {o.text.clone()}
                            </div>
                          }
                        }).collect_view()}
                      </div>
                    }
                    .into_any()
                  } else {
                    view! { <div class="mt-4 text-xs text-muted-foreground">{move || t("learning.think-of-the-answer")}</div> }
                      .into_any()
                  }
                }}
                <div class="mt-5 flex items-center gap-2">
                  {move || {
                    if card_revealed.get() {
                      view! {
                        <Button
                          variant=Variant::Default
                          size=Size::Default
                          on_click=Callback::new(move |_| card_mark(true))
                        >
                          {move || t("learning.know-it")}
                        </Button>
                        <Button
                          variant=Variant::Outline
                          size=Size::Default
                          on_click=Callback::new(move |_| card_mark(false))
                        >
                          {move || t("learning.don-t-know-2")}
                        </Button>
                      }
                      .into_any()
                    } else {
                      view! {
                        <Button
                          variant=Variant::Default
                          size=Size::Default
                          on_click=Callback::new(move |_| card_revealed.set(true))
                        >
                          {move || t("learning.show-answer")}
                        </Button>
                        <Button
                          variant=Variant::Ghost
                          size=Size::Default
                          class="ml-auto text-muted-foreground"
                          on_click=Callback::new(move |_| {
                                                    carding.set(false);
                                                    refresh();
                                                  })
                        >
                          {move || t("learning.end")}
                        </Button>
                      }
                      .into_any()
                    }
                  }}
                </div>
              </div>
            }
            .into_any()
          } else if practicing.get() {
            // 重练视图
            let Some(m) = session.with_untracked(|s| s.get(current.get()).cloned()) else {
              return view! { <div></div> }.into_any();
            };
            let total = session.with_untracked(Vec::len);
            let answer = m.question.answer_keys.join("、");
            let expl = m.question.clone();
            let mk = m.key.clone();
            let wrong_cause = RwSignal::new(None::<String>);
            // 笔记以 stable_id 为键，与练习 / 收藏页共用同一份数据。
            let m_note = m.question.clone();
            let note_id = Signal::derive(move || m_note.stable_id().unwrap_or_default());
            view! {
              <div class="space-y-4 rounded-xl border bg-card p-5">
                <div class="flex items-center justify-between text-xs text-muted-foreground">
                  <span>{move || tf("exam.question", &[&(current.get() + 1).to_string(), &total.to_string()])}</span>
                  <span>
                    {move || t("exam.correct-3")} <span class="font-semibold text-emerald-600">{correct.get()}</span>
                    "　" {move || t("exam.wrong-2")} <span class="font-semibold text-red-600">{wrong.get()}</span>
                  </span>
                </div>
                <p class="whitespace-pre-line text-sm font-medium leading-snug">{m.question.question.clone()}</p>
                {m.question.image().map(|src| view! { <img src=src.to_owned() alt=move || t("exam.question-image") class="max-h-64 rounded border" /> })}
                <div class="space-y-2">
                  {m.question.options.iter().map(|o| {
                    let key = o.key.clone();
                    let class_key = key.clone();
                    let pressed_key = key.clone();
                    let text = o.text.clone();
                    view! {
                      <button
                        type="button"
                        aria-pressed=move || selected.get().contains(&pressed_key).to_string()
                        disabled=move || feedback.get().is_some()
                        on:click=move |_| toggle(key.clone())
                        class=move || option_class(selected.get().contains(&class_key))
                      >
                        <span class="font-mono">{o.key.clone()}</span> "　" {text}
                      </button>
                    }
                  }).collect_view()}
                </div>
                <div class="flex items-center gap-2">
                  <Button
                    variant=Variant::Default
                    size=Size::Default
                    disabled=Signal::derive(move || feedback.get().is_some() || selected.get().is_empty())
                    on_click=Callback::new(move |_| submit())
                  >
                    {move || t("learning.submit")}
                  </Button>
                  <Button
                    variant=Variant::Outline
                    size=Size::Default
                    disabled=Signal::derive(move || feedback.get().is_none())
                    on_click=Callback::new(move |_| next())
                  >
                    {move || t("exam.next")}
                  </Button>
                  <Button
                    variant=Variant::Ghost
                    size=Size::Default
                    class="ml-auto text-muted-foreground"
                    on_click=Callback::new(move |_| exit())
                  >
                    {move || t("learning.end")}
                  </Button>
                </div>
                <div class="min-h-5 text-sm">
                  {move || {
                    feedback.get().map(|outcome| match outcome {
                      RecordOutcome::Wrong => view! {
                        <div class="space-y-2">
                          <span class="font-medium text-red-700 dark:text-red-400">
                            {move || t("exam.correct-answer-2")} <span class="font-mono font-semibold">{answer.clone()}</span>
                            <span class="ml-2 text-xs font-normal text-muted-foreground">{move || t("learning.the-correct-in-a")}</span>
                          </span>
                          <div class="flex flex-wrap items-center gap-1.5">
                            <span class="text-xs text-muted-foreground">{move || t("exam.why-did-you-get")}</span>
                            {WRONG_CAUSES
                              .iter()
                              .map(|&(ck, cn)| {
                                let mk2 = mk.clone();
                                view! {
                                  <button
                                    type="button"
                                    class=move || if wrong_cause.get().as_deref() == Some(ck) {
                                      "rounded-full bg-primary/15 px-2.5 py-0.5 text-xs font-medium text-primary"
                                    } else {
                                      "rounded-full border px-2.5 py-0.5 text-xs text-muted-foreground transition-colors hover:bg-accent"
                                    }
                                    on:click=move |_| {
                                      wrong_cause.set(Some(ck.to_owned()));
                                      crate::study::set_mistake_cause(&mk2, ck);
                                    }
                                  >
                                    {move || t(cn)}
                                  </button>
                                }
                              })
                              .collect_view()}
                          </div>
                        </div>
                      }.into_any(),
                      RecordOutcome::Mastered => view! {
                        <span class="font-medium text-emerald-700 dark:text-emerald-400">{move || t("learning.correct-mastered-removed-from")}</span>
                      }.into_any(),
                      RecordOutcome::Progressed { streak, target, next_days } => view! {
                        <span class="font-medium text-emerald-700 dark:text-emerald-400">
                          {tp("learning.correct-in-a-row", u32::from(next_days), &[&streak.to_string(), &target.to_string(), &next_days.to_string()])}
                        </span>
                      }.into_any(),
                      RecordOutcome::Correct => view! {
                        <span class="font-medium text-emerald-700 dark:text-emerald-400">{move || t("learning.correct")}</span>
                      }.into_any(),
                    })
                  }}
                </div>
                {move || feedback.get().is_some().then(|| view! { <ExplanationCard question=expl.clone() /> })}
                // 答完看到解析后正好是记笔记的时机：把易错点写下来，下次复习时一起出现。
                {move || {
                  feedback.get().is_some().then(|| {
                    view! { <NoteEditor question_id=note_id /> }
                  })
                }}
              </div>
            }
            .into_any()
          } else if finished.get() {
            view! {
              <div class="rounded-xl border bg-card px-4 py-10 text-center">
                <div class="text-lg font-semibold">{move || t("learning.retry-complete")}</div>
                <div class="mt-2 text-sm text-muted-foreground">
                  {move || t("exam.correct-3")} " " <span class="font-semibold text-emerald-600">{correct.get()}</span>
                  "　" {move || t("exam.wrong-2")} " " <span class="font-semibold text-red-600">{wrong.get()}</span>
                  "　" {move || t("learning.mastered")} " " <span class="font-semibold text-foreground">{mastered.get()}</span>
                </div>
                <Button
                  variant=Variant::Default
                  size=Size::Default
                  class="mt-5"
                  on_click=Callback::new(move |_| exit())
                >
                  {move || t("learning.back-to-list")}
                </Button>
              </div>
            }
            .into_any()
          } else if records.with(Vec::is_empty) {
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
            view! {
              <MistakeDiagnosis />
              {{
                let timeline = review_timeline.clone();
                let max = review_max;
                move || {
                  (review_total > 0).then(|| {
                    view! {
                      <div class="rounded-xl border bg-card p-4">
                        <div class="mb-2 flex items-center justify-between">
                          <h3 class="text-sm font-semibold">{move || t("exam.review-forecast")}</h3>
                          <span class="text-xs text-muted-foreground">
                            {tp("exam.questions-due-for-review", review_total, &[&review_total.to_string()])}
                          </span>
                        </div>
                        <div class="flex h-24 items-end gap-1.5">
                          {timeline
                            .iter()
                            .enumerate()
                            .map(|(i, n)| {
                              let h = if *n == 0 {
                                0.0
                              } else {
                                (*n as f64 / max as f64 * 100.0).max(8.0)
                              };
                              let label = match i {
                                0 => t("exam.today"),
                                1 => t("exam.tomorrow"),
                                2 => t("exam.day-after"),
                                _ => tp("exam.in-days", i, &[&i.to_string()]),
                              };
                              let bar_class = if *n > 0 {
                                "w-full rounded-t bg-primary/70"
                              } else {
                                "w-full rounded-t bg-muted"
                              };
                              view! {
                                <div class="flex min-w-0 flex-1 flex-col items-center gap-1">
                                  <div
                                    class=bar_class
                                    style=format!("height: {h}%")
                                    title=tp("exam.questions-2", *n, &[&label, &n.to_string()])
                                  ></div>
                                  <span class="text-[9px] text-muted-foreground">{label}</span>
                                </div>
                              }
                            })
                            .collect_view()}
                        </div>
                      </div>
                    }
                  })
                }
              }}
              {{
                move || {
                  let total = mastery.total();
                  (total > 0).then(|| {
                    let pct = |n: usize| -> String {
                      format!("{:.2}%", n as f64 / total as f64 * 100.0)
                    };
                    let dot = |color: &'static str| {
                      view! { <span class=format!("inline-block h-2 w-2 rounded-full {color}")></span> }
                    };
                    view! {
                      <div class="rounded-xl border bg-card p-4">
                        <div class="mb-2 flex items-center justify-between">
                          <h3 class="text-sm font-semibold">{move || t("learning.memory-consolidation")}</h3>
                          <span class="text-xs text-muted-foreground">
                            {move || t("learning.correct-answers-stretch-the")}
                          </span>
                        </div>
                        <div class="flex h-3 w-full overflow-hidden rounded-full bg-muted">
                          <div class="bg-red-400/60" style=format!("width: {}", pct(mastery.learning))></div>
                          <div class="bg-amber-400/70" style=format!("width: {}", pct(mastery.short))></div>
                          <div class="bg-sky-400/70" style=format!("width: {}", pct(mastery.medium))></div>
                          <div class="bg-emerald-500/70" style=format!("width: {}", pct(mastery.mature))></div>
                        </div>
                        <div class="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
                          <span class="inline-flex items-center gap-1">
                            {dot("bg-red-400/60")}
                            {tf("learning.learning", &[&mastery.learning.to_string()])}
                          </span>
                          <span class="inline-flex items-center gap-1">
                            {dot("bg-amber-400/70")}
                            {tf("learning.1-6-days", &[&mastery.short.to_string()])}
                          </span>
                          <span class="inline-flex items-center gap-1">
                            {dot("bg-sky-400/70")}
                            {tf("learning.7-20-days", &[&mastery.medium.to_string()])}
                          </span>
                          <span class="inline-flex items-center gap-1">
                            {dot("bg-emerald-500/70")}
                            {tf("learning.21-days", &[&mastery.mature.to_string()])}
                          </span>
                        </div>
                      </div>
                    }
                  })
                }
              }}
              {move || {
                if !filter.get().is_empty() {
                  return view! { <div></div> }.into_any();
                }
                let mut counts: Vec<(&'static str, &'static str, usize)> = records.with(|all| {
                  TOP_CATEGORIES
                    .iter()
                    .map(|c| (c.key, c.name, all.iter().filter(|m| in_bank(m) && top_of(&m.question) == c.key).count()))
                    .filter(|(_, _, n)| *n > 0)
                    .collect()
                });
                counts.sort_by_key(|(_, _, n)| std::cmp::Reverse(*n));
                if counts.is_empty() {
                  return view! { <div></div> }.into_any();
                }
                view! {
                  <div class="rounded-xl border bg-card p-4">
                    <h3 class="mb-2 text-sm font-semibold">{move || t("learning.weak-categories")}</h3>
                    <div class="flex flex-wrap gap-1.5">
                      {counts
                        .into_iter()
                        .map(|(key, name, n)| {
                          view! {
                            <button
                              type="button"
                              class="whitespace-nowrap rounded-full border px-3 py-1 text-xs transition-colors hover:bg-accent"
                              title=tf("learning.show-only-mistakes", &[(name)])
                              on:click=move |_| filter.set(key.to_owned())
                            >
                              {name} " " <span class="font-semibold tabular-nums">{n}</span>
                            </button>
                          }
                        })
                        .collect_view()}
                    </div>
                  </div>
                }
                .into_any()
              }}

              {{
                let cause_counts = cause_stats(&study::load_book());
                let has_cause = cause_counts.iter().any(|(_, _, n)| *n > 0);
                move || {
                  has_cause.then(|| {
                    view! {
                      <div class="rounded-xl border bg-card p-4">
                        <h3 class="mb-2 text-sm font-semibold">{move || t("exam.mistake-causes")}</h3>
                        <div class="flex flex-wrap gap-1.5">
                          {cause_counts
                            .iter()
                            .filter(|(_, _, n)| *n > 0)
                            .map(|&(_, name, n)| {
                              view! {
                                <span class="whitespace-nowrap rounded-full border px-3 py-1 text-xs">
                                  {move || t(name)} " " <span class="font-semibold tabular-nums">{n}</span>
                                </span>
                              }
                            })
                            .collect_view()}
                        </div>
                      </div>
                    }
                  })
                }
              }}

              <div class="flex flex-wrap items-center gap-2">
                {{
                  let category_options: Vec<SelectOption> = TOP_CATEGORIES
                    .iter()
                    .map(|c| SelectOption::new(c.key, c.name))
                    .collect();
                  view! {
                    <NativeSelect
                      value=filter
                      on_change=Callback::new(move |v: String| filter.set(v))
                      options=category_options
                      placeholder=Signal::derive(move || t("learning.all-categories"))
                      aria_label=Signal::derive(move || t("learning.all-categories"))
                      class="w-full sm:w-48"
                    />
                  }
                }}
                <div class="flex overflow-hidden rounded-lg border text-xs">
                  {std::iter::once(None)
                    .chain(Bank::ALL.into_iter().map(Some))
                    .map(|b| view! {
                      <button
                        type="button"
                        class=move || cn(&[
                          "px-3 py-1.5 font-medium transition-colors",
                          if bank.get() == b { "bg-primary text-primary-foreground" } else { "hover:bg-accent" },
                        ])
                        aria-pressed=move || (bank.get() == b).to_string()
                        on:click=move |_| bank.set(b)
                      >
                        {b.map_or_else(|| t("exam.all"), |b| tf("learning.class", &[&b.to_string()]))}
                      </button>
                    })
                    .collect_view()}
                </div>
              </div>
              {move || {
                let f = filter.get();
                let now = now_ms();
                records
                  .get()
                  .into_iter()
                  .filter(|m| (f.is_empty() || top_of(&m.question) == f.as_str()) && in_bank(m))
                  .map(|m| view! { <MistakeCard record=m now_ms=now on_remove=remove /> })
                  .collect_view()
              }}
            }
            .into_any()
          }
        }}
      </PageContainer>
    </div>
  }
}
