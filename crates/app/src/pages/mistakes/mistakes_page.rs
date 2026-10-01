use ham_web_core::categories::{TOP_CATEGORIES, sub_category};
use ham_web_core::mistake_book::{MASTER_STREAK, MistakeRecord, RecordOutcome};
use ham_web_core::{Bank, QuestionItem};
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::cn::cn;
use crate::components::common::ExplanationCard;
use crate::study;
use crate::ui::{Size, Variant, button_class, input_class};
use crate::util::{download_text, now_ms, set_title};

use super::mistake_card::MistakeCard;

/// 题目所属一级分类（用于分类筛选），无分类码时归入「其他」。
fn top_of(question: &QuestionItem) -> &'static str {
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
    s.push_str(&format!(
      "   正确答案：{}（累计答错 {} 次）\n\n",
      r.question.answer_keys.join("、"),
      r.wrong_count
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
  set_title("错题集");

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
  };
  let start_due = move || {
    let now = now_ms();
    let mut list: Vec<MistakeRecord> = filtered().into_iter().filter(|m| m.is_due(now)).collect();
    list.sort_by_key(|m| m.due_ms);
    start_with(list);
  };
  let start_all = move || start_with(filtered());

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

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-3xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"错题集"</h1>
            <div class="text-xs text-muted-foreground">
              {format!("练习 · 模拟考试 · 闪卡中答错的题；复习间隔随作答自适应，连续答对 {MASTER_STREAK} 次（常错题更多）自动移出")}
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
                    {move || format!("今日待复习 {}", due_count.get())}
                  </button>
                  <button type="button" class=button_class(Variant::Outline, Size::Sm, "") on:click=move |_| start_all()>
                    "全部重练"
                  </button>
                  <button type="button" class=button_class(Variant::Outline, Size::Sm, "") on:click=move |_| export()>
                    "导出"
                  </button>
                  <a
                    class=button_class(Variant::Outline, Size::Sm, "")
                    href=move || match bank.get() {
                      Some(b) => format!("/print?src=mistakes&bank={b}"),
                      None => "/print?src=mistakes".to_owned(),
                    }
                  >
                    "打印"
                  </a>
                  <button
                    type="button"
                    class=button_class(Variant::Ghost, Size::Sm, "text-muted-foreground")
                    on:click=move |_| confirm_clear.set(true)
                  >
                    "清空"
                  </button>
                }
              })
          }}
          <span class="rounded-full border px-3 py-1 text-xs text-muted-foreground">
            {move || match bank.get() {
              Some(b) => format!("{b} 类 {} / 共 {} 道错题", bank_count.get(), records.with(Vec::len)),
              None => format!("共 {} 道错题", records.with(Vec::len)),
            }}
          </span>
        </div>
        {move || {
          confirm_clear.get().then(|| {
            view! {
              <div class="mx-auto flex max-w-3xl flex-wrap items-center gap-2 px-4 pb-3 text-sm">
                <span class="mr-auto text-red-700 dark:text-red-400">"确定清空全部错题吗？此操作不可撤销。"</span>
                <button
                  type="button"
                  class=button_class(Variant::Destructive, Size::Sm, "")
                  on:click=move |_| {
                    study::clear_book();
                    confirm_clear.set(false);
                    refresh();
                  }
                >
                  "清空"
                </button>
                <button type="button" class=button_class(Variant::Outline, Size::Sm, "") on:click=move |_| confirm_clear.set(false)>
                  "取消"
                </button>
              </div>
            }
          })
        }}
      </header>

      <div class="mx-auto max-w-3xl space-y-4 px-4 py-5">
        {move || {
          if practicing.get() {
            // 重练视图
            let Some(m) = session.with_untracked(|s| s.get(current.get()).cloned()) else {
              return view! { <div></div> }.into_any();
            };
            let total = session.with_untracked(Vec::len);
            let answer = m.question.answer_keys.join("、");
            let expl = m.question.clone();
            view! {
              <div class="space-y-4 rounded-xl border bg-card p-5">
                <div class="flex items-center justify-between text-xs text-muted-foreground">
                  <span>{"第 "} <span class="font-semibold text-foreground">{current.get() + 1}</span> {" / "} {total} {" 题"}</span>
                  <span>{"正确 "} <span class="font-semibold text-emerald-600">{correct.get()}</span> {"　错误 "} <span class="font-semibold text-red-600">{wrong.get()}</span></span>
                </div>
                <p class="whitespace-pre-line text-sm font-medium leading-snug">{m.question.question.clone()}</p>
                {m.question.image().map(|src| view! { <img src=src.to_owned() alt="题目附图" class="max-h-64 rounded border" /> })}
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
                    "提交"
                  </button>
                  <button
                    type="button"
                    class=button_class(Variant::Outline, Size::Default, "")
                    disabled=move || feedback.get().is_none()
                    on:click=move |_| next()
                  >
                    "下一题"
                  </button>
                  <button
                    type="button"
                    class=button_class(Variant::Ghost, Size::Default, "ml-auto text-muted-foreground")
                    on:click=move |_| exit()
                  >
                    "结束"
                  </button>
                </div>
                <div class="min-h-5 text-sm">
                  {move || {
                    feedback.get().map(|outcome| match outcome {
                      RecordOutcome::Wrong => view! {
                        <span class="font-medium text-red-700 dark:text-red-400">
                          "正确答案：" <span class="font-mono font-semibold">{answer.clone()}</span>
                          <span class="ml-2 text-xs font-normal text-muted-foreground">"连续答对次数已清零"</span>
                        </span>
                      }.into_any(),
                      RecordOutcome::Mastered => view! {
                        <span class="font-medium text-emerald-700 dark:text-emerald-400">"正确！已掌握，移出错题本"</span>
                      }.into_any(),
                      RecordOutcome::Progressed { streak, target, next_days } => view! {
                        <span class="font-medium text-emerald-700 dark:text-emerald-400">
                          {format!("正确！已连续答对 {streak} / {target}，{next_days} 天后再复习")}
                        </span>
                      }.into_any(),
                      RecordOutcome::Correct => view! {
                        <span class="font-medium text-emerald-700 dark:text-emerald-400">"正确！"</span>
                      }.into_any(),
                    })
                  }}
                </div>
                {move || feedback.get().is_some().then(|| view! { <ExplanationCard question=expl.clone() /> })}
              </div>
            }
            .into_any()
          } else if finished.get() {
            view! {
              <div class="rounded-xl border bg-card px-4 py-10 text-center">
                <div class="text-lg font-semibold">"重练完成"</div>
                <div class="mt-2 text-sm text-muted-foreground">
                  "正确 " <span class="font-semibold text-emerald-600">{correct.get()}</span>
                  "　错误 " <span class="font-semibold text-red-600">{wrong.get()}</span>
                  "　已掌握 " <span class="font-semibold text-foreground">{mastered.get()}</span>
                </div>
                <button type="button" class=format!("{} mt-5", button_class(Variant::Default, Size::Default, "")) on:click=move |_| exit()>
                  "返回列表"
                </button>
              </div>
            }
            .into_any()
          } else if records.with(Vec::is_empty) {
            view! {
              <div class="rounded-xl border bg-card px-4 py-12 text-center">
                <div class="text-sm font-medium">"暂无错题"</div>
                <div class="mt-1 text-xs text-muted-foreground">
                  "去「练习」「模拟考试」或「闪卡」作答后，答错的题目会自动收进这里。"
                </div>
              </div>
            }
            .into_any()
          } else {
            view! {
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
                    <h3 class="mb-2 text-sm font-semibold">"薄弱分类"</h3>
                    <div class="flex flex-wrap gap-1.5">
                      {counts
                        .into_iter()
                        .map(|(key, name, n)| {
                          view! {
                            <button
                              type="button"
                              class="whitespace-nowrap rounded-full border px-3 py-1 text-xs transition-colors hover:bg-accent"
                              title=format!("只看「{name}」的错题")
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

              <div class="flex flex-wrap items-center gap-2">
                <select
                  prop:value=move || filter.get()
                  on:change=move |e| filter.set(event_target_value(&e))
                  class=format!("{} w-full sm:w-48", input_class(""))
                >
                  <option value="">"全部分类"</option>
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
                        {b.map_or_else(|| "全部".to_owned(), |b| format!("{b} 类"))}
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
