//! 通用组件：组件索引 + 六个子标签，与像素风共用同一批共享控件实样。
//!
//! 只有「电台读数 / 反馈」两族是像素风专属实现，这里用现代风等价物补上；
//! 其余 gallery 直接复用，控件本身在 `crate::ui` 里按构造期皮肤派发成 `md-*`。

use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::{Tabs, TabsList, TabsPanel, TabsTrigger};

use super::super::action_gallery::ActionGallery;
use super::super::advanced_input_gallery::AdvancedInputGallery;
use super::super::button_gallery::ButtonGallery;
use super::super::component_index::ComponentIndex;
use super::super::data_gallery::DataGallery;
use super::super::form_gallery::FormGallery;
use super::super::overlay_gallery::OverlayGallery;
use super::super::placeholder_gallery::PlaceholderGallery;
use super::super::spec_data::COMPONENT_GROUPS;
use super::feedback::ModernFeedback;
use super::radio::ModernRadio;

#[component]
pub fn ModernComponents() -> impl IntoView {
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
      <TabsPanel value="data" class="space-y-5 pt-4">
        <DataGallery />
      </TabsPanel>
      <TabsPanel value="overlays" class="space-y-5 pt-4">
        <OverlayGallery />
        <ModernFeedback />
      </TabsPanel>
      <TabsPanel value="radio" class="space-y-5 pt-4"><ModernRadio /></TabsPanel>
      <TabsPanel value="placeholders" class="space-y-5 pt-4">
        <PlaceholderGallery />
      </TabsPanel>
    </Tabs>
  }
}
