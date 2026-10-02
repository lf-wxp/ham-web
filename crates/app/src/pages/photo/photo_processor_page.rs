use leptos::prelude::*;

use crate::util::set_title;

use super::photo_processor::PhotoProcessor;
use crate::i18n::t;

#[component]
pub fn PhotoProcessorPage() -> impl IntoView {
  set_title(&t("报名照片处理工具 - 业余无线电执照考试"));
  view! {
    <div class="container mx-auto px-4 py-6 max-w-5xl space-y-4 pb-28 sm:pb-20 animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <div class="mb-8">
        <h1 class="text-lg font-semibold mb-2">{move || t("业余无线电报名照片处理")}</h1>
        <p class="text-muted-foreground">{move || t("专业的照片处理工具，帮助您快速处理符合报名要求的证件照和人像照")}</p>
      </div>
      <PhotoProcessor />
    </div>
  }
}
