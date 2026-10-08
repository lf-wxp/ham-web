use std::collections::HashMap;
use std::sync::Arc;

use ham_web_core::Bank;
use ham_web_core::categories::TOP_CATEGORIES;
use ham_web_core::glossary::{GlossaryEntry, OTHER_CATEGORY};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_query_map;

use crate::data;
use crate::icons::{Icon, IconKind};
use crate::pages::SLANG_CATEGORY;
use crate::ui::{Input, InputType};
use crate::util::window;

use super::stat::Stat;
use super::term_card::TermCard;
use super::{ALL, Haystacks, PAGE, category_meta, category_pos, pill};
use crate::i18n::t;

/// 术语表主体：搜索、分类筛选与词条列表。
#[component]
pub(crate) fn GlossaryView(entries: &'static [GlossaryEntry]) -> impl IntoView {
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
      .filter(|&i| entries[i].category_key() != SLANG_CATEGORY)
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
            <span class="truncate" title=name.to_string()>{name}</span>
            <span class="ml-auto text-xs tabular-nums">{move || cat_count.with(|m| m.get(key).copied().unwrap_or(0))}</span>
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
        let label = if key == ALL { t("exam.all") } else { t(category_meta(key).0) };
        view! {
          <button
            type="button"
            on:click=move |_| select_cat(key)
            class=move || pill(cat.with(|c| c == key), "shrink-0 inline-flex items-center whitespace-nowrap rounded-full border px-3 py-1 text-xs transition-colors")
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
      return view! { <div class="py-20 text-center text-muted-foreground">{move || t("knowledge.no-matching-terms")}</div> }
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
              {t("knowledge.load-more-showing")} {shown_len} " / " {total} {t("common.entry-2")}
            </button>
          }
        })}
    }
    .into_any()
  };

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-16 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("shell.glossary")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("knowledge.common-ham-terms-english")}</div>
          </div>

          <Input
            value=kw
            on_change=Callback::new(move |v: String| {
              kw.set(v);
              visible.set(PAGE);
            })
            kind=InputType::Search
            placeholder=Signal::derive(move || t("knowledge.search-terms-abbreviations-explanations"))
            prefix=move || view! { <Icon kind=IconKind::Search /> }
            clearable=true
            class="w-56"
          />

          <button
            type="button"
            on:click=move |_| {
              abbr_only.update(|v| *v = !*v);
              visible.set(PAGE);
            }
            class=move || pill(abbr_only.get(), "rounded-lg border px-3 py-1.5 text-sm transition-colors")
          >
            {move || t("knowledge.english-abbreviations-only")}
          </button>
        </div>
      </header>

      <div class="mx-auto grid max-w-5xl grid-cols-1 gap-5 px-4 py-5 md:grid-cols-[240px_1fr]">
        <aside class="hidden md:block">
          <div class="sticky top-[136px] max-h-[calc(100vh-152px)] overflow-y-auto rounded-xl border bg-card p-2">
            <button
              type="button"
              on:click=move |_| select_cat(ALL)
              class=move || {
                pill(cat.with(|c| c == ALL), "flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium transition-colors")
              }
            >
              <span class="h-2 w-2 rounded-full bg-foreground/60"></span>
              {move || t("knowledge.all-terms")}
              <span class="ml-auto text-xs tabular-nums">{move || base.with(Vec::len)}</span>
            </button>
            {sidebar}
          </div>
        </aside>

        <div class="min-w-0">
          <div class="-mx-4 mb-4 flex gap-2 overflow-x-auto px-4 pb-1 md:hidden">{chips}</div>
          <div class="mb-4 grid grid-cols-2 gap-3 sm:grid-cols-4">
            <Stat label=t("knowledge.total-terms") value=Signal::derive(move || base.with(Vec::len)) />
            <Stat label=t("knowledge.english-abbreviations") value=abbr_count />
            <Stat label=t("knowledge.term-categories") value=category_total />
            <Stat label=t("exam.current-filter") value=Signal::derive(move || filtered.with(Vec::len)) />
          </div>
          {list}
        </div>
      </div>
    </div>
  }
}
