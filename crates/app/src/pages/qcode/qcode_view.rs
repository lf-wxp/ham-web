use ham_web_core::glossary::GlossaryEntry;
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::components::common::{EmptyState, PageContainer, PageHeader};
use crate::icons::{Icon, IconKind};
use crate::pages::SLANG_CATEGORY;
use crate::ui::{Chip, ChipGroup, Input, InputType, Stat, card_title_class};

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
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.q-codes-title")
        subtitle=move || t("knowledge.q-codes-cw-abbreviations")
        actions=ViewFn::from(move || {
          view! {
            <ChipGroup
              value=Signal::derive(move || if common_only.get() { "common" } else { "all" }.to_owned())
              on_change=Callback::new(move |v: String| common_only.set(v == "common"))
              aria_label=Signal::derive(move || t("morse.scope"))
            >
              <Chip value="common".to_owned()>
                {view! { {move || t("knowledge.common")} }.into_any()}
              </Chip>
              <Chip value="all".to_owned()>
                {view! { {move || t("exam.all")} }.into_any()}
              </Chip>
            </ChipGroup>

            <Input
              value=kw
              on_change=Callback::new(move |v: String| kw.set(v))
              kind=InputType::Search
              placeholder=Signal::derive(move || t("knowledge.search-code-meaning"))
              prefix=move || view! { <Icon kind=IconKind::Search /> }
              clearable=true
              class="w-56"
            />
          }
        })
      />

      <PageContainer class="space-y-0">
        <div class="mb-6 grid grid-cols-2 gap-3 sm:grid-cols-4">
          <Stat label=t("knowledge.q-codes") value=Signal::derive(move || qcode.with(Vec::len)) />
          <Stat label=t("knowledge.contact-abbreviations") value=Signal::derive(move || abbrev.with(Vec::len)) />
          <Stat label=t("knowledge.contact-phrases") value=Signal::derive(move || phrase.with(Vec::len)) />
          <Stat
            label=t("knowledge.current-matches")
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
                  <h2 class=card_title_class("")>{move || t(group.label())}</h2>
                  <span class="text-xs text-muted-foreground">{move || t(group.hint())}</span>
                  <span class="ml-auto text-xs tabular-nums text-muted-foreground">
                    {move || list.with(Vec::len)}
                  </span>
                </div>
                {move || {
                  list.with(|items| {
                    if items.is_empty() {
                      view! {
                        <EmptyState
                          icon=IconKind::Search
                          title=Signal::derive(move || t("knowledge.no-matching-codes"))
                        />
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
      </PageContainer>
    </div>
  }
}
