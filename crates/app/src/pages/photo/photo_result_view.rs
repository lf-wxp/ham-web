use leptos::prelude::*;

use crate::i18n::t;
use crate::icons::{Icon, IconKind};
use crate::photo::{self, PhotoResult};
use crate::ui::{Button, Size, Variant};

#[component]
pub(super) fn PhotoResultView(result: PhotoResult, file_name: String) -> impl IntoView {
  let r = StoredValue::new(result.clone());
  let name = StoredValue::new(file_name);
  view! {
    <div class="space-y-4">
      <div class="flex items-center gap-2 p-3 bg-win/10 border border-win/40 text-win-text">
        <Icon kind=IconKind::CheckCircle class="size-6" />
        <span class="text-sm">{move || t("tools.done-right-click-or")}</span>
      </div>
      <div class="text-sm font-mono text-muted-foreground bg-muted p-3">
        <div class="grid grid-cols-2 gap-2">
          <div>{t("tools.file-size")} {photo::format_size(result.size)}</div>
          <div>{t("tools.format")} {result.mime.clone()}</div>
          <div>{t("tools.width")} {result.width} "px"</div>
          <div>{t("tools.height")} {result.height} "px"</div>
        </div>
      </div>
      <div class="relative w-full max-w-md mx-auto">
        <div class="relative aspect-auto border overflow-hidden">
          <img
            src=result.data_url.clone()
            alt=move || t("tools.processed-photo")
            width=result.width
            height=result.height
            class="w-full h-auto object-contain"
          />
        </div>
      </div>
      <div class="flex justify-center">
        <Button
          variant=Variant::Default
          size=Size::Default
          class="flex items-center gap-2"
          on_click=Callback::new(move |_| r.with_value(|r| name.with_value(|n| photo::download(r, n))))
        >
          <Icon kind=IconKind::Download class="size-6" />
          {move || t("tools.download-the-processed-photo")}
        </Button>
      </div>
    </div>
  }
}
