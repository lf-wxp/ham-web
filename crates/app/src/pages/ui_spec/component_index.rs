//! 通用组件总览：把 `ui/`、`components/radio/`、`components/common/` 的可复用组件按族列全。

use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::i18n::t;

use super::spec_data::COMPONENT_FAMILIES;

#[component]
pub fn ComponentIndex() -> impl IntoView {
  view! {
    <SectionCard title=Signal::derive(move || t("design.comp-index"))>
      <div class="space-y-3 p-4">
        <p class="text-xs leading-relaxed text-muted-foreground">
          {move || t("design.comp-index-hint")}
        </p>
        <div data-spec-list="component-families" class="grid gap-3 md:grid-cols-2">
          {COMPONENT_FAMILIES
            .iter()
            .map(|family| {
              let key = family.key;
              view! {
                <section class=crate::kit::tile_class("space-y-2 p-3 min-w-0")>
                  <h3 class=super::spec_data::pixel_only("pxl-sub-head", "md-section-title", "")>{move || t(key)}</h3>
                  <div class="flex flex-wrap gap-1">
                    {family
                      .items
                      .iter()
                      .map(|name| {
                        view! {
                          <code class=crate::kit::tag_class("text-xs")>{*name}</code>
                        }
                      })
                      .collect_view()}
                  </div>
                </section>
              }
            })
            .collect_view()}
        </div>
      </div>
    </SectionCard>
  }
}
