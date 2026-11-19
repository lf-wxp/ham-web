//! 占位与状态组件：空态、错误态、加载指示、骨架屏、卡片内加载占位。

use leptos::prelude::*;

use crate::components::common::{
  EmptyState, ErrorState, Loading, LoadingBlock, SectionCard, Skeleton,
};
use crate::i18n::t;
use crate::icons::IconKind;
use crate::ui::Separator;

#[component]
pub fn PlaceholderGallery() -> impl IntoView {
  let retried = RwSignal::new(0_usize);

  view! {
    <SectionCard title=Signal::derive(move || t("design.comp-placeholders"))>
      <div class="space-y-4 p-4">
        <p class="text-xs leading-relaxed text-muted-foreground">
          {move || t("design.comp-placeholders-hint")}
        </p>

        <div class="grid gap-3 md:grid-cols-2">
          <EmptyState
            title=Signal::derive(move || t("design.comp-empty"))
            description=Signal::derive(move || t("design.comp-empty-desc"))
            icon=IconKind::Search
          />
          <div class=crate::kit::tile_class("space-y-3 p-3")>
            <code class="block text-xs text-muted-foreground">"EmptyState plain=true"</code>
            <EmptyState
              plain=true
              title=Signal::derive(move || t("design.comp-empty-plain"))
              icon=IconKind::Box
            />
            <ErrorState
              title=Signal::derive(move || t("design.comp-error"))
              detail=Signal::derive(move || t("design.comp-error-detail"))
              retry_label=Signal::derive(move || t("design.comp-retry"))
              on_retry=Callback::new(move |_| retried.update(|n| *n += 1))
            />
            <code class="block text-xs text-muted-foreground">
              {move || format!("retry x {}", retried.get())}
            </code>
          </div>
        </div>

        <Separator />

        <div class="grid gap-3 md:grid-cols-2">
          <div class=crate::kit::tile_class("space-y-2 p-3")>
            <code class="block text-xs text-muted-foreground">"Loading"</code>
            <Loading
              label=Signal::derive(move || t("design.comp-loading"))
              class="py-6"
            />
          </div>
          <div class=crate::kit::tile_class("space-y-2 p-3")>
            <code class="block text-xs text-muted-foreground">"Skeleton / LoadingBlock"</code>
            <div class="space-y-2">
              <Skeleton class="h-4 w-2/3" />
              <Skeleton class="h-4 w-1/2" />
              <Skeleton class="h-16 w-full" />
            </div>
            <LoadingBlock label=Signal::derive(move || t("design.comp-loading-block")) />
          </div>
        </div>
      </div>
    </SectionCard>
  }
}
