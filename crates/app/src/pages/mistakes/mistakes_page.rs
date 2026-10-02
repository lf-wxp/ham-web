use ham_web_core::categories::{TOP_CATEGORIES, sub_category};
use ham_web_core::mistake_book::{
  MASTER_STREAK, MistakeRecord, RecordOutcome, WRONG_CAUSES, cause_stats,
};
use ham_web_core::{Bank, QuestionItem};
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::cn::cn;
use crate::components::common::{ExplanationCard, NoteEditor};
use crate::study;
use crate::ui::{Size, Variant, button_class, input_class};
use crate::util::{download_text, now_ms, set_title};

use super::mistake_card::MistakeCard;
use super::mistake_diagnosis::MistakeDiagnosis;
use crate::i18n::{t, tf};

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
    s.push_str(&tf(
      "   正确答案：{}（累计答错 {} 次）\n\n",
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
  set_title(&t("错题集"));

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

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-3xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("错题集")}</h1>
            <div class="text-xs text-muted-foreground">
              {tf("练习 · 模拟考试 · 闪卡中答错的题；复习间隔随作答自适应，连续答对 {} 次（常错题更多）自动移出", &[&MASTER_STREAK.to_string()])}
            </div>
          </div>
          {move || {
            idle()
              .then(|| {
                view! {
                  <button
                    type="button"
                    class=button_class(Variant::Default, Size::Sm, "")
                    disabled=move || due_count.get() == 0
                    on:click=move |_| start_due()
                  >
                    {move || tf("今日待复习 {}", &[&due_count.get().to_string()])}
                  </button>
                  <button type="button" class=button_class(Variant::Outline, Size::Sm, "") on:click=move |_| start_all()>
                    {move || t("全部重练")}
                  </button>
                  <button type="button" class=button_class(Variant::Outline, Size::Sm, "") on:click=move |_| start_cards()>
                    {move || t("闪卡复习")}
                  </button>
                  <button type="button" class=button_class(Variant::Outline, Size::Sm, "") on:click=move |_| export()>
                    {move || t("导出")}
                  </button>
                  <a
                    class=button_class(Variant::Outline, Size::Sm, "")
                    href=move || match bank.get() {
                      Some(b) => format!("/print?src=mistakes&bank={b}"),
                      None => "/print?src=mistakes".to_owned(),
                    }
                  >
                    {move || t("打印")}
                  </a>
                  <button
                    type="button"
                    class=button_class(Variant::Ghost, Size::Sm, "text-muted-foreground")
                    on:click=move |_| confirm_clear.set(true)
                  >
                    {move || t("清空")}
                  </button>
                }
              })
          }}
          <span class="rounded-full border px-3 py-1 text-xs text-muted-foreground">
            {move || match bank.get() {
              Some(b) => tf(
                "{} 类 {} / 共 {} 道错题",
                &[
                  &b.to_string(),
                  &bank_count.get().to_string(),
                  &records.with(Vec::len).to_string(),
                ],
              ),
              None => tf("共 {} 道错题", &[&records.with(Vec::len).to_string()]),
            }}
          </span>
        </div>
        {move || {
          confirm_clear.get().then(|| {
            view! {
              <div class="mx-auto flex max-w-3xl flex-wrap items-center gap-2 px-4 pb-3 text-sm">
                <span class="mr-auto text-red-700 dark:text-red-400">{move || t("确定清空全部错题吗？此操作不可撤销。")}</span>
                <button
                  type="button"
                  class=button_class(Variant::Destructive, Size::Sm, "")
                  on:click=move |_| {
                    study::clear_book();
                    confirm_clear.set(false);
                    refresh();
                  }
                >
                  {move || t("清空")}
                </button>
                <button type="button" class=button_class(Variant::Outline, Size::Sm, "") on:click=move |_| confirm_clear.set(false)>
                  {move || t("取消")}
                </button>
              </div>
            }
          })
        }}
      </header>

      <div class="mx-auto max-w-3xl space-y-4 px-4 py-5">
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
                  <span>{move || tf("第 {} / {} 题", &[&(current.get() + 1).to_string(), &total.to_string()])}</span>
                  <span>
                    {move || t("会")} <span class="font-semibold text-emerald-600">{card_known.get()}</span>
                    " · " {move || t("不会")} <span class="font-semibold text-red-600">{card_unknown.get()}</span>
                  </span>
                </div>
                <p class="whitespace-pre-line text-base font-medium leading-relaxed">{q.question.clone()}</p>
                {q.image().map(|src| view! { <img src=src.to_owned() alt=move || t("题目附图") class="max-h-64 rounded border" /> })}
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
                    view! { <div class="mt-4 text-xs text-muted-foreground">{move || t("先想答案，再点「显示答案」核对，然后自评会 / 不会（自评会推进复习间隔，不会则当天重来）。")}</div> }
                      .into_any()
                  }
                }}
                <div class="mt-5 flex items-center gap-2">
                  {move || {
                    if card_revealed.get() {
                      view! {
                        <button type="button" class=button_class(Variant::Default, Size::Default, "") on:click=move |_| card_mark(true)>
                          {move || t("会 ✓")}
                        </button>
                        <button type="button" class=button_class(Variant::Outline, Size::Default, "") on:click=move |_| card_mark(false)>
                          {move || t("不会 ✗")}
                        </button>
                      }
                      .into_any()
                    } else {
                      view! {
                        <button type="button" class=button_class(Variant::Default, Size::Default, "") on:click=move |_| card_revealed.set(true)>
                          {move || t("显示答案")}
                        </button>
                        <button
                          type="button"
                          class=button_class(Variant::Ghost, Size::Default, "ml-auto text-muted-foreground")
                          on:click=move |_| {
                            carding.set(false);
                            refresh();
                          }
                        >
                          {move || t("结束")}
                        </button>
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
                  <span>{move || tf("第 {} / {} 题", &[&(current.get() + 1).to_string(), &total.to_string()])}</span>
                  <span>
                    {move || t("正确")} <span class="font-semibold text-emerald-600">{correct.get()}</span>
                    "　" {move || t("错误")} <span class="font-semibold text-red-600">{wrong.get()}</span>
                  </span>
                </div>
                <p class="whitespace-pre-line text-sm font-medium leading-snug">{m.question.question.clone()}</p>
                {m.question.image().map(|src| view! { <img src=src.to_owned() alt=move || t("题目附图") class="max-h-64 rounded border" /> })}
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
                  <button
                    type="button"
                    class=button_class(Variant::Default, Size::Default, "")
                    disabled=move || feedback.get().is_some() || selected.get().is_empty()
                    on:click=move |_| submit()
                  >
                    {move || t("提交")}
                  </button>
                  <button
                    type="button"
                    class=button_class(Variant::Outline, Size::Default, "")
                    disabled=move || feedback.get().is_none()
                    on:click=move |_| next()
                  >
                    {move || t("下一题")}
                  </button>
                  <button
                    type="button"
                    class=button_class(Variant::Ghost, Size::Default, "ml-auto text-muted-foreground")
                    on:click=move |_| exit()
                  >
                    {move || t("结束")}
                  </button>
                </div>
                <div class="min-h-5 text-sm">
                  {move || {
                    feedback.get().map(|outcome| match outcome {
                      RecordOutcome::Wrong => view! {
                        <div class="space-y-2">
                          <span class="font-medium text-red-700 dark:text-red-400">
                            {move || t("正确答案：")} <span class="font-mono font-semibold">{answer.clone()}</span>
                            <span class="ml-2 text-xs font-normal text-muted-foreground">{move || t("连续答对次数已清零")}</span>
                          </span>
                          <div class="flex flex-wrap items-center gap-1.5">
                            <span class="text-xs text-muted-foreground">{move || t("为什么错？")}</span>
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
                        <span class="font-medium text-emerald-700 dark:text-emerald-400">{move || t("正确！已掌握，移出错题本")}</span>
                      }.into_any(),
                      RecordOutcome::Progressed { streak, target, next_days } => view! {
                        <span class="font-medium text-emerald-700 dark:text-emerald-400">
                          {tf("正确！已连续答对 {} / {}，{} 天后再复习", &[&streak.to_string(), &target.to_string(), &next_days.to_string()])}
                        </span>
                      }.into_any(),
                      RecordOutcome::Correct => view! {
                        <span class="font-medium text-emerald-700 dark:text-emerald-400">{move || t("正确！")}</span>
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
                <div class="text-lg font-semibold">{move || t("重练完成")}</div>
                <div class="mt-2 text-sm text-muted-foreground">
                  {move || t("正确")} " " <span class="font-semibold text-emerald-600">{correct.get()}</span>
                  "　" {move || t("错误")} " " <span class="font-semibold text-red-600">{wrong.get()}</span>
                  "　" {move || t("已掌握")} " " <span class="font-semibold text-foreground">{mastered.get()}</span>
                </div>
                <button type="button" class=format!("{} mt-5", button_class(Variant::Default, Size::Default, "")) on:click=move |_| exit()>
                  {move || t("返回列表")}
                </button>
              </div>
            }
            .into_any()
          } else if records.with(Vec::is_empty) {
            view! {
              <div class="rounded-xl border bg-card px-4 py-12 text-center">
                <div class="text-sm font-medium">{move || t("暂无错题")}</div>
                <div class="mt-1 text-xs text-muted-foreground">
                  {move || t("去「练习」「模拟考试」或「闪卡」作答后，答错的题目会自动收进这里。")}
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
                          <h3 class="text-sm font-semibold">{move || t("复习前瞻")}</h3>
                          <span class="text-xs text-muted-foreground">
                            {tf("未来 7 天共 {} 道待复习", &[&review_total.to_string()])}
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
                                0 => t("今天"),
                                1 => t("明天"),
                                2 => t("后天"),
                                _ => tf("{} 天后", &[&i.to_string()]),
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
                                    title=tf("{}：{} 道", &[&label, &n.to_string()])
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
                    <h3 class="mb-2 text-sm font-semibold">{move || t("薄弱分类")}</h3>
                    <div class="flex flex-wrap gap-1.5">
                      {counts
                        .into_iter()
                        .map(|(key, name, n)| {
                          view! {
                            <button
                              type="button"
                              class="whitespace-nowrap rounded-full border px-3 py-1 text-xs transition-colors hover:bg-accent"
                              title=tf("只看「{}」的错题", &[(name)])
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
                        <h3 class="mb-2 text-sm font-semibold">{move || t("错因分布")}</h3>
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
                <select
                  prop:value=move || filter.get()
                  on:change=move |e| filter.set(event_target_value(&e))
                  class=format!("{} w-full sm:w-48", input_class(""))
                >
                  <option value="">{move || t("全部分类")}</option>
                  {TOP_CATEGORIES
                    .iter()
                    .map(|c| {
                      view! { <option value=c.key>{c.name}</option> }
                    })
                    .collect_view()}
                </select>
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
                        {b.map_or_else(|| t("全部"), |b| tf("{} 类", &[&b.to_string()]))}
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
      </div>
    </div>
  }
}
