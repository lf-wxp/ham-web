use leptos::prelude::*;

use super::DEFAULT_TITLE;
use crate::components::common::PageContainer;
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn NotFoundPage() -> impl IntoView {
  set_title(DEFAULT_TITLE);
  // 404 在**路由兜底**里渲染，那里的 `DisplayPrefs` 是默认实例（不是页面树里的那个），
  // 所以 `is_modern()` / `is_modern_at_mount()` 一律返回「像素」。这里读 `<html data-ui-style>`
  // ——同一个事实来源，只是不依赖响应式上下文（`theme.rs` 里唯一的 DOM 读入口）。
  let modern = move || crate::theme::DisplayPrefs::style_is_modern_dom();
  view! {
    <PageContainer class="space-y-0 py-10 animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      // 刻意的例外（`ui_check` 的「页面手写 `<h1>`」跳过本文件）：404 是居中的错误页，
      // 没有内容页的「页头 + 主色左轨」语义。
      <h1 class=move || {
        if modern() { "md-h2 mb-2" } else { "pxl-title text-lg mb-2" }
      }>{move || t("knowledge.page-not-found")}</h1>
      <p class=move || {
        if modern() { "md-muted md-sm mb-4" } else { "text-sm text-muted-foreground mb-4" }
      }>{move || t("knowledge.the-page-you-requested")}</p>
      <a class=move || {
        if modern() { crate::kit::link_class() } else { "" }
      } href="/">
        {move || t("knowledge.back-to-home")}
      </a>
    </PageContainer>
  }
}
