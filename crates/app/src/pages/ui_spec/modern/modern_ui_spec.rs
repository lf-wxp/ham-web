//! 现代风规范页外壳：页头 + 七个标签，构造期由 `ui_spec_page.rs` 分叉进来。
//!
//! `TabsPanel` 首次切到才挂载，图标集与色板因此不进首帧；切走再切回不丢面板内的状态。

use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::ui::{Tabs, TabsList, TabsPanel, TabsTrigger};

use super::colors::ModernColors;
use super::components::ModernComponents;
use super::icons::ModernIcons;
use super::motion::ModernMotion;
use super::overview::ModernOverview;
use super::states::ModernStates;
use super::typography::ModernTypography;

#[component]
pub fn ModernUiSpec() -> impl IntoView {
  let active = RwSignal::new(String::from("overview"));
  view! {
    <PageHeader
      title=Signal::derive(move || t("design.modern-title"))
      subtitle=Signal::derive(move || t("design.modern-subtitle"))
    />
    <PageContainer>
      // 一段说明文字不是内容块：`.md-panel::before` 的左上角标会落在圆角里像被裁掉一半。
      <section data-spec-intro class=crate::kit::tile_class("px-4 py-3")>
        <p class="md-muted md-sm">{move || t("design.md-intro")}</p>
      </section>
      <Tabs value=active on_change=Callback::new(move |v: String| active.set(v))>
        <TabsList aria_label=Signal::derive(move || t("design.tabs-label"))>
          <TabsTrigger value="overview">{move || t("design.tab-overview")}</TabsTrigger>
          <TabsTrigger value="components">{move || t("design.tab-components")}</TabsTrigger>
          <TabsTrigger value="icons">{move || t("design.tab-icons")}</TabsTrigger>
          <TabsTrigger value="colors">{move || t("design.tab-colors")}</TabsTrigger>
          <TabsTrigger value="typography">{move || t("design.tab-typography")}</TabsTrigger>
          <TabsTrigger value="motion">{move || t("design.tab-motion")}</TabsTrigger>
          <TabsTrigger value="states">{move || t("design.tab-states")}</TabsTrigger>
        </TabsList>
        // 内层 `data-spec-panel` 是 e2e 的定位锚点；纵向节奏只由它一层给，外层只留顶部间距。
        <TabsPanel value="overview" class="pt-5"><div data-spec-panel="overview" class="space-y-5"><ModernOverview /></div></TabsPanel>
        <TabsPanel value="components" class="pt-5"><div data-spec-panel="components" class="space-y-5"><ModernComponents /></div></TabsPanel>
        <TabsPanel value="icons" class="pt-5"><div data-spec-panel="icons" class="space-y-5"><ModernIcons /></div></TabsPanel>
        <TabsPanel value="colors" class="pt-5"><div data-spec-panel="colors" class="space-y-5"><ModernColors /></div></TabsPanel>
        <TabsPanel value="typography" class="pt-5"><div data-spec-panel="typography" class="space-y-5"><ModernTypography /></div></TabsPanel>
        <TabsPanel value="motion" class="pt-5"><div data-spec-panel="motion" class="space-y-5"><ModernMotion /></div></TabsPanel>
        <TabsPanel value="states" class="pt-5"><div data-spec-panel="states" class="space-y-5"><ModernStates /></div></TabsPanel>
      </Tabs>
    </PageContainer>
  }
}
