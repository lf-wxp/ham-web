//! 题库分类浏览：按题目类型分类，仅显示正确答案，附解析、知识点与参考依据。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use ham_web_core::categories::{self, RefKind, TOP_CATEGORIES};
use ham_web_core::{Bank, QuestionItem, fingerprint};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_query_map;

use crate::cn::cn;
use crate::data::{self, Questions};
use crate::icons::{Icon, IconKind};
use crate::util::set_title;

const PAGE: usize = 40;
const ALL: &str = "全部";
const OTHER: &str = "其他";

fn top_of(q: &QuestionItem) -> Option<&'static str> {
  categories::sub_category(q.p_code()?).map(|s| s.top)
}

fn sub_name_of(q: &QuestionItem) -> Option<&'static str> {
  categories::sub_category(q.p_code()?).map(|s| s.name)
}

fn pill(active: bool, base: &str) -> String {
  cn(&[
    base,
    if active {
      "bg-primary text-primary-foreground"
    } else {
      "hover:bg-accent"
    },
  ])
}

#[component]
pub fn BrowsePage() -> impl IntoView {
  set_title("题库分类浏览");
  let all: RwSignal<Questions> = RwSignal::new(Arc::new(Vec::new()));
  let loading = RwSignal::new(true);
  let bank = RwSignal::new(Bank::A);
  let top = RwSignal::new(ALL.to_owned());
  let sub = RwSignal::new(None::<&'static str>);
  let kw = RwSignal::new(String::new());
  let multi_only = RwSignal::new(false);
  let unique_only = RwSignal::new(false);
  let visible = RwSignal::new(PAGE);
  let open_top = RwSignal::new(HashSet::<&'static str>::new());

  // 支持 /browse?bank=B&q=驻波比（术语表页跳转）
  let query = use_query_map();
  Effect::new(move |_| {
    let (b, q) = query.with(|p| (p.get("bank"), p.get("q")));
    if let Some(b) = b.and_then(|b| b.parse::<Bank>().ok()) {
      bank.set(b);
      top.set(ALL.to_owned());
      sub.set(None);
    }
    if let Some(q) = q {
      kw.set(q);
    }
    visible.set(PAGE);
  });

  spawn_local(async move {
    let mut merged = Vec::new();
    for b in Bank::ALL {
      if let Ok(qs) = data::load_bank(None, b, false).await {
        merged.extend(qs.iter().cloned());
      }
    }
    all.try_set(Arc::new(merged));
    loading.try_set(false);
  });

  // 各题库题目指纹（用于「只看本类新增」）
  let fingerprints = Memo::new(move |_| {
    let mut m: HashMap<Bank, HashSet<String>> = HashMap::new();
    all.with(|qs| {
      for q in qs.iter() {
        m.entry(Bank::of_id(q.id_str()))
          .or_default()
          .insert(fingerprint(q));
      }
    });
    m
  });

  let bank_questions = Memo::new(move |_| {
    let b = bank.get();
    let unique = unique_only.get();
    all.with(|qs| {
      fingerprints.with(|fps| {
        let has = |bank: Bank, f: &str| fps.get(&bank).is_some_and(|s| s.contains(f));
        qs.iter()
          .enumerate()
          .filter(|(_, q)| Bank::of_id(q.id_str()) == b)
          .filter(|(_, q)| {
            if !unique || b == Bank::A {
              return true;
            }
            let f = fingerprint(q);
            match b {
              Bank::B => !has(Bank::A, &f),
              _ => !has(Bank::A, &f) && !has(Bank::B, &f),
            }
          })
          .map(|(i, _)| i)
          .collect::<Vec<_>>()
      })
    })
  });

  let cat_count = Memo::new(move |_| {
    let mut m: HashMap<&'static str, usize> = HashMap::new();
    all.with(|qs| {
      bank_questions.with(|idx| {
        for &i in idx {
          *m.entry(top_of(&qs[i]).unwrap_or(OTHER)).or_default() += 1;
        }
      });
    });
    m
  });

  let sub_count = Memo::new(move |_| {
    let mut m: HashMap<&'static str, usize> = HashMap::new();
    all.with(|qs| {
      bank_questions.with(|idx| {
        for &i in idx {
          if let Some(name) = sub_name_of(&qs[i]) {
            *m.entry(name).or_default() += 1;
          }
        }
      });
    });
    m
  });

  let filtered = Memo::new(move |_| {
    let k = kw.with(|k| k.trim().to_lowercase());
    let (t, s, multi) = (top.get(), sub.get(), multi_only.get());
    all.with(|qs| {
      bank_questions.with(|idx| {
        idx
          .iter()
          .copied()
          .filter(|&i| {
            let q = &qs[i];
            if t != ALL && top_of(q) != Some(t.as_str()) {
              return false;
            }
            if s.is_some() && sub_name_of(q) != s {
              return false;
            }
            if multi && !q.is_multiple() {
              return false;
            }
            k.is_empty() || q.search_text().contains(&k)
          })
          .collect::<Vec<_>>()
      })
    })
  });

  let multi_count = Memo::new(move |_| {
    all.with(|qs| bank_questions.with(|idx| idx.iter().filter(|&&i| qs[i].is_multiple()).count()))
  });

  let select_top = move |key: &str| {
    top.set(key.to_owned());
    sub.set(None);
  };
  let toggle_top = move |key: &'static str| {
    open_top.update(|s| {
      if !s.remove(key) {
        s.insert(key);
      }
    });
  };

  let sidebar = move || {
    TOP_CATEGORIES
      .iter()
      .map(|c| {
        let expanded = move || open_top.with(|s| s.contains(c.key));
        let subs = move || {
          categories::sub_names_of(c.key)
            .into_iter()
            .filter(|name| sub_count.with(|m| m.get(name).copied().unwrap_or(0)) > 0)
            .map(|name| {
              view! {
                <button
                  type="button"
                  on:click=move |_| {
                    top.set(c.key.to_owned());
                    sub.set(Some(name));
                  }
                  class=move || {
                    cn(&[
                      "block w-full truncate rounded-md px-2 py-1 text-left text-xs transition-colors",
                      if sub.get() == Some(name) {
                        "bg-accent font-medium text-foreground"
                      } else {
                        "text-muted-foreground hover:bg-accent/60"
                      },
                    ])
                  }
                >
                  {name}
                  <span class="ml-1 opacity-60">{move || sub_count.with(|m| m.get(name).copied().unwrap_or(0))}</span>
                </button>
              }
            })
            .collect_view()
        };
        view! {
          <div>
            <button
              type="button"
              on:click=move |_| {
                select_top(c.key);
                toggle_top(c.key);
              }
              class=move || {
                pill(top.with(|t| t == c.key), "flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium transition-colors")
              }
            >
              <span class="h-2 w-2 shrink-0 rounded-full" style=format!("background: {}", c.color)></span>
              <span class="truncate">{c.name}</span>
              <span class="ml-auto text-xs opacity-70">{move || cat_count.with(|m| m.get(c.key).copied().unwrap_or(0))}</span>
              <Icon
                kind=IconKind::ChevronDown
                class=Signal::derive(move || {
                  cn(&["h-3.5 w-3.5 shrink-0 transition-transform", if expanded() { "rotate-180" } else { "" }])
                })
              />
            </button>
            {move || expanded().then(|| view! { <div class="mb-1 ml-4 border-l pl-2">{subs}</div> })}
          </div>
        }
      })
      .collect_view()
  };

  let list = move || {
    if loading.get() {
      return view! {
        <div class="flex items-center justify-center gap-2 py-20 text-muted-foreground">
          <Icon kind=IconKind::Loader2 class="h-5 w-5 animate-spin" />
          " 正在加载题库…"
        </div>
      }
      .into_any();
    }
    let total = filtered.with(Vec::len);
    if total == 0 {
      return view! { <div class="py-20 text-center text-muted-foreground">"没有匹配的题目"</div> }
        .into_any();
    }
    let shown: Vec<usize> = filtered.with(|f| f.iter().copied().take(visible.get()).collect());
    let shown_len = shown.len();
    let rows = all.with(|qs| {
      shown
        .iter()
        .map(|&i| view! { <QuestionRow q=qs[i].clone() /> })
        .collect_view()
    });
    view! {
      <div class="space-y-4">{rows}</div>
      {(shown_len < total)
        .then(|| {
          view! {
            <button
              type="button"
              on:click=move |_| visible.update(|v| *v += PAGE)
              class="mt-4 w-full rounded-lg border py-2.5 text-sm text-muted-foreground transition-colors hover:bg-accent"
            >
              "加载更多（已显示 " {shown_len} " / " {total} "）"
            </button>
          }
        })}
    }
    .into_any()
  };

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-6xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <div class="text-base font-semibold leading-tight">"题库分类浏览"</div>
            <div class="text-xs text-muted-foreground">"按题目类型分类 · 仅显示正确答案 · 附解析与参考依据"</div>
          </div>

          <div class="flex overflow-hidden rounded-lg border">
            {Bank::ALL
              .into_iter()
              .map(|b| {
                view! {
                  <button
                    type="button"
                    on:click=move |_| {
                      bank.set(b);
                      visible.set(PAGE);
                      select_top(ALL);
                    }
                    class=move || pill(bank.get() == b, "px-3 py-1.5 text-sm font-medium transition-colors")
                  >
                    {b.as_str()}
                    " 类"
                  </button>
                }
              })
              .collect_view()}
          </div>

          <div class="relative">
            <Icon
              kind=IconKind::Search
              class="pointer-events-none absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground"
            />
            <input
              prop:value=move || kw.get()
              on:input=move |e| {
                kw.set(event_target_value(&e));
                visible.set(PAGE);
              }
              placeholder="搜索题干 / 答案 / 解析…"
              class="h-9 w-56 rounded-lg border bg-background pl-8 pr-3 text-sm outline-none focus:ring-2 focus:ring-ring/50"
            />
          </div>

          <button
            type="button"
            on:click=move |_| multi_only.update(|v| *v = !*v)
            class=move || pill(multi_only.get(), "rounded-lg border px-3 py-1.5 text-sm transition-colors")
          >
            "只看多选"
          </button>
          <button
            type="button"
            on:click=move |_| unique_only.update(|v| *v = !*v)
            class=move || pill(unique_only.get(), "rounded-lg border px-3 py-1.5 text-sm transition-colors")
          >
            "只看本类新增"
          </button>
        </div>
      </header>

      <div class="mx-auto grid max-w-6xl grid-cols-1 gap-5 px-4 py-5 md:grid-cols-[240px_1fr]">
        <aside class="hidden md:block">
          <div class="sticky top-[76px] max-h-[calc(100vh-90px)] overflow-y-auto rounded-xl border bg-card p-2">
            <button
              type="button"
              on:click=move |_| select_top(ALL)
              class=move || {
                pill(top.with(|t| t == ALL), "flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium transition-colors")
              }
            >
              <span class="h-2 w-2 rounded-full bg-foreground/60"></span>
              "全部题目"
              <span class="ml-auto text-xs opacity-70">{move || bank_questions.with(Vec::len)}</span>
            </button>
            {sidebar}
          </div>
        </aside>

        <main class="min-w-0">
          <div class="mb-4 grid grid-cols-2 gap-3 sm:grid-cols-4">
            <Stat label="题目总数" value=Signal::derive(move || bank_questions.with(Vec::len)) />
            <Stat label="多选题" value=multi_count />
            <Stat label="题目类型" value=Signal::stored(TOP_CATEGORIES.len()) />
            <Stat label="当前筛选" value=Signal::derive(move || filtered.with(Vec::len)) />
          </div>
          {list}
        </main>
      </div>
    </div>
  }
}

