//! 现代风（modern UI）下的占位页。
//!
//! TODO: 现代风自己的组件设计规范页尚未实现，这里先给一块可操作、不串皮肤的占位。
//! 像素风的整套展示留到 `ui_spec_page.rs` 的另一分支，两边不共享任何类名或视图。

use ham_web_core::ui_style::UiStyle;
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::kit_modern::{MIcon, ModernIcon};
use crate::theme::use_display_prefs;
use crate::ui::{Button, Variant};

use super::spec_data::PLANNED_SECTIONS;

#[component]
pub fn ModernPlaceholder() -> impl IntoView {
  let prefs = use_display_prefs();
  view! {
    <PageHeader
      title=Signal::derive(move || t("design.modern-title"))
      subtitle=Signal::derive(move || t("design.modern-subtitle"))
    />
    <PageContainer>
      <div class="md-empty-card">
        <ModernIcon kind=MIcon::Grid />
        <div class="md-empty-title">{move || t("design.modern-empty-title")}</div>
        <p class="md-muted md-sm">{move || t("design.modern-empty-desc")}</p>
        <Button
          variant=Variant::Default
          class=crate::kit::btn_wrap_class()
          on_click=Callback::new(move |_| prefs.set_ui_style(UiStyle::Pixel))
        >
          {move || t("design.modern-switch")}
        </Button>
        <p class="md-muted md-sm">{move || t("design.modern-note")}</p>
      </div>

      <section class="md-panel">
        <h2 class="md-panel-title">{move || t("design.modern-planned")}</h2>
        <div class="flex flex-wrap gap-2">
          {PLANNED_SECTIONS
            .iter()
            .map(|key| {
              view! { <span class="md-tile md-sm">{move || t(key)}</span> }
            })
            .collect_view()}
        </div>
      </section>
    </PageContainer>
  }
}
