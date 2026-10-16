use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader, Skeleton};
use crate::data;
use crate::util::set_title;

use super::glossary_view::GlossaryView;
use crate::i18n::t;

/// 术语表页面：加载词条数据后交由 [`GlossaryView`] 渲染。
#[component]
pub fn GlossaryPage() -> impl IntoView {
  set_title("shell.glossary");
  let glossary = LocalResource::new(data::load_glossary);
  move || match glossary.get() {
    Some(g) => view! { <GlossaryView entries=g.entries() /> }.into_any(),
    // 首屏数据未到时铺骨架行（页头先行、列表轮廓占位）。
    None => view! {
      <PageHeader
        title=move || t("shell.glossary")
        subtitle=move || t("knowledge.common-ham-terms-english")
      />
      <PageContainer>
        <span class="sr-only" role="status">{move || t("knowledge.loading-glossary")}</span>
        {(0..6)
          .map(|_| {
            view! {
              <div class="pxl-card border bg-card p-4">
                <Skeleton class="h-3 w-28" />
                <Skeleton class="mt-3 h-4 w-4/5" />
              </div>
            }
          })
          .collect_view()}
      </PageContainer>
    }
    .into_any(),
  }
}