#[component]
fn Stat(label: &'static str, #[prop(into)] value: Signal<usize>) -> impl IntoView {
  view! {
    <div class="rounded-xl border bg-card p-3">
      <div class="text-2xl font-semibold tabular-nums">{move || value.get()}</div>
      <div class="text-xs text-muted-foreground">{label}</div>
    </div>
  }
}

#[component]
fn QuestionRow(q: QuestionItem) -> impl IntoView {
  let sub = q.p_code().and_then(categories::sub_category);
  let top_cat = sub.and_then(|s| categories::top_category(s.top));
  let answers: Vec<_> = q
    .options
    .iter()
    .filter(|o| q.answer_keys.contains(&o.key))
    .cloned()
    .collect();
  let note = q.p_code().and_then(categories::sub_note);
  let refs = top_cat.map_or(&[][..], |c| categories::top_refs(c.key));
  let is_multiple = q.is_multiple();
  let answer_title = if is_multiple {
    format!("正确答案（共 {} 项）", answers.len())
  } else {
    "正确答案".to_owned()
  };

  view! {
    <article class="rounded-xl border bg-card p-4 sm:p-5">
      <div class="mb-3 flex flex-wrap items-center gap-2">
        <span class="rounded-md bg-muted px-2 py-0.5 font-mono text-xs text-muted-foreground">
          {q.id_str().or(q.j_code()).map(ToOwned::to_owned)}
        </span>
        {top_cat
          .map(|c| {
            view! {
              <span
                class="rounded-full px-2.5 py-0.5 text-xs font-semibold text-white"
                style=format!("background: {}", c.color)
              >
                {c.name}
              </span>
            }
          })}
        {sub.map(|s| view! { <span class="rounded-full bg-accent px-2.5 py-0.5 text-xs text-muted-foreground">{s.name}</span> })}
        {is_multiple
          .then(|| {
            view! {
              <span class="rounded-full bg-amber-100 px-2 py-0.5 text-xs font-semibold text-amber-700 dark:bg-amber-900/40 dark:text-amber-400">
                "多选"
              </span>
            }
          })}
        <span class="ml-auto font-mono text-xs text-muted-foreground">{q.p_code().map(ToOwned::to_owned)}</span>
      </div>

      <div class="mb-3 font-medium leading-relaxed">{q.question.clone()}</div>
      {q.image().map(|src| view! { <img src=src.to_owned() alt="题目附图" loading="lazy" class="mb-3 max-h-64 rounded-lg border" /> })}

      <div class="mb-3 rounded-lg border border-emerald-200 bg-emerald-50/60 p-3 dark:border-emerald-900/50 dark:bg-emerald-950/30">
        <div class="mb-1.5 flex items-center gap-1.5 text-xs font-semibold text-emerald-700 dark:text-emerald-400">
          <Icon kind=IconKind::CheckCircle2 class="h-3.5 w-3.5" />
          {answer_title}
        </div>
        {answers
          .into_iter()
          .map(|a| {
            view! {
              <div class="flex items-start gap-2 py-0.5 text-sm">
                <span class="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded bg-emerald-600 text-xs font-bold text-white">
                  {a.key}
                </span>
                <span class="flex-1">{a.text}</span>
              </div>
            }
          })
          .collect_view()}
      </div>

      {q
        .explanation
        .clone()
        .filter(|e| !e.is_empty())
        .map(|e| {
          view! {
            <div class="mb-3 border-t border-dashed pt-3">
              <div class="mb-1 text-xs font-semibold text-muted-foreground">"解析"</div>
              <div class="whitespace-pre-line text-sm leading-6 text-muted-foreground">{e}</div>
            </div>
          }
        })}

      {note
        .map(|n| {
          view! {
            <div class="mb-3 border-t border-dashed pt-3">
              <div class="mb-1 text-xs font-semibold text-muted-foreground">
                "知识点 · " {sub.map(|s| s.name)}
              </div>
              <div class="text-sm leading-6 text-muted-foreground">{n}</div>
            </div>
          }
        })}

      {(!refs.is_empty())
        .then(|| {
          view! {
            <div class="border-t border-dashed pt-3">
              <div class="mb-1 text-xs font-semibold text-muted-foreground">"参考依据"</div>
              <ul class="space-y-1">
                {refs
                  .iter()
                  .map(|r| {
                    let badge = cn(&[
                      "mt-0.5 shrink-0 rounded px-1.5 py-0.5 text-[10px] font-bold",
                      if r.kind == RefKind::Law {
                        "bg-indigo-100 text-indigo-700 dark:bg-indigo-900/40 dark:text-indigo-300"
                      } else {
                        "bg-emerald-100 text-emerald-700 dark:bg-emerald-900/40 dark:text-emerald-300"
                      },
                    ]);
                    let body = match r.url {
                      Some(url) => view! {
                        <a
                          href=url
                          target="_blank"
                          rel="noopener noreferrer"
                          class="inline-flex items-center gap-1 underline-offset-2 hover:underline"
                        >
                          {r.text}
                          <Icon kind=IconKind::ExternalLink class="h-3 w-3" />
                        </a>
                      }
                      .into_any(),
                      None => view! { <span>{r.text}</span> }.into_any(),
                    };
                    view! {
                      <li class="flex items-start gap-2 text-sm text-muted-foreground">
                        <span class=badge>{r.kind.label()}</span>
                        {body}
                      </li>
                    }
                  })
                  .collect_view()}
              </ul>
            </div>
          }
        })}
    </article>
  }
}
