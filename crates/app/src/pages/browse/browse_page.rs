use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use ham_web_core::categories::{self, TOP_CATEGORIES};
use ham_web_core::{Bank, fingerprint};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_query_map;

use crate::cn::cn;
use crate::components::common::{
  EmptyState, LoadMore, PageContainer, PageHeader, Skeleton, StickyPanel,
};
use crate::components::sidebar_item::SidebarItem;
use crate::data::{self, Questions};
use crate::icons::{Icon, IconKind};
use crate::store;
use crate::tone;
use crate::ui::{Chip, ChipGroup, ChipToggle, Input, InputType, MobileNavDrawer, Tooltip};
use crate::util::set_title;

use super::question_row::QuestionRow;
use super::stat::Stat;
use super::{ALL, FIRST_ROWS, OTHER, PAGE, ROW_STEP, sub_name_of, top_of};
use crate::i18n::{bank_class, t};

#[component]
pub fn BrowsePage() -> impl IntoView {
  set_title("shell.browse-questions");
  // 本页渲染 `crates/core` 的中文原文（`kt_tracked`），译文词典要自己挂一次拉取，
  // 否则以英文 / 西语直接打开本页时词典永远不来、这些文案停在中文。
  data::use_knowledge_i18n();
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
  // 收藏快照：每行从里面读自己那一项（`QuestionRow` 里的 Memo），切换时由行就地增删。
  let bookmarks = store::use_bookmark_snapshot();
  // 搜索框绑 `kw`（字立刻上屏），筛选按防抖后的副本算（见 `debounced`）。
  let kw_settled = crate::debounced::use_debounced(kw, crate::debounced::SEARCH_DEBOUNCE);
  // 进入本页的第一帧只建前 `FIRST_ROWS` 道题（手机一屏放得下三四道），画出来之后每帧补 `ROW_STEP` 道，
  // 补到 `visible` 为止：每道题带选项、解析、知识点与参考依据，一次建 40 道是这一页换页时最重的一块，
  // 一次补 32 道的那一帧也一样重。
  // 题库是异步到达的：数据到了才启动补齐（见下面的 `spawn_local`），骨架屏在场期间不推进。
  let row_cap = RwSignal::new(FIRST_ROWS);

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
      // 地址带来的关键词立刻生效（两份一起写），不走输入防抖。
      kw_settled.set(q.clone());
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
    crate::debounced::set_if_changed(visible, PAGE);
  });

  spawn_local(async move {
    // 合集在会话内缓存（`data::all_banks_merged`）：题库没变时第二次进来不再逐题深拷贝。
    let merged = data::all_banks_merged().await;
    // 页面在等数据期间被换走了（信号已释放）：不必再往下做。
    if all.try_set(merged).is_some() {
      return;
    }
    loading.try_set(false);
    // 行出现之后才开始补：补齐是一串每帧一步、有限步内必然结束的循环（见 `grow_cap`），
    // 不能在数据没到时就先启动 —— 那样上限会在列表出现之前走完，数据一到就是一整页一次建完。
    crate::after_paint::grow_cap(row_cap, ROW_STEP, move || {
      visible.try_get_untracked().unwrap_or(0)
    });
  });

  // 各题库题目指纹（用于「只看本类新增」）。`Memo` 是惰性的：只有下面真的用到时才逐题计算，
  // 不勾「只看本类新增」就一次都不算（每次进入本页逐题算指纹，在手机上要几十毫秒）。
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
      let in_bank = qs
        .iter()
        .enumerate()
        .filter(|(_, q)| Bank::of_id(q.id_str()) == b);
      // A 类没有「更低的题库」可比，不勾「只看本类新增」时也不需要指纹。
      if !unique || b == Bank::A {
        return in_bank.map(|(i, _)| i).collect::<Vec<_>>();
      }
      fingerprints.with(|fps| {
        let has = |bank: Bank, f: &str| fps.get(&bank).is_some_and(|s| s.contains(f));
        in_bank
          .filter(|(_, q)| {
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

  // 每道题的检索文本（题干 + 选项 + 解析拼接后转小写）：惰性 Memo，第一次输入关键词时才建，
  // 之后每次击键只做子串查找 —— 此前每键都要对本类 1000 多道题各拼接、转小写一遍。
  let search_texts = Memo::new(move |_| {
    all.with(|qs| Arc::new(qs.iter().map(|q| q.search_text()).collect::<Vec<String>>()))
  });

  let filtered = Memo::new(move |_| {
    let k = kw_settled.with(|k| k.trim().to_lowercase());
    let (t, s, multi) = (top.get(), sub.get(), multi_only.get());
    let texts = (!k.is_empty()).then(|| search_texts.get());
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
            texts.as_ref().is_none_or(|t| t[i].contains(&k))
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
                        "bg-accent text-foreground"
                      } else {
                        "text-muted-foreground hover:bg-accent/60"
                      },
                    ])
                  }
                >
                  <Tooltip class="min-w-0 flex-1" label=Signal::derive(move || crate::data::kt_tracked(name))>
                    <span class="w-full truncate">{move || crate::data::kt_tracked(name)}</span>
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
              <span class=format!("size-2 shrink-0 {}", tone::dot(c.tone))></span>
              // 分类名来自 `crates/core` 的中文常量，必须走 `t` 反查（中文原文 → 词条）；
              // 直接渲染 `c.name` 会让英文 / 西文界面的侧栏永远停在中文。
              <Tooltip class="min-w-0 flex-1" label=Signal::derive(move || crate::data::core_text_tracked(c.name))>
                <span class="w-full leading-snug">{move || crate::data::core_text_tracked(c.name)}</span>
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

  // 题目行：按「题库里的下标」键控的 `<For>`。补齐（`grow_cap`）、「加载更多」时已建好的行不重建，
  // 只追加新的 —— 此前每次都整体 `collect_view`，补一批就把前面的行全部丢掉重建，一批比一批重。
  // 换筛选 / 换题库时下标集合变了，`<For>` 按键增删，行里没有本地状态要保留。
  // 前提：`all` 在本页只写一次（上面的 `spawn_local` 里数据落地那一下）。键是 `all` 里的下标，
  // 若以后允许题库热更新替换 `all`，同一个下标会指向另一道题、`<For>` 却认为是同一行不重建 ——
  // 到时要把键换成题号（`id_str`），或在替换时让整块重建。
  let question_rows = move || {
    view! {
      <For
        each=move || {
          filtered.with(|f| {
            f.iter()
              .copied()
              .take(visible.get().min(row_cap.get()))
              .collect::<Vec<usize>>()
          })
        }
        key=|i| *i
        children=move |i| {
          let q = all.with(|qs| qs[i].clone());
          view! { <QuestionRow q=q bookmarks=bookmarks /> }
        }
      />
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
                <div class=crate::kit::card_pad_class()>
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
      return view! { <EmptyState title=Signal::derive(move || t("exam.no-matching-questions")) /> }.into_any();
    }
    // 这个闭包里**不读 `visible`**：读了的话每次「加载更多」整块重跑，`<For>` 也跟着重建，
    // 键控就白做了。「加载更多」按钮与它的计数在 `LoadMore` 自己的闭包里读。
    view! {
      <div class="space-y-4">{question_rows()}</div>
      <LoadMore visible=visible total=total step=PAGE />
    }
    .into_any()
  };

  // 现代风：分类从「可展开的树」改成两排胶囊（大类 + 选中的大类下的子类），
  // 窄屏不再需要抽屉；筛选与搜索同一条工具行。
  let modern_list = move || {
    if loading.get() {
      return view! {
        <div class="flex flex-col gap-3">
          <span class="sr-only" role="status">{move || t("exam.loading-the-question-bank")}</span>
          {(0..5)
            .map(|_| view! { <div class="md-skel" style="height: 108px; border-radius: var(--md-r-lg)"></div> })
            .collect_view()}
        </div>
      }
        .into_any();
    }
    let total = filtered.with(Vec::len);
    if total == 0 {
      // 筛到零条时给一条明确出口：把所有筛选一起清掉。
      // 只写一句「没有匹配的题目」，用户还得自己回头找是哪一项设的限。
      let reset = Callback::new(move |_| {
        kw.set(String::new());
        multi_only.set(false);
        unique_only.set(false);
        select_top(ALL);
        sub.set(None);
        crate::debounced::set_if_changed(visible, PAGE);
      });
      return view! {
        <EmptyState
          title=Signal::derive(move || t("exam.no-matching-questions"))
          actions=ViewFn::from(move || {
            view! {
              <button
                type="button"
                class="md-btn md-btn-outline mt-1"
                on:click=move |_| reset.run(())
              >
                {move || t("exam.all-questions")}
              </button>
            }
          })
        />
      }
      .into_any();
    }
    view! {
      <div class="flex flex-col gap-3">{question_rows()}</div>
      <LoadMore visible=visible total=total step=PAGE />
    }
    .into_any()
  };

  if crate::kit::is_modern_at_mount() {
    // 分类的两种宽度、两种控件，数据只有一份：
    //   · `>=768`：胶囊行（桌面一屏铺得下，点一下换一层）；
    //   · `<768`：右下角悬浮按钮 + 抽屉（`ModernMenuDrawer`），与工具目录、术语分类同一种交互。
    // 窄屏不能留胶囊行：三行胶囊换行时 9 个分类铺成 5 行，首屏就只剩筛选器。
    let type_chips = move || {
      view! {
        <button
          type="button"
          class="md-chip"
          aria-pressed=move || top.with(|t| t == ALL).to_string()
          on:click=move |_| select_top(ALL)
        >
          {move || t("exam.all-questions")}
          <span class="md-menu-count">{move || bank_questions.with(Vec::len)}</span>
        </button>
        {TOP_CATEGORIES
          .iter()
          .map(|c| {
            let key = c.key;
            view! {
              <button
                type="button"
                class="md-chip"
                aria-pressed=move || top.with(|t| t == key).to_string()
                on:click=move |_| select_top(key)
              >
                {move || crate::data::core_text_tracked(c.name)}
                <span class="md-menu-count">
                  {move || cat_count.with(|m| m.get(key).copied().unwrap_or(0))}
                </span>
              </button>
            }
          })
          .collect_view()}
      }
    };
    let sub_chips = move || {
      let current = top.get();
      view! {
        <button
          type="button"
          class="md-chip"
          aria-pressed=move || sub.with(Option::is_none).to_string()
          on:click=move |_| sub.set(None)
        >
          {move || t("exam.all-questions")}
        </button>
        {categories::sub_names_of(&current)
          .into_iter()
          .filter(|name| sub_count.with(|m| m.get(name).copied().unwrap_or(0)) > 0)
          .map(|name| {
            view! {
              <button
                type="button"
                class="md-chip md-chip-sm"
                aria-pressed=move || (sub.get() == Some(name)).to_string()
                on:click=move |_| {
                  sub.set(Some(name));
                  crate::debounced::set_if_changed(visible, PAGE);
                }
              >
                {move || crate::data::core_text_tracked(name)}
              </button>
            }
          })
          .collect_view()}
      }
    };
    // 抽屉内容：题型一层，子类**缩进挂在被选中那一类的下面**（不是另起一段）。
    // 选题型不关抽屉 —— 子类就在它下面；选「全部题目」或某个子类才收起。
    let drawer_tree = move || {
      view! {
        <ul class="md-drawer-list">
          <li>
            <button
              type="button"
              class="md-drawer-item"
              aria-pressed=move || top.with(|t| t == ALL).to_string()
              on:click=move |_| {
                select_top(ALL);
                cat_open.set(false);
              }
            >
              {move || t("exam.all-questions")}
              <span class="md-menu-count">{move || bank_questions.with(Vec::len)}</span>
            </button>
          </li>
          {TOP_CATEGORIES
            .iter()
            .map(|c| {
              let key = c.key;
              view! {
                <li>
                  <button
                    type="button"
                    class="md-drawer-item"
                    aria-pressed=move || top.with(|t| t == key).to_string()
                    // 已经有子类展开时再点一次：不会收起（那一类仍是当前分类），
                    // 只把子类选择清掉 —— 「本类全部」就是这个状态。
                    on:click=move |_| select_top(key)
                  >
                    {move || crate::data::core_text_tracked(c.name)}
                    <span class="md-menu-count">
                      {move || cat_count.with(|m| m.get(key).copied().unwrap_or(0))}
                    </span>
                  </button>
                  {move || {
                    (top.with(|t| t == key))
                      .then(|| {
                        view! {
                          <ul class="md-drawer-sublist">
                            {categories::sub_names_of(key)
                              .into_iter()
                              .filter(|name| {
                                sub_count.with(|m| m.get(name).copied().unwrap_or(0)) > 0
                              })
                              .map(|name| {
                                view! {
                                  <li>
                                    <button
                                      type="button"
                                      class="md-drawer-item"
                                      aria-pressed=move || (sub.get() == Some(name)).to_string()
                                      on:click=move |_| {
                                        sub.set(Some(name));
                                        crate::debounced::set_if_changed(visible, PAGE);
                                        cat_open.set(false);
                                      }
                                    >
                                      {move || crate::data::core_text_tracked(name)}
                                      <span class="md-menu-count">
                                        {move || sub_count
                                          .with(|m| m.get(name).copied().unwrap_or(0))}
                                      </span>
                                    </button>
                                  </li>
                                }
                              })
                              .collect_view()}
                          </ul>
                        }
                      })
                  }}
                </li>
              }
            })
            .collect_view()}
        </ul>
      }
    };

    return view! {
      <div class="md-root">
        <div class="md-page">
          <crate::kit_modern::ModernPageHead
            title=Signal::derive(move || t("shell.browse-questions"))
            subtitle=Signal::derive(move || t("exam.grouped-by-question-type"))
          />

          <div class="md-glossary-bar">
            <crate::kit_modern::ModernInput
              value=kw
              on_input=Callback::new(move |v: String| {
                kw.set(v);
                crate::debounced::set_if_changed(visible, PAGE);
              })
              kind="search"
              placeholder=Signal::derive(move || t("exam.search-question-answer-explanation"))
              aria_label=Signal::derive(move || t("exam.search-question-answer-explanation"))
              class="md-glossary-search"
            />
            // 题库选择是「这份列表的数据范围」，与搜索 / 多选 / 新增同一层：
            // 它是数据源、不是分类导航，所以不进下面的菜单条（像素风也放在页头）。
            // `md-chip-bar`：≥641 是 `display: contents`（胶囊仍与搜索框同处一条 flex-wrap 行，版式与以前一致）；
            // ≤640 变成独占一行的横滚条 —— 5 颗胶囊以前折成两行 98px，把首张题卡推到视口 60% 以下。
            <div class="md-chip-bar">
              {Bank::ALL
                .into_iter()
                .map(|b| {
                  view! {
                    <button
                      type="button"
                      class="md-chip"
                      aria-pressed=move || (bank.get() == b).to_string()
                      on:click=move |_| {
                        bank.set(b);
                        crate::debounced::set_if_changed(visible, PAGE);
                        select_top(ALL);
                      }
                    >
                      {move || bank_class(b.as_str())}
                    </button>
                  }
                })
                .collect_view()}
              <button
                type="button"
                class="md-chip"
                aria-pressed=move || multi_only.get().to_string()
                on:click=move |_| multi_only.update(|v| *v = !*v)
              >
                {move || t("exam.multiple-only")}
              </button>
              <button
                type="button"
                class="md-chip"
                aria-pressed=move || unique_only.get().to_string()
                on:click=move |_| unique_only.update(|v| *v = !*v)
              >
                {move || t("exam.new-in-this-class")}
              </button>
            </div>
          </div>

          // `>=768`：分类胶囊行（题型一层 + 选了题型后的子类一层）。
          <div class="md-menu-chips">
            <div class="md-chip-row mb-2">{type_chips()}</div>
            {move || {
              (top.with(|t| t != ALL))
                .then(|| view! { <div class="md-chip-row mb-4">{sub_chips()}</div> })
            }}
          </div>

          // `<768`：同一个分类树收进抽屉，按钮在右下角。
          <crate::kit_modern::ModernMenuDrawer
            open=cat_open
            title=Signal::derive(move || t("exam.question-types"))
            button_label=Signal::derive(move || t("exam.category"))
          >
            {drawer_tree()}
          </crate::kit_modern::ModernMenuDrawer>

          <div class="md-strip mb-5">
            <div class="md-strip-item">
              <crate::kit_modern::ModernStatValue value=Signal::derive(move || {
                  Some(bank_questions.with(Vec::len))
              }) />
              <span class="md-strip-label">{move || t("exam.total-questions")}</span>
            </div>
            <div class="md-strip-item">
              <crate::kit_modern::ModernStatValue value=Signal::derive(move || {
                  Some(multi_count.get())
              }) />
              <span class="md-strip-label">{move || t("exam.multiple-answer")}</span>
            </div>
            <div class="md-strip-item">
              <crate::kit_modern::ModernStatValue value=Signal::stored(Some(TOP_CATEGORIES.len())) />
              <span class="md-strip-label">{move || t("exam.question-types")}</span>
            </div>
            // 第四格「当前筛选」删掉了：默认筛选下它恒等于「总题数」，
            // 两个一样的数字并排只是占位。
          </div>

          {modern_list}
        </div>
      </div>
    }
    .into_any();
  }

  view! {
    <div class="min-h-svh animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
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
                  crate::debounced::set_if_changed(visible, PAGE);
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
                crate::debounced::set_if_changed(visible, PAGE);
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
          <StickyPanel class=crate::kit::card_class_with("max-h-[calc(100svh-var(--sticky-top)-16px)] overflow-y-auto overscroll-contain p-2")>
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
            <Stat label=Signal::derive(move || t("exam.total-questions")) value=Signal::derive(move || bank_questions.with(Vec::len)) />
            <Stat label=Signal::derive(move || t("exam.multiple-answer")) value=multi_count />
            <Stat label=Signal::derive(move || t("exam.question-types")) value=Signal::stored(TOP_CATEGORIES.len()) />
            <Stat label=Signal::derive(move || t("exam.current-filter")) value=Signal::derive(move || filtered.with(Vec::len)) />
          </div>
          {list}
        </div>
      </PageContainer>
    </div>
  }
  .into_any()
}
