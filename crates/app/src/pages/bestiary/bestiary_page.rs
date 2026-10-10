use std::collections::BTreeMap;

use ham_web_core::categories::top_of;
use ham_web_core::mistake_book::MistakeRecord;
use leptos::prelude::*;

use crate::components::common::{EmptyState, PageContainer, PageHeader};
use crate::i18n::{t, tf};
use crate::icons::IconKind;
use crate::study;
use crate::ui::{Button, ButtonLink, Chip, ChipGroup, Size, Variant};
use crate::util::{encode_uri_component, now_ms, set_title};

use super::bestiary_summary::BestiarySummary;
use super::monster_card::MonsterCard;
use super::topic_bars::TopicBars;

/// 一次最多渲染多少只怪物：错题本可能上千条，一次全画出来既慢又没人看得完。
const PAGE: usize = 24;

/// 没有分类码的题归到这一类。key 用非空哨兵，避免与「全部专题」的空串撞车
/// （见 [`crate::rpg::OTHER_TOPIC`]）。
const OTHER: (&str, &str) = (crate::rpg::OTHER_TOPIC, "rpg.topic-other");

/// 错题所属的一级专题 `(key, 名称或词条 key)`。
fn topic_of(r: &MistakeRecord) -> (&'static str, &'static str) {
  r.question
    .p_code()
    .and_then(top_of)
    .map_or(OTHER, |t| (t.key, t.name))
}

/// 怪物图鉴。
#[component]
pub fn BestiaryPage() -> impl IntoView {
  set_title("rpg.bestiary");
  let now = now_ms();
  let mut records = study::load_book().records;
  // 到期的在前（`false < true`，所以取反），同样到期状态再按到期时间：最该复仇的排最上面。
  records.sort_by_key(|r| (!r.is_due(now), r.due_ms));
  let defeated = crate::rpg::load_bestiary().count();
  let due = records.iter().filter(|r| r.is_due(now)).count();
  let alive = records.len();

  let mut counts: BTreeMap<&'static str, (&'static str, usize)> = BTreeMap::new();
  for r in &records {
    let (key, name) = topic_of(r);
    counts.entry(key).or_insert((name, 0)).1 += 1;
  }
  let mut rows: Vec<(&'static str, &'static str, usize)> = counts
    .into_iter()
    .map(|(key, (name, n))| (key, name, n))
    .collect();
  rows.sort_by_key(|&(_, _, n)| std::cmp::Reverse(n));

  let records = StoredValue::new(records);
  let filter = RwSignal::new(String::new());
  let shown = RwSignal::new(PAGE);
  // 换了专题就回到第一页，否则「显示更多」留下的长列表会让新筛选结果看起来被截断。
  Effect::new(move |_| {
    filter.track();
    shown.set(PAGE);
  });

  let revenge_href = Signal::derive(move || {
    let topic = filter.get();
    if topic.is_empty() {
      "/battle?mode=revenge".to_owned()
    } else {
      format!(
        "/battle?mode=revenge&topic={}",
        encode_uri_component(&topic)
      )
    }
  });

  let actions = ViewFn::from(move || {
    view! {
      {(alive > 0)
        .then(|| {
          view! {
            <ButtonLink href=revenge_href variant=Variant::Default size=Size::Default>
              {move || t("rpg.revenge-all")}
            </ButtonLink>
          }
        })}
      <ButtonLink href="/mistakes" variant=Variant::Outline size=Size::Default>
        {move || t("rpg.classic-list")}
      </ButtonLink>
    }
  });

  let chips = {
    let rows = rows.clone();
    move || {
      view! {
        <ChipGroup
          value=filter
          on_change=Callback::new(move |v: String| filter.set(v))
          aria_label=Signal::derive(move || t("rpg.filter-topic"))
        >
          <Chip value="">{move || t("rpg.topic-all")}</Chip>
          {rows
            .iter()
            .map(|&(key, name, n)| {
              view! { <Chip value=key>{move || format!("{} ({n})", t(name))}</Chip> }
            })
            .collect_view()}
        </ChipGroup>
      }
    }
  };

  let grid = move || {
    let topic = filter.get();
    let limit = shown.get();
    let (cards, total) = records.with_value(|all| {
      let matched: Vec<&MistakeRecord> = all
        .iter()
        .filter(|r| topic.is_empty() || topic_of(r).0 == topic)
        .collect();
      let total = matched.len();
      let cards = matched
        .into_iter()
        .take(limit)
        .map(|r| {
          let (key, name) = topic_of(r);
          view! { <MonsterCard record=r.clone() top_key=key top_name=name now=now /> }
        })
        .collect_view();
      (cards, total)
    });
    view! {
      <ul class="grid gap-3 sm:grid-cols-2">{cards}</ul>
      {(total > limit)
        .then(|| {
          view! {
            <div class="flex justify-center">
              <Button
                variant=Variant::Outline
                size=Size::Default
                on_click=Callback::new(move |()| shown.update(|n| *n += PAGE))
              >
                {move || tf("rpg.show-more", &[&(total - limit).to_string()])}
              </Button>
            </div>
          }
        })}
    }
  };

  view! {
    <PageHeader
      title=Signal::derive(move || t("rpg.bestiary"))
      subtitle=Signal::derive(move || t("rpg.bestiary-subtitle"))
      actions=actions
    />
    <PageContainer>
      {if alive == 0 && defeated == 0 {
        view! {
          <EmptyState
            title=t("rpg.dex-empty")
            description=t("rpg.dex-empty-hint")
            icon=IconKind::Trophy
            actions=ViewFn::from(|| {
              view! {
                <ButtonLink href="/map" variant=Variant::Default size=Size::Default>
                  {move || t("rpg.to-map")}
                </ButtonLink>
              }
            })
          />
        }
          .into_any()
      } else {
        view! {
          <BestiarySummary alive=alive defeated=defeated due=due />
          {(!rows.is_empty())
            .then(|| {
              let bars: Vec<(&'static str, usize)> = rows
                .iter()
                .take(5)
                .map(|&(_, name, n)| (name, n))
                .collect();
              view! { <TopicBars rows=bars /> }
            })}
          {(alive > 0).then(|| view! { {chips()} {grid} })}
        }
          .into_any()
      }}
    </PageContainer>
  }
}
