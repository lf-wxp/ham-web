use std::collections::HashSet;

use ham_web_core::categories::{TOP_CATEGORIES, sub_category};
use ham_web_core::mistakes::{Mistake, mistakes_from_state};
use ham_web_core::{Bank, QuestionItem};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::cn::cn;
use crate::data;
use crate::store;
use crate::ui::{Size, Variant, button_class, input_class};
use crate::util::{download_text, set_title};

use super::mistake_card::MistakeCard;

/// 题目所属一级分类（用于分类筛选），无分类码时归入「其他」。
fn top_of(question: &QuestionItem) -> &'static str {
  question
    .p_code()
    .and_then(sub_category)
    .map_or("其他", |s| s.top)
}

/// 把错题导出为纯文本。
fn export_text(mistakes: &[Mistake]) -> String {
  let mut s = String::new();
  for (i, m) in mistakes.iter().enumerate() {
    let j = m.question.j_code().unwrap_or("—");
    s.push_str(&format!("{}. [{}] {}\n", i + 1, j, m.question.question));
    s.push_str(&format!("   正确答案：{}\n\n", m.correct.join("、")));
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

  let mistakes = RwSignal::new(Vec::<Mistake>::new());
  let loading = RwSignal::new(true);
  let practicing = RwSignal::new(false);
  let finished = RwSignal::new(false);
  let current = RwSignal::new(0usize);
  let selected = RwSignal::new(Vec::<String>::new());
  let feedback = RwSignal::new(None::<bool>);
  let correct = RwSignal::new(0usize);
  let wrong = RwSignal::new(0usize);
  let filter = RwSignal::new(String::new());

  spawn_local(async move {
    let mut all: Vec<Mistake> = Vec::new();
    let versions = data::all_versions(false).await.unwrap_or_default();
    let mut version_ids: Vec<Option<String>> = vec![None];
    version_ids.extend(versions.iter().map(|v| Some(v.id.clone())));

    for vid in version_ids {
      let vid_ref = vid.as_deref();
      for bank in Bank::ALL {
        if let Some(state) = store::load_practice(bank, vid_ref)
          && let Ok(qs) = data::load_bank(vid_ref, bank, false).await
        {
          all.extend(mistakes_from_state(
            &qs,
            &state.order_indices,
            &state.answers_by_position,
          ));
        }
        if let Some(state) = store::load_exam(bank, vid_ref)
          && let Ok(qs) = data::load_bank(vid_ref, bank, false).await
        {
          let rebuilt = state.reconstruct(&qs);
          let order: Vec<usize> = (0..rebuilt.len()).collect();
          all.extend(mistakes_from_state(
            &rebuilt,
            &order,
            &state.answers_by_position,
          ));
        }
      }
    }

    let mut seen: HashSet<String> = HashSet::new();
    all.retain(|m| {
      let key = m
        .question
        .stable_id()
        .unwrap_or_else(|| m.question.question.clone());
      seen.insert(key)
    });

    mistakes.set(all);
    loading.set(false);
  });

  let start = move || {
    if mistakes.get_untracked().is_empty() {
      return;
    }
    practicing.set(true);
    finished.set(false);
    current.set(0);
    selected.set(Vec::new());
    feedback.set(None);
    correct.set(0);
    wrong.set(0);
  };

  let toggle = move |key: String| {
    let all = mistakes.get_untracked();
    let Some(m) = all.get(current.get_untracked()) else {
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
    let all = mistakes.get_untracked();
    let Some(m) = all.get(current.get_untracked()) else {
      return;
    };
    let sel = selected.get_untracked();
    if sel.is_empty() {
      return;
    }
    let ok = m.question.is_answer_correct(&sel);
    feedback.set(Some(ok));
    if ok {
      correct.update(|c| *c += 1);
    } else {
      wrong.update(|c| *c += 1);
    }
  };

  let next = move || {
    let total = mistakes.get_untracked().len();
    if current.get_untracked() + 1 >= total {
      finished.set(true);
      practicing.set(false);
    } else {
      current.update(|c| *c += 1);
      selected.set(Vec::new());
      feedback.set(None);
    }
  };

  let exit = move || {
    practicing.set(false);
    finished.set(false);
  };

  let export = move || {
    let mut list = mistakes.get_untracked();
    let f = filter.get_untracked();
    if !f.is_empty() {
      list.retain(|m| top_of(&m.question) == f);
    }
    download_text("mistakes.txt", &export_text(&list), "text/plain");
  };

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-3xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <div class="text-base font-semibold leading-tight">"错题集"</div>
            <div class="text-xs text-muted-foreground">"汇总练习与模拟考试中的错题"</div>
          </div>
          {move || {
            (!practicing.get() && !finished.get() && !loading.get() && !mistakes.get().is_empty())
              .then(|| {
                view! {
                  <button type="button" class=button_class(Variant::Default, Size::Sm, "") on:click=move |_| start()>
                    "开始重练"
                  </button>
                }
              })
          }}
          {move || {
            (!practicing.get() && !finished.get() && !loading.get() && !mistakes.get().is_empty())
              .then(|| {
                view! {
                  <button type="button" class=button_class(Variant::Outline, Size::Sm, "") on:click=move |_| export()>
                    "导出"
                  </button>
                }
              })
          }}
          <span class="rounded-full border px-3 py-1 text-xs text-muted-foreground">
            {move || format!("共 {} 道错题", mistakes.get().len())}
          </span>
        </div>
      </header>

      <div class="mx-auto max-w-3xl space-y-4 px-4 py-5">
        {move || {
          if loading.get() {
            view! { <div class="px-4 py-10 text-center text-sm text-muted-foreground">"正在汇总错题..."</div> }
              .into_any()
          } else if practicing.get() {
            // 重练视图
            let Some(m) = mistakes.get_untracked().get(current.get()).cloned() else {
              return view! { <div></div> }.into_any();
            };
            let total = mistakes.get_untracked().len();
            view! {
              <div class="space-y-4 rounded-xl border bg-card p-5">
                <div class="flex items-center justify-between text-xs text-muted-foreground">
                  <span>{"第 "} <span class="font-semibold text-foreground">{current.get() + 1}</span> {" / "} {total} {" 题"}</span>
                  <span>{"正确 "} <span class="font-semibold text-emerald-600">{correct.get()}</span> {"　错误 "} <span class="font-semibold text-red-600">{wrong.get()}</span></span>
                </div>
                <p class="text-sm font-medium leading-snug">{m.question.question.clone()}</p>
                <div class="space-y-2">
                  {m.question.options.iter().map(|o| {
                    let key = o.key.clone();
                    let class_key = key.clone();
                    let text = o.text.clone();
                    view! {
                      <button
                        type="button"
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
                    disabled=move || feedback.get().is_some()
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
                </div>
                <div class="min-h-5 text-sm">
                  {move || {
                    feedback.get().map(|ok| {
                      if ok {
                        view! { <span class="font-medium text-emerald-600 dark:text-emerald-400">"正确！"</span> }.into_any()
                      } else {
                        view! {
                          <span class="font-medium text-red-600 dark:text-red-400">
                            "正确答案：" <span class="font-mono font-semibold">{m.correct.join("、")}</span>
                          </span>
                        }.into_any()
                      }
                    })
                  }}
                </div>
              </div>
            }
            .into_any()
          } else if finished.get() {
            // 完成统计
            view! {
              <div class="rounded-xl border bg-card px-4 py-10 text-center">
                <div class="text-lg font-semibold">"重练完成"</div>
                <div class="mt-2 text-sm text-muted-foreground">
                  "正确 " <span class="font-semibold text-emerald-600">{correct.get()}</span>
                  "　错误 " <span class="font-semibold text-red-600">{wrong.get()}</span>
                </div>
                <button type="button" class=format!("{} mt-5", button_class(Variant::Default, Size::Default, "")) on:click=move |_| exit()>
                  "返回列表"
                </button>
              </div>
            }
            .into_any()
          } else if mistakes.get().is_empty() {
            view! {
              <div class="rounded-xl border bg-card px-4 py-12 text-center">
                <div class="text-sm font-medium">"暂无错题"</div>
                <div class="mt-1 text-xs text-muted-foreground">
                  "去「练习」或「模拟考试」作答后，答错的题目会汇总到这里。"
                </div>
              </div>
            }
            .into_any()
          } else {
            view! {
              {move || {
                let f = filter.get();
                if !f.is_empty() {
                  return view! { <div></div> }.into_any();
                }
                let all = mistakes.get_untracked();
                let mut counts: Vec<(&'static str, &'static str, usize)> = TOP_CATEGORIES
                  .iter()
                  .map(|c| {
                    let n = all.iter().filter(|m| top_of(&m.question) == c.key).count();
                    (c.key, c.name, n)
                  })
                  .filter(|(_, _, n)| *n > 0)
                  .collect();
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
                              class="rounded-full border px-3 py-1 text-xs transition-colors hover:bg-accent"
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
              {move || {
                let f = filter.get();
                mistakes
                  .get()
                  .into_iter()
                  .filter(|m| f.is_empty() || top_of(&m.question) == f.as_str())
                  .map(|m| view! { <MistakeCard mistake=m /> })
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
