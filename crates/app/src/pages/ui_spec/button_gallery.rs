//! 按钮矩阵：变体 × 尺寸、指示灯、加载与禁用态，以及最后一次动作的读数屏。

use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::i18n::t;
use crate::icons::{Icon, IconKind};
use crate::ui::{Button, LedTone, Size, Switch, Variant};

const VARIANTS: &[(Variant, &str)] = &[
  (Variant::Default, "Default"),
  (Variant::Destructive, "Destructive"),
  (Variant::Outline, "Outline"),
  (Variant::Secondary, "Secondary"),
  (Variant::Ghost, "Ghost"),
  (Variant::Text, "Text"),
];

const SIZES: &[(Size, &str)] = &[
  (Size::Xs, "Xs"),
  (Size::Sm, "Sm"),
  (Size::Default, "Default"),
  (Size::Lg, "Lg"),
];

#[component]
pub fn ButtonGallery() -> impl IntoView {
  let last = RwSignal::new(String::new());
  let busy = RwSignal::new(false);
  let readout = move || {
    let text = last.get();
    if text.is_empty() {
      t("design.comp-idle")
    } else {
      text
    }
  };

  view! {
    <SectionCard title=Signal::derive(move || t("design.comp-buttons"))>
      <div class="space-y-4 p-4">
        <p class="text-xs leading-relaxed text-muted-foreground">
          {move || t("design.comp-buttons-hint")}
        </p>
        <div class="grid gap-3 lg:grid-cols-[1fr_240px]">
          <div class="space-y-3 min-w-0">
            {VARIANTS
              .iter()
              .map(|(variant, variant_name)| {
                let name = *variant_name;
                view! {
                  <div class=crate::kit::tile_class("flex flex-wrap items-center gap-2 p-2")>
                    <code class="text-xs text-muted-foreground w-24 shrink-0">{name}</code>
                    {SIZES
                      .iter()
                      .map(|(size, size_name)| {
                        let size_name = *size_name;
                        view! {
                          <Button
                            variant=*variant
                            size=*size
                            on_click=Callback::new(move |_| {
                              last.set(format!("{name} · {size_name}"));
                            })
                          >
                            {size_name}
                          </Button>
                        }
                      })
                      .collect_view()}
                  </div>
                }
              })
              .collect_view()}
          </div>
          <div class=super::spec_data::pixel_only("pxl-display", "md-panel", "flex flex-col gap-2 p-3 min-w-0")>
            <span class=super::spec_data::pixel_only("pxl-label", "", "text-xs")>{move || t("design.comp-last-action")}</span>
            <span class=crate::kit::stat_text_class("text-base break-all")>{readout}</span>
          </div>
        </div>
        <div class=crate::kit::tile_class("flex flex-wrap items-center gap-3 p-3")>
          <Button
            variant=Variant::Default
            led=LedTone::Rx
            led_on=Signal::derive(|| true)
            on_click=Callback::new(move |_| last.set(String::from("LedTone::Rx")))
          >
            {move || t("design.comp-with-led")}
          </Button>
          <Button
            variant=Variant::Outline
            loading=busy
            on_click=Callback::new(move |_| last.set(String::from("loading")))
          >
            {move || t("design.states-loading")}
          </Button>
          <label class="flex cursor-pointer items-center gap-2 text-xs text-muted-foreground">
            <Switch checked=busy on_change=Callback::new(move |v: bool| busy.set(v)) />
            {move || t("design.states-loading")}
          </label>
          <Button
            variant=Variant::Destructive
            disabled=Signal::derive(|| true)
          >
            {move || t("design.states-disabled")}
          </Button>
          <Button
            variant=Variant::Outline
            size=Size::Icon
            aria_label=Signal::derive(move || t("design.comp-icon-button"))
            on_click=Callback::new(move |_| last.set(String::from("Size::Icon")))
          >
            <Icon kind=IconKind::Settings />
          </Button>
        </div>
      </div>
    </SectionCard>
  }
}
