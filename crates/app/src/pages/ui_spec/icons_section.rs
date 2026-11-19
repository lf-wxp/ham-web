//! 图标集：93 个像素图标的搜索、倍率切换、选中详情，以及像素精灵。

use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::debounced::{SEARCH_DEBOUNCE, set_if_changed, use_debounced};
use crate::i18n::t;
use crate::icons::{Icon, IconKind, PixelSprite};
use crate::ui::{Button, Chip, ChipGroup, Input, InputType, Variant, push_success};
use crate::util::copy_text;

use super::spec_data::SPRITES;

const SIZES: &[(&str, &str)] = &[("1", "size-6"), ("2", "size-12"), ("3", "size-18")];

const FIRST_BATCH: usize = 24;
const BATCH_STEP: usize = 24;

#[component]
pub fn IconsSection() -> impl IntoView {
  let query = RwSignal::new(String::new());
  let debounced = use_debounced(query, SEARCH_DEBOUNCE);
  let size = RwSignal::new(String::from("1"));
  let icon_class = RwSignal::new(String::from("size-6"));
  let selected = RwSignal::new(IconKind::Radio);
  let cap = RwSignal::new(FIRST_BATCH);

  let filtered = Memo::new(move |_| {
    let needle = debounced.get().to_lowercase();
    IconKind::ALL
      .iter()
      .copied()
      .filter(|kind| {
        needle.is_empty()
          || kind.name().contains(&needle)
          || format!("{kind:?}").to_lowercase().contains(&needle)
      })
      .collect::<Vec<IconKind>>()
  });
  let shown = Memo::new(move |_| {
    filtered
      .get()
      .into_iter()
      .take(cap.get())
      .collect::<Vec<IconKind>>()
  });
  crate::after_paint::grow_cap(cap, BATCH_STEP, move || {
    filtered.with_untracked(|items| items.len())
  });

  let select_size = Callback::new(move |v: String| {
    size.set(v.clone());
    let class = SIZES
      .iter()
      .find(|(slug, _)| *slug == v)
      .map_or("size-6", |(_, class)| *class);
    icon_class.set(String::from(class));
  });

  view! {
    <SectionCard title=Signal::derive(move || t("design.icons-title"))>
      <div class="space-y-4 p-4">
        <p class="text-xs leading-relaxed text-muted-foreground">
          {move || t("design.icons-hint")}
        </p>
        <div class="flex flex-wrap items-center gap-3">
          <Input
            value=query
            on_change=Callback::new(move |v: String| {
              query.set(v);
              set_if_changed(cap, FIRST_BATCH);
            })
            kind=InputType::Search
            clearable=true
            placeholder=Signal::derive(move || t("design.icons-placeholder"))
            aria_label=Signal::derive(move || t("design.icons-search-label"))
            class="w-56"
          />
          <div class="flex items-center gap-2">
            <span class="pxl-label text-xs text-muted-foreground">
              {move || t("design.icons-size")}
            </span>
            <ChipGroup
              value=size
              on_change=select_size
              aria_label=Signal::derive(move || t("design.icons-size"))
            >
              <Chip value="1">"1x"</Chip>
              <Chip value="2">"2x"</Chip>
              <Chip value="3">"3x"</Chip>
            </ChipGroup>
          </div>
          <span class="text-xs text-muted-foreground">
            {move || format!("{} {}", filtered.get().len(), t("design.icons-count"))}
          </span>
        </div>

        <div class="grid gap-4 lg:grid-cols-[1fr_240px]">
          <div class="grid grid-cols-3 gap-2 sm:grid-cols-4 md:grid-cols-6">
            <For each=move || shown.get() key=|kind| kind.name() let:kind>
              <Button
                variant=Variant::Outline
                class="h-auto flex flex-col items-center gap-2 p-2 min-w-0"
                aria_label=Signal::derive(move || String::from(kind.name()))
                on_click=Callback::new(move |_| selected.set(kind))
              >
                <Icon kind=kind class=icon_class />
                <span class="w-full truncate text-xs text-muted-foreground">{kind.name()}</span>
              </Button>
            </For>
          </div>
          <div class=crate::kit::tile_class("flex flex-col items-center gap-3 p-3 min-w-0")>
            <span class="pxl-label text-xs text-muted-foreground">
              {move || t("design.icons-detail")}
            </span>
            <div class="pxl-display flex items-center justify-center p-3">
              {move || view! {
                <Icon kind=selected.get() class=Signal::derive(|| String::from("size-18")) />
              }}
            </div>
            <code class="w-full text-xs break-all text-foreground">
              {move || format!("IconKind::{:?}", selected.get())}
            </code>
            <code class="w-full text-xs break-all text-muted-foreground">
              {move || selected.get().name()}
            </code>
            <Button
              variant=Variant::Secondary
              on_click=Callback::new(move |_| {
                copy_text(&format!("<Icon kind=IconKind::{:?} />", selected.get()));
                push_success(t("design.icons-copied"));
              })
            >
              {move || t("design.icons-copy")}
            </Button>
          </div>
        </div>
        {move || {
          filtered.get().is_empty().then(|| {
            view! { <p class="text-xs text-muted-foreground">{t("design.icons-empty")}</p> }
          })
        }}
      </div>
    </SectionCard>

    <SectionCard title=Signal::derive(move || t("design.icons-sprites"))>
      <div class="space-y-3 p-4">
        <p class="text-xs leading-relaxed text-muted-foreground">
          {move || t("design.icons-sprites-hint")}
        </p>
        <div class="flex flex-wrap gap-3">
          {SPRITES
            .iter()
            .map(|name| {
              view! {
                <div class=crate::kit::tile_class("flex flex-col items-center gap-1 p-2")>
                  <PixelSprite name=*name scale=3 />
                  <code class="text-xs text-muted-foreground">{*name}</code>
                </div>
              }
            })
            .collect_view()}
        </div>
      </div>
    </SectionCard>
  }
}
