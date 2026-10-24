use leptos::prelude::*;

use super::DEFAULT_TITLE;
use crate::components::common::PageContainer;
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn NotFoundPage() -> impl IntoView {
  set_title(DEFAULT_TITLE);
  view! {
    <PageContainer class="space-y-0 py-10 animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      // 刻意的例外（`ui_check` 的「页面手写 `<h1>`」跳过本文件）：404 是居中的错误页，
      // 没有内容页的「页头 + 主色左轨」语义；但字体仍走 `pxl-title`，与全站标题同一族。
      <h1 class="pxl-title text-lg mb-2">{move || t("knowledge.page-not-found")}</h1>
      <p class="text-sm text-muted-foreground mb-4">{move || t("knowledge.the-page-you-requested")}</p>
      <a class="underline underline-offset-4" href="/">
        {move || t("knowledge.back-to-home")}
      </a>
    </PageContainer>
  }
}
