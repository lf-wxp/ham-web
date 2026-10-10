use leptos::prelude::*;

use crate::components::common::PageContainer;
use crate::util::set_title;

use super::photo_processor::PhotoProcessor;
use crate::i18n::t;

#[component]
pub fn PhotoProcessorPage() -> impl IntoView {
  set_title("shell.photo-tool-amateur-radio");
  view! {
    <PageContainer class="py-6 space-y-4 pb-28 sm:pb-20 animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <div class="mb-8">
        <h1 class="text-lg font-semibold mb-2">{move || t("tools.amateur-radio-application-photo")}</h1>
        <p class="text-muted-foreground">{move || t("tools.a-photo-tool-to")}</p>
      </div>
      <PhotoProcessor />
    </PageContainer>
  }
}
