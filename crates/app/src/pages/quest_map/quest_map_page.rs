use ham_web_core::Bank;
use ham_web_core::rpg::{Stage, StageState, stage_id, stage_states, stages_of};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;

use crate::components::common::{PageContainer, PageHeader};
use crate::data;
use crate::i18n::{bank_class, t, tf};
use crate::pages::{bank_href, use_bank_query};
use crate::rpg::load_stars;
use crate::ui::{Button, Chip, ChipGroup, Size, Variant};
use crate::util::{encode_uri_component, set_title};

use super::next_quest::NextQuest;
use super::stage_node::StageNode;

/// 地图数据：关卡 + 各关状态。
#[derive(Clone)]
struct MapData {
  stages: Vec<Stage>,
  states: Vec<StageState>,
}

/// 闯关地图。
#[component]
pub fn QuestMapPage() -> impl IntoView {
  set_title("rpg.quest-map");
  let (version, bank) = use_bank_query();
  let navigate = use_navigate();

  let map = RwSignal::new(None::<MapData>);
  let failed = RwSignal::new(false);
  let attempt = RwSignal::new(0u32);
  let generation = StoredValue::new(0u32);

  Effect::new(move |_| {
    let (v, b) = (version.get(), bank.get());
    attempt.track();
    generation.update_value(|g| *g += 1);
    let current = generation.get_value();
    map.set(None);
    failed.set(false);
    spawn_local(async move {
      let result = data::load_bank(v.as_deref(), b, true).await;
      if generation.try_get_value() != Some(current) {
        return;
      }
      match result {
        Ok(all) => {
          let stages = stages_of(&all);
          let order: Vec<String> = stages.iter().map(|s| stage_id(b.as_str(), s.key)).collect();
          let states = stage_states(&order, &load_stars());
          map.set(Some(MapData { stages, states }));
        }
        Err(_) => failed.set(true),
      }
    });
  });

  let switch_bank = Callback::new(move |value: String| {
    let b = Bank::from_param(Some(&value));
    if b == bank.get_untracked() {
      return;
    }
    navigate(
      &bank_href("/map", version.get_untracked().as_deref(), b),
      NavigateOptions {
        replace: true,
        ..Default::default()
      },
    );
  });

  let bank_chips = move || {
    view! {
      <ChipGroup
        value=Signal::derive(move || bank.get().as_str().to_owned())
        on_change=switch_bank
        aria_label=Signal::derive(move || t("exam.bank-class"))
      >
        {Bank::ALL
          .into_iter()
          .map(|b| view! { <Chip value=b.as_str()>{move || bank_class(b.as_str())}</Chip> })
          .collect_view()}
      </ChipGroup>
    }
  };

  let body = move || {
    if failed.get() {
      return view! {
        <section class="pxl-window space-y-3 p-6" role="alert">
          <p>{t("rpg.load-failed")}</p>
          <Button
            variant=Variant::Default
            size=Size::Default
            on_click=Callback::new(move |()| attempt.update(|n| *n += 1))
          >
            {t("rpg.retry")}
          </Button>
        </section>
      }
      .into_any();
    }
    let Some(data) = map.get() else {
      return view! { <div class="pxl-window p-6" aria-live="polite">{t("rpg.map-loading")}</div> }
        .into_any();
    };
    if data.stages.is_empty() {
      return view! { <div class="pxl-window p-6">{t("rpg.map-empty")}</div> }.into_any();
    }

    let (b, v) = (bank.get_untracked(), version.get_untracked());
    let cleared = data
      .states
      .iter()
      .filter(|s| matches!(s, StageState::Cleared(_)))
      .count();
    let stars: u32 = data
      .states
      .iter()
      .map(|s| match s {
        StageState::Cleared(n) => u32::from(*n),
        _ => 0,
      })
      .sum();
    let total = data.stages.len();
    // 推荐关：第一个「可挑战」的关；全部通关后为 `None`，引导去打 Boss。
    let next = data
      .stages
      .iter()
      .zip(&data.states)
      .find(|(_, s)| **s == StageState::Open)
      .map(|(stage, _)| *stage);

    let nodes = data
      .stages
      .iter()
      .zip(&data.states)
      .enumerate()
      .map(|(i, (stage, state))| {
        let href = format!(
          "{}&stage={}",
          bank_href("/battle", v.as_deref(), b),
          encode_uri_component(stage.key)
        );
        view! { <StageNode index=i stage=*stage state=*state href=href /> }
      })
      .collect_view();

    view! {
      <p class="pxl-label text-xs text-muted-foreground" aria-live="polite">
        {tf(
          "rpg.map-summary",
          &[
            &cleared.to_string(),
            &total.to_string(),
            &stars.to_string(),
            &(total * 3).to_string(),
          ],
        )}
      </p>
      <NextQuest stage=next bank=b version=v />
      <ol class="space-y-3" aria-label=move || t("rpg.quest-map")>{nodes}</ol>
    }
    .into_any()
  };

  view! {
    <PageHeader
      title=Signal::derive(move || t("rpg.quest-map"))
      subtitle=Signal::derive(move || t("rpg.map-subtitle"))
      actions=ViewFn::from(bank_chips)
    />
    <PageContainer>{body}</PageContainer>
  }
}
