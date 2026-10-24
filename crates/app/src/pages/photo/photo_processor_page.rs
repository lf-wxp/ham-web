use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::util::set_title;

use super::photo_processor::PhotoProcessor;
use crate::i18n::t;

#[component]
pub fn PhotoProcessorPage() -> impl IntoView {
  set_title("shell.photo-tool-amateur-radio");
  view! {
    // 页头走 `PageHeader`（`pxl-title` + 吸顶 + 主色左轨的唯一来源）。
    <div class="min-h-svh animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=Signal::derive(move || t("tools.amateur-radio-application-photo"))
        subtitle=Signal::derive(move || t("tools.a-photo-tool-to"))
      />
      <PageContainer class="py-6 space-y-4 pb-28 sm:pb-20">
        <PhotoProcessor />
      </PageContainer>
    </div>
  }
}
