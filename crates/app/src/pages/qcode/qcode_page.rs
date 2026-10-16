use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader, Skeleton};
use crate::data;
use crate::util::set_title;

use super::qcode_view::QCodeView;
use crate::i18n::t;

/// 简语速查页面：加载词条数据后交由 [`QCodeView`] 渲染。
#[component]
pub fn QCodePage() -> impl IntoView {
  set_title("shell.q-codes-title");
  let glossary = LocalResource::new(data::load_glossary);
  move || match glossary.get() {
    Some(g) => view! { <QCodeView entries=g.entries() /> }.into_any(),
    // 首屏数据未到时铺骨架行（页头先行、列表轮廓占位），避免 spinner 消失瞬间的高度跳变。
    None => view! {
      <PageHeader
        title=move || t("shell.q-codes-title")
        subtitle=move || t("knowledge.q-codes-cw-abbreviations")
      />
      <PageContainer>
        <span class="sr-only" role="status">{move || t("knowledge.loading-abbreviations")}</span>
        {(0..6)
          .map(|_| {
            view! {
              <div class="pxl-card border bg-card p-4">
                <Skeleton class="h-3 w-24" />
                <Skeleton class="mt-3 h-4 w-3/4" />
              </div>
            }
          })
          .collect_view()}
      </PageContainer>
    }
    .into_any(),
  }
}
