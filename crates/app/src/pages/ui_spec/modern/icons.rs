//! 图标一览：58 个 `MIcon` 的搜索 / 3 档倍率 / 计数 / 详情复制 / 分批，以及 10 个徽记。
//!
//! 项目与交互对齐像素风的「图标集 + 精灵」两块，只是现代风没有点阵精灵，
//! 这一区用同一批形状名的大尺寸图标呈现。

use leptos::prelude::*;

use crate::components::common::{EmptyState, SectionCard};
use crate::debounced::{SEARCH_DEBOUNCE, use_debounced};
use crate::i18n::{t, tf};
use crate::kit_modern::ModernIcon;
use crate::ui::{Button, Chip, ChipGroup, Input, InputType, Variant, push_success};
use crate::util::copy_text;

use super::data::{MD_EMBLEMS, MD_ICONS};

const SIZES: &[(&str, &str)] = &[("1", "size-6"), ("2", "size-12"), ("3", "size-18")];
const FIRST_BATCH: usize = 24;
const BATCH_STEP: usize = 24;

#[component]
pub fn ModernIcons() -> impl IntoView {
  let query = RwSignal::new(String::new());
  let debounced = use_debounced(query, SEARCH_DEBOUNCE);
  let size = RwSignal::new(String::from("1"));
  let selected = RwSignal::new(String::from("Grid"));
  let cap = RwSignal::new(FIRST_BATCH);

  // 倍率只有一个来源：选中的档位。图标类名由它派生，不再单独存一份（两份会不同步）。
  let icon_class = Signal::derive(move || {
    SIZES
      .iter()
      .find(|(slug, _)| *slug == size.get())
      .map_or_else(|| String::from("size-6"), |(_, class)| String::from(*class))
  });

  let filtered = Memo::new(move |_| {
    let needle = debounced.get().to_lowercase();
    MD_ICONS
      .iter()
      .copied()
      .filter(|(_, name)| needle.is_empty() || name.to_lowercase().contains(&needle))
      .collect::<Vec<_>>()
  });
  let shown = Memo::new(move |_| {
    filtered
      .get()
      .into_iter()
      .take(cap.get())
      .collect::<Vec<_>>()
  });
  // 查询结果一变就复位并重新分批：`grow_cap` 补满即结束，只登记一次的话清空搜索后只剩首批。
  let filtered_len = Memo::new(move |_| filtered.with(Vec::len));
  crate::after_paint::grow_cap_per_filter(cap, BATCH_STEP, FIRST_BATCH, filtered_len);

  view! {
    <SectionCard title=Signal::derive(move || t("design.icons-title"))>
      <div class="space-y-4 p-4">
        <p class="md-muted md-sm">{move || t("design.md-icons-hint")}</p>
        <div class="flex flex-wrap items-center gap-3">
          <Input
            value=query
            on_change=Callback::new(move |v: String| query.set(v))
            kind=InputType::Search
            clearable=true
            aria_label=Signal::derive(move || t("design.md-icon-search"))
            placeholder=Signal::derive(move || t("design.md-icon-search"))
          />
          <ChipGroup
            value=size
            on_change=Callback::new(move |v: String| size.set(v))
            aria_label=Signal::derive(move || t("design.icons-size"))
          >
            <Chip value="1">"1x"</Chip>
            <Chip value="2">"2x"</Chip>
            <Chip value="3">"3x"</Chip>
          </ChipGroup>
          <span class="md-muted md-sm">
            {move || tf("design.md-icon-count", &[&filtered.get().len().to_string()])}
          </span>
        </div>
        <div class="grid gap-2 sm:grid-cols-2">
          <div data-spec-list="icons" class="grid grid-cols-2 gap-2 sm:grid-cols-4">
            <For each=move || shown.get() key=|row| row.1 let:row>
              <Button
                variant=Variant::Outline
                class="md-tile h-auto flex flex-col items-center gap-1 p-2"
                on_click=Callback::new(move |_| selected.set(String::from(row.1)))
              >
                <ModernIcon kind=row.0 class=icon_class />
                <code class="md-muted md-sm">{row.1}</code>
              </Button>
            </For>
          </div>
          <div class=crate::kit::tile_class("space-y-3 p-3")>
            <code class="md-sm">{move || selected.get()}</code>
            <div class="flex items-center gap-2">
              {move || {
                let name = selected.get();
                let kind = MD_ICONS
                  .iter()
                  .find(|(_, n)| **n == name)
                  .map_or(MD_ICONS[0].0, |(kind, _)| *kind);
                view! {
                  <ModernIcon kind=kind class=Signal::derive(move || String::from("size-12")) />
                }
                .into_any()
              }}
              <span class="md-muted md-sm">{move || selected.get()}</span>
            </div>
            <Button
              variant=Variant::Outline
              on_click=Callback::new(move |_| {
                copy_text(&format!("<ModernIcon kind=MIcon::{} />", selected.get()));
                push_success(t("design.md-icon-copied"));
              })
            >
              {move || t("design.md-icon-copy")}
            </Button>
          </div>
        </div>
        {move || {
          (filtered.get().is_empty()).then(|| {
            view! {
              <EmptyState plain=true title=Signal::derive(move || t("design.icons-empty")) />
            }
          })
        }}
      </div>
    </SectionCard>

    <SectionCard title=Signal::derive(move || t("design.md-emblem-title"))>
      <div data-spec-list="emblems" class="grid grid-cols-2 gap-2 p-4 sm:grid-cols-5">
        <For each=move || MD_EMBLEMS key=|row| row.0 let:row>
          <div class=crate::kit::tile_class("flex flex-col items-center gap-2 p-3")>
            <ModernIcon kind=row.1 class=Signal::derive(move || String::from("size-12")) />
            <code class="md-muted md-sm">{row.0}</code>
          </div>
        </For>
      </div>
    </SectionCard>
  }
}
