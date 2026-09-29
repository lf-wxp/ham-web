//! 术语表：业余无线电常用术语、英文缩写与通俗解释，按题目类型分类，并统计在各题库中出现的题数。

use std::collections::HashMap;
use std::sync::Arc;

use ham_exam_core::Bank;
use ham_exam_core::categories::{TOP_CATEGORIES, top_category};
use ham_exam_core::glossary::{GlossaryEntry, OTHER_CATEGORY};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_query_map;

use crate::cn::cn;
use crate::data;
use crate::icons::{Icon, IconKind};
use crate::util::{set_title, window};

const PAGE: usize = 60;
const ALL: &str = "全部";
const OTHER_COLOR: &str = "#71717a";

/// 各题库题目的小写搜索文本（与分类浏览页的关键词搜索范围一致）。
type Haystacks = Arc<Vec<(Bank, Vec<String>)>>;

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

/// 分类排序位置：按一级分类定义顺序，「其他」排最后。
fn category_pos(key: &str) -> usize {
  TOP_CATEGORIES
    .iter()
    .position(|c| c.key == key)
    .unwrap_or(TOP_CATEGORIES.len())
}

fn category_meta(key: &str) -> (&'static str, &'static str) {
  top_category(key).map_or((OTHER_CATEGORY, OTHER_COLOR), |c| (c.name, c.color))
}

/// 过短的纯 ASCII 术语（如 `K`、`73`、`DE`）做子串统计会产生大量误匹配，不统计题库出现次数。
fn is_countable(term: &str) -> bool {
  !term.is_ascii() || term.chars().count() >= 3
}

fn browse_href(bank: Bank, term: &str) -> String {
  format!(
    "/browse?bank={bank}&q={}",
    String::from(js_sys::encode_uri_component(term))
  )
}

