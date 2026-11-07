use std::collections::HashMap;

use ham_web_core::categories::TOP_CATEGORIES;
use ham_web_core::glossary::{GlossaryEntry, OTHER_CATEGORY};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_query_map;

use crate::components::common::{EmptyState, PageContainer, PageHeader, StickyPanel};
use crate::components::sidebar_item::SidebarItem;
use crate::icons::{Icon, IconKind};
use crate::pages::SLANG_CATEGORY;
use crate::ui::{Button, ChipToggle, Input, InputType, MobileNavDrawer, Tooltip, Variant};
use crate::util::window;

use super::hits::{self, HitMap};
use super::stat::Stat;
use super::term_card::TermCard;
use super::{ALL, Haystacks, PAGE, category_meta, category_pos, is_countable};
use crate::i18n::{t, tf};

/// 术语表主体：搜索、分类筛选与词条列表。
#[component]
pub(crate) fn GlossaryView(entries: &'static [GlossaryEntry]) -> impl IntoView {
  let query = use_query_map();

  let kw = RwSignal::new(String::new());
  let cat = RwSignal::new(ALL.to_owned());
  let abbr_only = RwSignal::new(false);
  let visible = RwSignal::new(PAGE);
  let haystacks: RwSignal<Option<Haystacks>> = RwSignal::new(None);
  // 已统计的「术语 → 各题库出现题数」（卡片底栏读它）。
  let hit_map: RwSignal<HitMap> = RwSignal::new(HitMap::new());

  // 搜索框绑 `kw`，筛选按防抖后的副本算（见 `debounced`）；程序写入（地址、跳转）两份一起写。
  let kw_settled = crate::debounced::use_debounced(kw, crate::debounced::SEARCH_DEBOUNCE);

  // 支持 /glossary?q=驻波比 直接定位
  Effect::new(move |_| {
    if let Some(q) = query.with(|q| q.get("q")) {
      kw_settled.set(q.clone());
      kw.set(q);
      cat.set(ALL.to_owned());
      crate::debounced::set_if_changed(visible, PAGE);
    }
  });

  // 题库已在内存里时这段是同步返回的：缓存命中就当场把上次的结果交给卡片（再进来不再闪「统计中」）；
  // 没命中时逐题拼接检索文本要几十毫秒，放到首帧画出之后再做，不挡页面出来。
  spawn_local(async move {
    let banks = hits::load_banks().await;
    if let Some((h, cached)) = hits::cached(&banks) {
      hit_map.try_set(cached);
      haystacks.try_set(Some(h));
      return;
    }
    crate::after_paint::after_next_paint(move || {
      haystacks.try_set(Some(hits::build(banks)));
    });
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
    let q = kw_settled.with(|k| k.trim().to_lowercase());
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

  // 统计当前显示的卡片里还没统计过的术语：首帧之后每帧一批（`hits::BATCH`）。
  // 列表换了（搜索 / 筛选 / 加载更多）就开新一轮，旧一轮在下一帧发现代号不对自行停下。
  let round = StoredValue::new(0u32);
  Effect::new(move |_| {
    let Some(h) = haystacks.get() else { return };
    let shown: Vec<&'static str> = filtered.with(|f| {
      f.iter()
        .take(visible.get())
        .map(|&i| entries[i].term.as_str())
        .filter(|term| is_countable(term))
        .collect()
    });
    let pending: Vec<&'static str> =
      hit_map.with_untracked(|m| shown.into_iter().filter(|t| !m.contains_key(t)).collect());
    if pending.is_empty() {
      return;
    }
    let this = round.get_value().wrapping_add(1);
    round.set_value(this);
    let mut queue = pending.into_iter();
    crate::after_paint::each_frame(move || {
      if round.try_get_value() != Some(this) {
        return false;
      }
      let batch: Vec<_> = queue
        .by_ref()
        .take(hits::BATCH)
        .map(|term| (term, hits::count(&h, term)))
        .collect();
      if batch.is_empty() {
        return false;
      }
      hit_map.try_update(|m| m.extend(batch)).is_some()
    });
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
    crate::debounced::set_if_changed(visible, PAGE);
  };
  // 移动端分类抽屉的开关：选中分类后要主动收起。
  let cat_open = RwSignal::new(false);
  let jump_to = Callback::new(move |term: String| {
    kw_settled.set(term.clone());
    kw.set(term);
    cat.set(ALL.to_owned());
    crate::debounced::set_if_changed(visible, PAGE);
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
        let (name, cat_tone) = category_meta(key);
        view! {
          <SidebarItem
            active=Signal::derive(move || cat.with(|c| c == key))
            on_click=Callback::new(move |_| {
              select_cat(key);
              cat_open.set(false);
            })
          >
            <span class=format!("size-2 shrink-0 {}", cat_tone.map_or("bg-muted", crate::tone::dot))></span>
            // 分类名来自 `crates/core` 的中文常量，必须走 `t` 反查（中文原文 → 词条）；
            // 直接渲染 `name` 会让英文 / 西文界面的侧栏永远停在中文。
            <Tooltip class="min-w-0 flex-1" label=Signal::derive(move || crate::data::core_text_tracked(name))>
              <span class="w-full leading-snug">{move || crate::data::core_text_tracked(name)}</span>
            </Tooltip>
            <span class="ml-auto text-xs tabular-nums">{move || cat_count.with(|m| m.get(key).copied().unwrap_or(0))}</span>
          </SidebarItem>
        }
      })
      .collect_view()
  };

  // 分类列表（桌面侧栏与移动端抽屉共用）：全部 + 各分类。
  let category_tree = move || {
    view! {
      <SidebarItem
        active=Signal::derive(move || cat.with(|c| c == ALL))
        on_click=Callback::new(move |_| {
          select_cat(ALL);
          cat_open.set(false);
        })
      >
        <span class="h-2 w-2 bg-foreground/60"></span>
        {move || t("knowledge.all-terms")}
        <span class="ml-auto text-xs tabular-nums">{move || base.with(Vec::len)}</span>
      </SidebarItem>
      {sidebar}
    }
  };

  let list = move || {
    let total = filtered.with(Vec::len);
    if total == 0 {
      return view! {
        <EmptyState
          icon=IconKind::Search
          title=Signal::derive(move || t("knowledge.no-matching-terms"))
        />
      }
      .into_any();
    }
    let shown: Vec<usize> = filtered.with(|f| f.iter().copied().take(visible.get()).collect());
    let shown_len = shown.len();
    let cards = shown
      .into_iter()
      .map(|i| view! { <TermCard entry=&entries[i] hits=hit_map on_jump=jump_to /> })
      .collect_view();
    view! {
      <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">{cards}</div>
      {(shown_len < total)
        .then(|| {
          view! {
            <Button
              variant=Variant::Outline
              class="mt-4 w-full"
              on_click=Callback::new(move |_| visible.update(|v| *v += PAGE))
            >
              {move || tf("knowledge.load-more-count", &[&shown_len.to_string(), &total.to_string()])}
            </Button>
          }
        })}
    }
    .into_any()
  };

  // 现代风：文章页头 + 搜索/筛选条 + 术语卡两列。分类从侧栏改成横向胶囊，
  // 窄屏不用再开抽屉（原来侧栏在 <md 时是隐藏的，等于手机上没法切分类）。
  let modern_list = move || {
    let total = filtered.with(Vec::len);
    if total == 0 {
      return view! {
        <p class="md-palette-empty">{move || t("knowledge.no-matching-terms")}</p>
      }
      .into_any();
    }
    let shown: Vec<usize> = filtered.with(|f| f.iter().copied().take(visible.get()).collect());
    let shown_len = shown.len();
    view! {
      <div class="md-term-grid">
        {shown
          .into_iter()
          .map(|i| view! { <TermCard entry=&entries[i] hits=hit_map on_jump=jump_to /> })
          .collect_view()}
      </div>
      {(shown_len < total)
        .then(|| {
          view! {
            <button
              type="button"
              class="md-btn md-btn-outline mt-4 w-full"
              on:click=move |_| visible.update(|v| *v += PAGE)
            >
              {move || tf("knowledge.load-more-count", &[&shown_len.to_string(), &total.to_string()])}
            </button>
          }
        })}
    }
      .into_any()
  };

  if crate::kit::is_modern_at_mount() {
    return view! {
      <div class="md-root">
        <div class="md-page md-article-page">
          <crate::kit_modern::ModernPageHead
            title=Signal::derive(move || t("shell.glossary"))
            subtitle=Signal::derive(move || t("knowledge.common-ham-terms-english"))
          />

          <div class="md-glossary-bar">
            <crate::kit_modern::ModernInput
              value=kw
              on_input=Callback::new(move |v: String| {
                kw.set(v);
                crate::debounced::set_if_changed(visible, PAGE);
              })
              kind="search"
              placeholder=Signal::derive(move || t("knowledge.search-terms-abbreviations-explanations"))
              aria_label=Signal::derive(move || t("knowledge.search-terms-abbreviations-explanations"))
              class="md-glossary-search"
            />
            <button
              type="button"
              class="md-chip"
              aria-pressed=move || abbr_only.get().to_string()
              on:click=move |_| {
                abbr_only.update(|v| *v = !*v);
                crate::debounced::set_if_changed(visible, PAGE);
              }
            >
              {move || t("knowledge.english-abbreviations-only")}
            </button>
          </div>

          // `>=768`：分类胶囊行（桌面一屏铺得下）。
          <div class="md-menu-chips">
            <div class="md-chip-row mb-4">
              <button
                type="button"
                class="md-chip"
                aria-pressed=move || (cat.with(|c| c == ALL)).to_string()
                on:click=move |_| select_cat(ALL)
              >
                {move || t("knowledge.all-terms")}
                <span class="md-menu-count">{move || base.with(Vec::len)}</span>
              </button>
              {move || {
                categories()
                  .into_iter()
                  .map(|key| {
                    let (name, _) = category_meta(key);
                    view! {
                      <button
                        type="button"
                        class="md-chip"
                        aria-pressed=move || cat.with(|c| c == key).to_string()
                        on:click=move |_| select_cat(key)
                      >
                        {move || crate::data::core_text_tracked(name)}
                        <span class="md-menu-count">
                          {move || cat_count.with(|m| m.get(key).copied().unwrap_or(0))}
                        </span>
                      </button>
                    }
                  })
                  .collect_view()
              }}
            </div>
          </div>

          // `<768`：同一份分类收进右下角悬浮按钮的抽屉（与题库分类、工具目录同一种交互）。
          <crate::kit_modern::ModernMenuDrawer
            open=cat_open
            title=Signal::derive(move || t("knowledge.term-categories"))
            button_label=Signal::derive(move || t("knowledge.term-categories"))
          >
            <ul class="md-drawer-list">
              <li>
                <button
                  type="button"
                  class="md-drawer-item"
                  aria-pressed=move || (cat.with(|c| c == ALL)).to_string()
                  on:click=move |_| {
                    select_cat(ALL);
                    cat_open.set(false);
                  }
                >
                  {move || t("knowledge.all-terms")}
                  <span class="md-menu-count">{move || base.with(Vec::len)}</span>
                </button>
              </li>
              {move || {
                categories()
                  .into_iter()
                  .map(|key| {
                    let (name, _) = category_meta(key);
                    view! {
                      <li>
                        <button
                          type="button"
                          class="md-drawer-item"
                          aria-pressed=move || cat.with(|c| c == key).to_string()
                          on:click=move |_| {
                            select_cat(key);
                            cat_open.set(false);
                          }
                        >
                          {move || crate::data::core_text_tracked(name)}
                          <span class="md-menu-count">
                            {move || cat_count.with(|m| m.get(key).copied().unwrap_or(0))}
                          </span>
                        </button>
                      </li>
                    }
                  })
                  .collect_view()
              }}
            </ul>
          </crate::kit_modern::ModernMenuDrawer>

          <div class="md-strip mb-6">
            <div class="md-strip-item">
              <crate::kit_modern::ModernStatValue value=Signal::derive(move || {
                  Some(base.with(Vec::len))
              }) />
              <span class="md-strip-label">{move || t("knowledge.total-terms")}</span>
            </div>
            <div class="md-strip-item">
              <crate::kit_modern::ModernStatValue value=Signal::derive(move || {
                  Some(abbr_count.get())
              }) />
              <span class="md-strip-label">{move || t("knowledge.english-abbreviations")}</span>
            </div>
            <div class="md-strip-item">
              <crate::kit_modern::ModernStatValue value=Signal::derive(move || {
                  Some(category_total.get())
              }) />
              <span class="md-strip-label">{move || t("knowledge.term-categories")}</span>
            </div>
            // 第四格「当前筛选」删掉了：默认筛选下它恒等于「术语总数」，
            // 两个一样的数字并排只是占位。
          </div>

          {modern_list}
        </div>
      </div>
    }
    .into_any();
  }

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.glossary")
        subtitle=move || t("knowledge.common-ham-terms-english")
        actions=ViewFn::from(move || {
          view! {
            <Input
              value=kw
              on_change=Callback::new(move |v: String| {
                kw.set(v);
                crate::debounced::set_if_changed(visible, PAGE);
              })
              kind=InputType::Search
              placeholder=Signal::derive(move || t("knowledge.search-terms-abbreviations-explanations"))
              prefix=move || view! { <Icon kind=IconKind::Search /> }
              clearable=true
              class="w-56"
            />

            <ChipToggle
              active=abbr_only
              on_change=Callback::new(move |v: bool| {
                abbr_only.set(v);
                crate::debounced::set_if_changed(visible, PAGE);
              })
            >
              {view! { {move || t("knowledge.english-abbreviations-only")} }.into_any()}
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
          title=Signal::derive(move || t("knowledge.term-categories"))
          button_label=Signal::derive(move || t("knowledge.term-categories"))
          icon=IconKind::SlidersHorizontal
          class="md:hidden"
        >
          {category_tree()}
        </MobileNavDrawer>

        <div class="min-w-0">
          <div class="mb-4 grid grid-cols-2 gap-3 sm:grid-cols-4">
            <Stat label=Signal::derive(move || t("knowledge.total-terms")) value=Signal::derive(move || base.with(Vec::len)) />
            <Stat label=Signal::derive(move || t("knowledge.english-abbreviations")) value=abbr_count />
            <Stat label=Signal::derive(move || t("knowledge.term-categories")) value=category_total />
            <Stat label=Signal::derive(move || t("exam.current-filter")) value=Signal::derive(move || filtered.with(Vec::len)) />
          </div>
          {list}
        </div>
      </PageContainer>
    </div>
  }
  .into_any()
}
