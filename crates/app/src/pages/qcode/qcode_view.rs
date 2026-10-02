use ham_web_core::glossary::GlossaryEntry;
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::icons::{Icon, IconKind};
use crate::pages::SLANG_CATEGORY;
use crate::ui::Stat;

use super::qcode_quiz::QCodeQuiz;
use super::qcode_reverse_quiz::QCodeReverseQuiz;
use super::slang_card::SlangCard;
use crate::i18n::t;

/// 简语分组。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlangGroup {
  /// Q 简语（QRA、QRM、QSL…）。
  QCode,
  /// 通联缩语（73、DE、CQ、RST…）。
  Abbrev,
  /// 通联用语（字母解释法、莫尔斯电码…）。
  Phrase,
}

impl SlangGroup {
  const fn label(self) -> &'static str {
    match self {
      Self::QCode => "Q 简语",
      Self::Abbrev => "通联缩语",
      Self::Phrase => "通联用语",
    }
  }

  const fn hint(self) -> &'static str {
    match self {
      Self::QCode => "ITU 三字母简语，可兼作问句或陈述句",
      Self::Abbrev => "CW 与话音通联常用缩语、信号报告",
      Self::Phrase => "通联中常用的语音拼读与电码",
    }
  }
}

const GROUPS: &[SlangGroup] = &[SlangGroup::QCode, SlangGroup::Abbrev, SlangGroup::Phrase];

/// 按词条归入对应分组。
fn group_of(entry: &GlossaryEntry) -> SlangGroup {
  let term = entry.term.as_str();
  if term.len() == 3 && term.starts_with('Q') && term.is_ascii() {
    SlangGroup::QCode
  } else if term.is_ascii() {
    SlangGroup::Abbrev
  } else {
    SlangGroup::Phrase
  }
}

/// 是否属于「简语」中指定分组的词条。
fn is_slang(entry: &GlossaryEntry, group: SlangGroup) -> bool {
  entry.category_key() == SLANG_CATEGORY && group_of(entry) == group
}

/// 在全部词条中筛选指定分组、且匹配关键字 `q`（已转小写）的词条下标；
/// `common_only` 为真时只保留「常用」词条。
fn matches(entries: &[GlossaryEntry], q: &str, group: SlangGroup, common_only: bool) -> Vec<usize> {
  entries
    .iter()
    .enumerate()
    .filter(|(_, e)| is_slang(e, group))
    .filter(|(_, e)| !common_only || e.common)
    .filter_map(|(i, e)| e.match_rank(q).map(|_| i))
    .collect()
}

/// 「常用 / 全部」切换按钮的样式。
fn scope_pill(active: bool) -> &'static str {
  if active {
    "inline-flex items-center whitespace-nowrap rounded-md px-2.5 py-1 text-xs font-medium text-primary-foreground bg-primary transition-colors"
  } else {
    "inline-flex items-center whitespace-nowrap rounded-md px-2.5 py-1 text-xs font-medium text-muted-foreground transition-colors hover:bg-accent"
  }
}

/// 简语速查主体：分组列表、搜索与「常用 / 全部」切换。
#[component]
pub(crate) fn QCodeView(entries: &'static [GlossaryEntry]) -> impl IntoView {
  let query = use_query_map();
  let kw = RwSignal::new(String::new());
  // 默认展示「常用」，可切换为「全部」
  let common_only = RwSignal::new(true);

  // 支持 /q-code?q=QRM 直接搜索
  Effect::new(move |_| {
    if let Some(q) = query.with(|q| q.get("q")) {
      kw.set(q);
    }
  });

  let qcode = Memo::new(move |_| {
    let q = kw.with(|k| k.trim().to_lowercase());
    matches(entries, &q, SlangGroup::QCode, common_only.get())
  });
  let abbrev = Memo::new(move |_| {
    let q = kw.with(|k| k.trim().to_lowercase());
    matches(entries, &q, SlangGroup::Abbrev, common_only.get())
  });
  let phrase = Memo::new(move |_| {
    let q = kw.with(|k| k.trim().to_lowercase());
    matches(entries, &q, SlangGroup::Phrase, common_only.get())
  });
  let lists = [qcode, abbrev, phrase];

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-16 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("简语速查")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("Q 简语 · CW 缩语 · 通联常用语")}</div>
          </div>

          <div class="flex items-center gap-0.5 rounded-lg border p-0.5">
            <button
              type="button"
              on:click=move |_| common_only.set(true)
              class=move || scope_pill(common_only.get())
            >
              {move || t("常用")}
            </button>
            <button
              type="button"
              on:click=move |_| common_only.set(false)
              class=move || scope_pill(!common_only.get())
            >
              {move || t("全部")}
            </button>
          </div>

          <div class="relative">
            <Icon
              kind=IconKind::Search
              class="pointer-events-none absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground"
            />
            <input
              prop:value=move || kw.get()
              on:input=move |e| kw.set(event_target_value(&e))
              placeholder=move || t("搜索简语 / 含义…")
              class="h-9 w-56 rounded-lg border bg-background pl-8 pr-8 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            />
            {move || {
              kw.with(|k| !k.is_empty())
                .then(|| {
                  view! {
                    <button
                      type="button"
                      aria-label=move || t("清除")
                      class="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                      on:click=move |_| kw.set(String::new())
                    >
                      <Icon kind=IconKind::X class="h-4 w-4" />
                    </button>
                  }
                })
            }}
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl px-4 py-5">
        <div class="mb-6 grid grid-cols-2 gap-3 sm:grid-cols-4">
          <Stat label=t("Q 简语") value=Signal::derive(move || qcode.with(Vec::len)) />
          <Stat label=t("通联缩语") value=Signal::derive(move || abbrev.with(Vec::len)) />
          <Stat label=t("通联用语") value=Signal::derive(move || phrase.with(Vec::len)) />
          <Stat
            label=t("当前匹配")
            value=Signal::derive(move || lists.iter().map(|l| l.with(Vec::len)).sum::<usize>())
          />
        </div>

        <QCodeQuiz entries=entries />

        <QCodeReverseQuiz entries=entries />

        {GROUPS
          .iter()
          .zip(lists)
          .map(|(&group, list)| {
            view! {
              <section class="mb-8 last:mb-0">
                <div class="mb-3 flex flex-wrap items-baseline gap-x-2 gap-y-1">
                  <h2 class="text-sm font-semibold">{move || t(group.label())}</h2>
                  <span class="text-xs text-muted-foreground">{move || t(group.hint())}</span>
                  <span class="ml-auto text-xs tabular-nums text-muted-foreground">
                    {move || list.with(Vec::len)}
                  </span>
                </div>
                {move || {
                  list.with(|items| {
                    if items.is_empty() {
                      view! {
                        <div class="rounded-xl border border-dashed py-10 text-center text-sm text-muted-foreground">
                          {move || t("没有匹配的简语")}
                        </div>
                      }
                      .into_any()
                    } else {
                      view! {
                        <div class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
                          {items
                            .iter()
                            .map(|&i| view! { <SlangCard entry=&entries[i] /> })
                            .collect_view()}
                        </div>
                      }
                      .into_any()
                    }
                  })
                }}
              </section>
            }
          })
          .collect_view()}
      </div>
    </div>
  }
}
