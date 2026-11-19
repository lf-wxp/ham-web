//! 通用组件展示：子标签把六组示例分开，切走再切回来不丢已经调过的控件状态。

use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::{Tabs, TabsList, TabsPanel, TabsTrigger};

use super::action_gallery::ActionGallery;
use super::advanced_input_gallery::AdvancedInputGallery;
use super::button_gallery::ButtonGallery;
use super::component_index::ComponentIndex;
use super::data_gallery::DataGallery;
use super::feedback_gallery::FeedbackGallery;
use super::form_gallery::FormGallery;
use super::overlay_gallery::OverlayGallery;
use super::placeholder_gallery::PlaceholderGallery;
use super::radio_gallery::RadioGallery;
use super::spec_data::COMPONENT_GROUPS;

#[component]
pub fn ComponentsSection() -> impl IntoView {
  let group = RwSignal::new(String::from("basics"));
  view! {
    <ComponentIndex />
    <Tabs value=group on_change=Callback::new(move |v: String| group.set(v))>
      <TabsList aria_label=Signal::derive(move || t("design.comp-groups"))>
        {COMPONENT_GROUPS
          .iter()
          .map(|(slug, key)| {
            let key = *key;
            view! { <TabsTrigger value=*slug>{move || t(key)}</TabsTrigger> }
          })
          .collect_view()}
      </TabsList>
      <TabsPanel value="basics" class="space-y-5 pt-4">
        <ButtonGallery />
        <FormGallery />
      </TabsPanel>
      <TabsPanel value="advanced" class="space-y-5 pt-4">
        <AdvancedInputGallery />
        <ActionGallery />
      </TabsPanel>
      <TabsPanel value="data" class="space-y-5 pt-4"><DataGallery /></TabsPanel>
      <TabsPanel value="overlays" class="space-y-5 pt-4">
        <OverlayGallery />
        <FeedbackGallery />
      </TabsPanel>
      <TabsPanel value="radio" class="space-y-5 pt-4"><RadioGallery /></TabsPanel>
      <TabsPanel value="placeholders" class="space-y-5 pt-4"><PlaceholderGallery /></TabsPanel>
    </Tabs>
  }
}
