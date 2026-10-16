use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use ham_web_core::categories::{self, TOP_CATEGORIES};
use ham_web_core::{Bank, fingerprint};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_query_map;

use crate::cn::cn;
use crate::components::common::{EmptyState, PageContainer, PageHeader, Skeleton, StickyPanel};
use crate::components::sidebar_item::SidebarItem;
use crate::data::{self, Questions};
use crate::icons::{Icon, IconKind};
use crate::ui::{
  Button, Chip, ChipGroup, ChipToggle, Input, InputType, MobileNavDrawer, Tooltip, Variant,
};
use crate::util::set_title;

use super::question_row::QuestionRow;
use super::stat::Stat;
use super::{ALL, OTHER, PAGE, sub_name_of, top_of};
use crate::i18n::{bank_class, t};

#[component]
pub fn BrowsePage() -> impl IntoView {
  set_title("shell.browse-questions");
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

  // 支持 /browse?bank=B&q=驻波比（术语表页跳转）、/browse?sub=驻波比 SWR（错题「同类题」跳转）
  let query = use_query_map();
  Effect::new(move |_| {
    let (b, q, s) = query.with(|p| (p.get("bank"), p.get("q"), p.get("sub")));
    if let Some(b) = b.and_then(|b| b.parse::<Bank>().ok()) {
      bank.set(b);
      top.set(ALL.to_owned());
      sub.set(None);
    }
    if let Some(q) = q {
      kw.set(q);
    }
    if let Some(s) = s
      && let Some(sc) = categories::SUB_CATEGORIES
        .iter()
        .find(|sc| sc.name == s.as_str())
    {
      top.set(sc.top.to_owned());
      sub.set(Some(sc.name));
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
  // 移动端分类抽屉的开关：选中分类后要主动收起。
  let cat_open = RwSignal::new(false);

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
                    cat_open.set(false);
                  }
                  class=move || {
                    cn(&[
                      "block w-full truncate px-2 py-1 text-left text-xs transition-colors",
                      if sub.get() == Some(name) {
                        "bg-accent font-medium text-foreground"
                      } else {
                        "text-muted-foreground hover:bg-accent/60"
                      },
                    ])
                  }
                >
                  <Tooltip class="min-w-0 flex-1" label=name.to_string()>
                    <span class="w-full truncate">{name}</span>
                  </Tooltip>
                  <span class="ml-1 opacity-60">{move || sub_count.with(|m| m.get(name).copied().unwrap_or(0))}</span>
                </button>
              }
            })
            .collect_view()
        };
        view! {
          <div>
            <SidebarItem
              active=Signal::derive(move || top.with(|t| t == c.key))
              on_click=Callback::new(move |_| {
                select_top(c.key);
                toggle_top(c.key);
                cat_open.set(false);
              })
            >
              <span class="h-2 w-2 shrink-0" style=format!("background: {}", c.color)></span>
              <Tooltip class="min-w-0 flex-1" label=c.name.to_string()>
                <span class="w-full truncate">{c.name}</span>
              </Tooltip>
              <span class="ml-auto text-xs tabular-nums">{move || cat_count.with(|m| m.get(c.key).copied().unwrap_or(0))}</span>
              <Icon
                kind=IconKind::ChevronDown
                class=Signal::derive(move || {
                  cn(&["h-3.5 w-3.5 shrink-0 transition-transform", if expanded() { "rotate-180" } else { "" }])
                })
              />
            </SidebarItem>
            {move || expanded().then(|| view! { <div class="mb-1 ml-4 border-l pl-2">{subs}</div> })}
          </div>
        }
      })
      .collect_view()
  };

  // 分类列表（桌面侧栏与移动端抽屉共用）：全部 + 各大类（含可展开的子类）。
  let category_tree = move || {
    view! {
      <SidebarItem
        active=Signal::derive(move || top.with(|t| t == ALL))
        on_click=Callback::new(move |_| {
          select_top(ALL);
          cat_open.set(false);
        })
      >
        <span class="h-2 w-2 bg-foreground/60"></span>
        {move || t("exam.all-questions")}
        <span class="ml-auto text-xs tabular-nums">{move || bank_questions.with(Vec::len)}</span>
      </SidebarItem>
      {sidebar}
    }
  };

  let list = move || {
    if loading.get() {
      // 骨架屏而不是 spinner：题库首屏一次铺开十几行，spinner 消失换成真实列表会有一次
      // 明显的高度跳变；骨架条按题目行的轮廓铺，跳变被摊平，也让「在加载」有确定感。
      // `sr-only` 的 status 节点保留读屏播报 —— 骨架条本身是 aria-hidden 的。
      return view! {
        <div class="space-y-4">
          <span class="sr-only" role="status">{move || t("exam.loading-the-question-bank")}</span>
          {(0..6)
            .map(|_| {
              view! {
                <div class="pxl-card border bg-card p-4">
                  <Skeleton class="h-3 w-20" />
                  <Skeleton class="mt-3 h-4 w-3/4" />
                  <Skeleton class="mt-2.5 h-4 w-1/2" />
                </div>
              }
            })
            .collect_view()}
        </div>
      }
      .into_any();
    }
    let total = filtered.with(Vec::len);
    if total == 0 {
      return view! { <EmptyState title=t("exam.no-matching-questions") /> }.into_any();
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
            <Button
              variant=Variant::Outline
              class="mt-4 w-full"
              on_click=Callback::new(move |_| visible.update(|v| *v += PAGE))
            >
              "加载更多（已显示 " {shown_len} " / " {total} "）"
            </Button>
          }
        })}
    }
    .into_any()
  };

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.browse-questions")
        subtitle=move || t("exam.grouped-by-question-type")
        actions=ViewFn::from(move || {
          view! {
            <ChipGroup
              value=Signal::derive(move || bank.get().as_str().to_string())
              on_change=Callback::new(move |v: String| {
                if let Ok(b) = v.parse::<Bank>() {
                  bank.set(b);
                  visible.set(PAGE);
                  select_top(ALL);
                }
              })
              aria_label=Signal::derive(move || t("exam.bank-class"))
            >
              {Bank::ALL
                .into_iter()
                .map(|b| {
                  let label = view! { {move || bank_class(b.as_str())} }.into_any();
                  view! { <Chip value=b.as_str().to_string()>{label}</Chip> }
                })
                .collect_view()}
            </ChipGroup>

            <Input
              value=kw
              on_change=Callback::new(move |v: String| {
                kw.set(v);
                visible.set(PAGE);
              })
              kind=InputType::Search
              placeholder=Signal::derive(move || t("exam.search-question-answer-explanation"))
              prefix=move || view! { <Icon kind=IconKind::Search /> }
              clearable=true
              class="w-56"
            />

            <ChipToggle
              active=multi_only
              on_change=Callback::new(move |v: bool| multi_only.set(v))
            >
              {view! { {move || t("exam.multiple-only")} }.into_any()}
            </ChipToggle>
            <ChipToggle
              active=unique_only
              on_change=Callback::new(move |v: bool| unique_only.set(v))
            >
              {view! { {move || t("exam.new-in-this-class")} }.into_any()}
            </ChipToggle>
          }
        })
      />

      <PageContainer class="space-y-0 grid grid-cols-1 gap-5 md:grid-cols-[240px_1fr]">
        <aside class="hidden md:block">
          <StickyPanel class="max-h-[calc(100vh-var(--sticky-top)-16px)] overflow-y-auto pxl-card border bg-card p-2">
            {category_tree()}
          </StickyPanel>
        </aside>

        <MobileNavDrawer
          open=cat_open
          title=Signal::derive(move || t("exam.question-types"))
          button_label=Signal::derive(move || t("exam.category"))
          icon=IconKind::SlidersHorizontal
          class="md:hidden"
        >
          {category_tree()}
        </MobileNavDrawer>

        <div class="min-w-0">
          <div class="mb-4 grid grid-cols-2 gap-3 sm:grid-cols-4">
            <Stat label=t("exam.total-questions") value=Signal::derive(move || bank_questions.with(Vec::len)) />
            <Stat label=t("exam.multiple-answer") value=multi_count />
            <Stat label=t("exam.question-types") value=Signal::stored(TOP_CATEGORIES.len()) />
            <Stat label=t("exam.current-filter") value=Signal::derive(move || filtered.with(Vec::len)) />
          </div>
          {list}
        </div>
      </PageContainer>
    </div>
  }
}
