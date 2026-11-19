//! 设计总规格：原则卡片（可按维度筛选）与规格速查表。

use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::i18n::t;
use crate::icons::Icon;
use crate::ui::{Button, Chip, ChipGroup, Variant};

use super::spec_data::{FILTERS, PRINCIPLES, SPECS};

#[component]
pub fn OverviewSection() -> impl IntoView {
  let filter = RwSignal::new(String::from("all"));
  let selected = RwSignal::new(String::new());
  let visible = Memo::new(move |_| {
    let current = filter.get();
    PRINCIPLES
      .iter()
      .enumerate()
      .filter(|(_, p)| current == "all" || p.group.slug() == current)
      .map(|(i, _)| i)
      .collect::<Vec<usize>>()
  });

  view! {
    <SectionCard title=Signal::derive(move || t("design.overview-principles"))>
      <div class="space-y-4 p-4">
        <p class="text-xs leading-relaxed text-muted-foreground">
          {move || t("design.overview-principles-hint")}
        </p>
        <div class="flex flex-wrap items-center gap-2">
          <span class="pxl-label text-xs text-muted-foreground">
            {move || t("design.filter-label")}
          </span>
          <ChipGroup
            value=filter
            on_change=Callback::new(move |v: String| filter.set(v))
            aria_label=Signal::derive(move || t("design.filter-label"))
          >
            {FILTERS
              .iter()
              .map(|(slug, key)| {
                let key = *key;
                view! { <Chip value=*slug>{move || t(key)}</Chip> }
              })
              .collect_view()}
          </ChipGroup>
        </div>
        <div class="grid gap-3 sm:grid-cols-2">
          <For each=move || visible.get() key=|i| *i let:i>
            <Button
              variant=Variant::Outline
              class="h-auto flex flex-col items-start gap-2 p-3 text-left whitespace-normal"
              aria_label=Signal::derive(move || t(PRINCIPLES[i].title))
              on_click=Callback::new(move |_| selected.set(String::from(PRINCIPLES[i].spec)))
            >
              <span class="flex items-center gap-2">
                <Icon kind=PRINCIPLES[i].icon />
                <span class="text-base leading-tight text-foreground">
                  {move || t(PRINCIPLES[i].title)}
                </span>
              </span>
              <span class="text-xs leading-relaxed text-muted-foreground">
                {move || t(PRINCIPLES[i].desc)}
              </span>
              <code class=crate::kit::tag_class("text-xs")>{PRINCIPLES[i].value}</code>
            </Button>
          </For>
        </div>
      </div>
    </SectionCard>

    <SectionCard title=Signal::derive(move || t("design.overview-specs"))>
      <div class="divide-y">
        {SPECS
          .iter()
          .map(|row| {
            let key = row.key;
            let value = row.value;
            view! {
              <div
                class=move || {
                  if selected.get() == key {
                    "flex flex-wrap items-center justify-between gap-2 bg-accent/40 px-4 py-2"
                  } else {
                    "flex flex-wrap items-center justify-between gap-2 px-4 py-2"
                  }
                }
              >
                <span class="text-xs text-foreground">{move || t(key)}</span>
                <code class=crate::kit::tag_class("text-xs")>{value}</code>
              </div>
            }
          })
          .collect_view()}
      </div>
    </SectionCard>
  }
}
