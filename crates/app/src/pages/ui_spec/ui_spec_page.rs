//! 页面外壳：按皮肤分叉。
//!
//! 两套皮肤各有一份七板块规范展示（现代风在 `modern/`），互不共享视图与取值。
//! 分叉必须发生在组件体里、用 `is_modern_at_mount()`：风格切换靠 `MainContent` 用两种不同根元素
//! 重建整棵路由子树，这样两套 DOM 都不会漏出对方的类名。
//!
//! `TabsPanel` 只在第一次切到时才挂载内容，所以图标集（93 个 SVG）与色板不会进首帧；
//! 切走再切回来也不丢已经调过的筛选、配色与输入状态。

use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::ui::{Tabs, TabsList, TabsPanel, TabsTrigger};
use crate::util::set_title;

use super::colors_section::ColorsSection;
use super::components_section::ComponentsSection;
use super::icons_section::IconsSection;
use super::modern::ModernUiSpec;
use super::motion_section::MotionSection;
use super::overview_section::OverviewSection;
use super::states_section::StatesSection;
use super::typography_section::TypographySection;

#[component]
pub fn UiSpecPage() -> impl IntoView {
  if crate::kit::is_modern_at_mount() {
    set_title("design.modern-title");
    return view! { <ModernUiSpec /> }.into_any();
  }
  set_title("design.page-title");
  let active = RwSignal::new(String::from("overview"));
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=Signal::derive(move || t("design.page-title"))
        subtitle=Signal::derive(move || t("design.page-subtitle"))
      />
      <PageContainer>
        <section class=crate::kit::card_class_with("p-4")>
          <p class="text-sm leading-relaxed text-muted-foreground">{move || t("design.intro")}</p>
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
          <TabsPanel value="overview" class="space-y-5 pt-5"><OverviewSection /></TabsPanel>
          <TabsPanel value="components" class="space-y-5 pt-5"><ComponentsSection /></TabsPanel>
          <TabsPanel value="icons" class="space-y-5 pt-5"><IconsSection /></TabsPanel>
          <TabsPanel value="colors" class="space-y-5 pt-5"><ColorsSection /></TabsPanel>
          <TabsPanel value="typography" class="space-y-5 pt-5"><TypographySection /></TabsPanel>
          <TabsPanel value="motion" class="space-y-5 pt-5"><MotionSection /></TabsPanel>
          <TabsPanel value="states" class="space-y-5 pt-5"><StatesSection /></TabsPanel>
        </Tabs>
      </PageContainer>
    </div>
  }
  .into_any()
}
