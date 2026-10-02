use leptos::prelude::*;

use super::DEFAULT_TITLE;
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn NotFoundPage() -> impl IntoView {
  set_title(DEFAULT_TITLE);
  view! {
    <div class="container mx-auto max-w-2xl px-4 py-10 animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <h1 class="text-lg font-semibold mb-2">{move || t("未找到页面")}</h1>
      <p class="text-sm text-muted-foreground mb-4">{move || t("您访问的页面不存在。")}</p>
      <a class="underline underline-offset-4" href="/">
        {move || t("返回首页")}
      </a>
    </div>
  }
}