#[component]
pub fn GlossaryPage() -> impl IntoView {
  set_title("术语表");
  let entries: &'static [GlossaryEntry] = data::glossary().entries();
  let query = use_query_map();

  let kw = RwSignal::new(String::new());
  let cat = RwSignal::new(ALL.to_owned());
  let abbr_only = RwSignal::new(false);
  let visible = RwSignal::new(PAGE);
  let haystacks: RwSignal<Option<Haystacks>> = RwSignal::new(None);

  // 支持 /glossary?q=驻波比 直接定位
  Effect::new(move |_| {
    if let Some(q) = query.with(|q| q.get("q")) {
      kw.set(q);
      cat.set(ALL.to_owned());
      visible.set(PAGE);
    }
  });

  spawn_local(async move {
    let mut out = Vec::with_capacity(Bank::ALL.len());
    for b in Bank::ALL {
      let texts = data::load_bank(None, b, false)
        .await
        .map(|qs| qs.iter().map(|q| q.search_text()).collect())
        .unwrap_or_default();
      out.push((b, texts));
    }
    haystacks.try_set(Some(Arc::new(out)));
  });

  let base = Memo::new(move |_| {
    let only = abbr_only.get();
    (0..entries.len())
      .filter(|&i| !only || entries[i].abbreviation().is_some())
      .collect::<Vec<_>>()
  });

  let cat_count = Memo::new(move |_| {
    let mut m: HashMap<&'static str, usize> = HashMap::new();
    base.with(|idx| {
      for &i in idx {
        *m.entry(entries[i].category_key()).or_default() += 1;
      }
    });
    m
  });

  let filtered = Memo::new(move |_| {
    let q = kw.with(|k| k.trim().to_lowercase());
    let c = cat.get();
    let mut ranked: Vec<(u8, usize, usize)> = base.with(|idx| {
      idx
        .iter()
        .filter_map(|&i| {
          let e = &entries[i];
          if c != ALL && e.category_key() != c {
            return None;
          }
          Some((e.match_rank(&q)?, category_pos(e.category_key()), i))
        })
        .collect()
    });
    ranked.sort_unstable();
    ranked.into_iter().map(|(_, _, i)| i).collect::<Vec<_>>()
  });

  let abbr_count = Memo::new(move |_| {
    base.with(|idx| {
      idx
        .iter()
        .filter(|&&i| entries[i].abbreviation().is_some())
        .count()
    })
  });
  let category_total = Memo::new(move |_| cat_count.with(HashMap::len));

  let select_cat = move |key: &str| {
    cat.set(key.to_owned());
    visible.set(PAGE);
  };
  let jump_to = Callback::new(move |term: String| {
    kw.set(term);
    cat.set(ALL.to_owned());
    visible.set(PAGE);
    window().scroll_to_with_x_and_y(0.0, 0.0);
  });

  // 侧栏与移动端分类条共用的分类列表（仅显示有词条的分类）
  let categories = move || {
    let mut keys: Vec<&'static str> = TOP_CATEGORIES.iter().map(|c| c.key).collect();
    keys.push(OTHER_CATEGORY);
    keys.retain(|k| cat_count.with(|m| m.get(k).copied().unwrap_or(0)) > 0);
    keys
  };

  let sidebar = move || {
    categories()
      .into_iter()
      .map(|key| {
        let (name, color) = category_meta(key);
        view! {
          <button
            type="button"
            on:click=move |_| select_cat(key)
            class=move || {
              pill(cat.with(|c| c == key), "flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium transition-colors")
            }
          >
            <span class="h-2 w-2 shrink-0 rounded-full" style=format!("background: {color}")></span>
            <span class="truncate">{name}</span>
            <span class="ml-auto text-xs opacity-70">{move || cat_count.with(|m| m.get(key).copied().unwrap_or(0))}</span>
          </button>
        }
      })
      .collect_view()
  };

  let chips = move || {
    let mut keys = vec![ALL];
    keys.extend(categories());
    keys
      .into_iter()
      .map(|key| {
        let label = if key == ALL { "全部" } else { category_meta(key).0 };
        view! {
          <button
            type="button"
            on:click=move |_| select_cat(key)
            class=move || pill(cat.with(|c| c == key), "shrink-0 rounded-full border px-3 py-1 text-xs transition-colors")
          >
            {label}
          </button>
        }
      })
      .collect_view()
  };

  let list = move || {
    let total = filtered.with(Vec::len);
    if total == 0 {
      return view! { <div class="py-20 text-center text-muted-foreground">"没有匹配的术语"</div> }
        .into_any();
    }
    let shown: Vec<usize> = filtered.with(|f| f.iter().copied().take(visible.get()).collect());
    let shown_len = shown.len();
    let cards = shown
      .into_iter()
      .map(|i| view! { <TermCard entry=&entries[i] haystacks=haystacks on_jump=jump_to /> })
      .collect_view();
    view! {
      <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">{cards}</div>
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
            <div class="text-base font-semibold leading-tight">"术语表"</div>
            <div class="text-xs text-muted-foreground">"业余无线电常用术语 · 英文缩写 · 通俗解释"</div>
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
              placeholder="搜索术语 / 缩写 / 解释…"
              class="h-9 w-56 rounded-lg border bg-background pl-8 pr-8 text-sm outline-none focus:ring-2 focus:ring-ring/50"
            />
            {move || {
              kw.with(|k| !k.is_empty())
                .then(|| {
                  view! {
                    <button
                      type="button"
                      aria-label="清除"
                      class="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                      on:click=move |_| kw.set(String::new())
                    >
                      <Icon kind=IconKind::X class="h-4 w-4" />
                    </button>
                  }
                })
            }}
          </div>

          <button
            type="button"
            on:click=move |_| {
              abbr_only.update(|v| *v = !*v);
              visible.set(PAGE);
            }
            class=move || pill(abbr_only.get(), "rounded-lg border px-3 py-1.5 text-sm transition-colors")
          >
            "只看英文缩写"
          </button>
        </div>
      </header>

      <div class="mx-auto grid max-w-6xl grid-cols-1 gap-5 px-4 py-5 md:grid-cols-[240px_1fr]">
        <aside class="hidden md:block">
          <div class="sticky top-[76px] max-h-[calc(100vh-90px)] overflow-y-auto rounded-xl border bg-card p-2">
            <button
              type="button"
              on:click=move |_| select_cat(ALL)
              class=move || {
                pill(cat.with(|c| c == ALL), "flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium transition-colors")
              }
            >
              <span class="h-2 w-2 rounded-full bg-foreground/60"></span>
              "全部术语"
              <span class="ml-auto text-xs opacity-70">{move || base.with(Vec::len)}</span>
            </button>
            {sidebar}
          </div>
        </aside>

        <main class="min-w-0">
          <div class="-mx-4 mb-4 flex gap-2 overflow-x-auto px-4 pb-1 md:hidden">{chips}</div>
          <div class="mb-4 grid grid-cols-2 gap-3 sm:grid-cols-4">
            <Stat label="术语总数" value=Signal::derive(move || base.with(Vec::len)) />
            <Stat label="英文缩写" value=abbr_count />
            <Stat label="术语分类" value=category_total />
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
fn TermCard(
  entry: &'static GlossaryEntry,
  haystacks: RwSignal<Option<Haystacks>>,
  on_jump: Callback<String>,
) -> impl IntoView {
  let (cat_name, color) = category_meta(entry.category_key());
  let term_class = if entry.term.is_ascii() {
    "font-mono text-base font-semibold"
  } else {
    "text-base font-semibold"
  };
  let abbr = entry.abbr.as_deref().filter(|a| *a != entry.term);
  let countable = is_countable(&entry.term);

  // 题库出现次数（懒计算，仅对当前渲染的卡片统计）
  let counts = Memo::new(move |_| {
    if !countable {
      return None;
    }
    let needle = entry.term.to_lowercase();
    haystacks.with(|h| {
      h.as_ref().map(|banks| {
        banks
          .iter()
          .map(|(b, texts)| (*b, texts.iter().filter(|t| t.contains(&needle)).count()))
          .collect::<Vec<_>>()
      })
    })
  });

  let footer = move || {
    if !countable {
      return None;
    }
    let body = match counts.get() {
      None => view! { <span>"统计题库出现次数…"</span> }.into_any(),
      Some(list) if list.iter().all(|(_, n)| *n == 0) => view! { <span>"题库中暂无直接出现该术语的题目"</span> }.into_any(),
      Some(list) => view! {
        <span>"题库出现："</span>
        {list
          .into_iter()
          .filter(|(_, n)| *n > 0)
          .map(|(b, n)| {
            view! {
              <a
                href=browse_href(b, &entry.term)
                class="rounded-md border px-2 py-0.5 transition-colors hover:bg-accent hover:text-foreground"
                title=format!("在 {b} 类题库中查看包含「{}」的题目", entry.term)
              >
                {b.as_str()}
                " 类 "
                <span class="font-semibold tabular-nums">{n}</span>
              </a>
            }
          })
          .collect_view()}
      }
      .into_any(),
    };
    Some(view! {
      <div class="mt-3 flex flex-wrap items-center gap-2 border-t border-dashed pt-3 text-xs text-muted-foreground">
        {body}
      </div>
    })
  };

  view! {
    <article class="flex flex-col rounded-xl border bg-card p-4 sm:p-5">
      <div class="mb-2 flex flex-wrap items-center gap-2">
        <h3 class=term_class>{entry.term.as_str()}</h3>
        {abbr
          .map(|a| {
            view! {
              <span class="rounded-md bg-muted px-2 py-0.5 font-mono text-xs font-semibold text-foreground">{a}</span>
            }
          })}
        <span
          class="ml-auto rounded-full px-2.5 py-0.5 text-xs font-semibold text-white"
          style=format!("background: {color}")
        >
          {cat_name}
        </span>
      </div>
      {entry.en.as_deref().map(|en| view! { <div class="mb-2 text-xs italic text-muted-foreground">{en}</div> })}
      <p class="text-sm leading-6 text-muted-foreground">{entry.desc.as_str()}</p>
      {(!entry.aliases.is_empty())
        .then(|| {
          view! { <div class="mt-2 text-xs text-muted-foreground">"又称：" {entry.aliases.join("、")}</div> }
        })}
      {entry
        .see
        .as_deref()
        .map(|see| {
          view! {
            <div class="mt-2 text-xs text-muted-foreground">
              "参见："
              <button
                type="button"
                class="font-medium text-foreground underline-offset-2 hover:underline"
                on:click=move |_| on_jump.run(see.to_owned())
              >
                {see}
              </button>
            </div>
          }
        })}
      <div class="mt-auto">{footer}</div>
    </article>
  }
}
