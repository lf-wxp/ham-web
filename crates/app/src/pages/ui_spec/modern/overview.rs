//! 设计总规格：6 个筛选维度、8 张原则卡、8 行规格速查，点卡片联动高亮速查行。

use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::i18n::t;
use crate::kit_modern::ModernIcon;
use crate::ui::{Button, Chip, ChipGroup, Variant};

use super::super::spec_data::FILTERS;
use super::data::{MD_PRINCIPLES, MD_SPEC_ROWS};

#[component]
pub fn ModernOverview() -> impl IntoView {
  let filter = RwSignal::new(String::from("all"));
  let selected = RwSignal::new(String::new());
  let visible = Memo::new(move |_| {
    let current = filter.get();
    MD_PRINCIPLES
      .iter()
      .enumerate()
      .filter(|(_, p)| current == "all" || p.group.slug() == current)
      .map(|(i, _)| i)
      .collect::<Vec<usize>>()
  });

  view! {
    <SectionCard title=Signal::derive(move || t("design.overview-principles"))>
      <div class="space-y-4 p-4">
        <p class="md-muted md-sm">{move || t("design.md-specs-hint")}</p>
        <div class="flex flex-wrap items-center gap-2">
          <span class="md-muted md-sm">{move || t("design.filter-label")}</span>
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
        <div data-spec-list="principles" class="grid gap-3 sm:grid-cols-2">
          <For each=move || visible.get() key=|i| *i let:i>
            // 不写 `aria_label`：卡片名取自全部可见文字，避免 axe 的 label-content-name-mismatch。
            // 现代风按钮默认固定高度单行；长取值会撑出横向滚动，所以按既有做法加 `md-btn-wrap`。
            <Button
              variant=Variant::Outline
              // `md-tile`：这是卡片不是药丸按钮，`.md-btn` 默认的 `--md-r-pill` 交给 tile 的 `--md-r-md` 覆盖。
              class=crate::cn::cn(&[
                "md-tile h-auto flex min-w-0 flex-col items-start gap-2 p-3 text-left whitespace-normal",
                crate::kit::btn_wrap_class(),
              ])
              on_click=Callback::new(move |_| selected.set(String::from(MD_PRINCIPLES[i].spec)))
            >
              <span class="flex min-w-0 items-center gap-2">
                <ModernIcon kind=MD_PRINCIPLES[i].icon />
                <span class="md-sm">{move || t(MD_PRINCIPLES[i].title)}</span>
              </span>
              <span class="md-muted md-sm break-words">{move || t(MD_PRINCIPLES[i].desc)}</span>
              <code class="md-mono md-sm min-w-0 break-all">{MD_PRINCIPLES[i].value}</code>
            </Button>
          </For>
        </div>
      </div>
    </SectionCard>

    <SectionCard title=Signal::derive(move || t("design.overview-specs"))>
      <div data-spec-list="specs" class="divide-y">
        <For each=move || MD_SPEC_ROWS key=|row| row.0 let:row>
          <div
            class=move || {
              if selected.get() == row.0 {
                "flex min-w-0 flex-wrap items-center justify-between gap-2 bg-accent/40 px-4 py-2"
              } else {
                "flex min-w-0 flex-wrap items-center justify-between gap-2 px-4 py-2"
              }
            }
          >
            <span class="md-sm">{move || t(row.0)}</span>
            <code class="md-mono md-sm min-w-0 max-w-full break-all">{row.1}</code>
          </div>
        </For>
      </div>
    </SectionCard>
  }
}
