use leptos::prelude::*;

use crate::icons::{Icon, IconKind};
use crate::photo::{self, PhotoResult};
use crate::ui::{Size, Variant, button_class};

#[component]
pub(super) fn PhotoResultView(result: PhotoResult, file_name: String) -> impl IntoView {
  let r = StoredValue::new(result.clone());
  let name = StoredValue::new(file_name);
  view! {
    <div class="space-y-4">
      <div class="flex items-center gap-2 p-3 bg-green-50 border border-green-200 rounded-lg text-green-800 dark:bg-green-950/40 dark:border-green-900 dark:text-green-200">
        <Icon kind=IconKind::CheckCircle class="w-4 h-4" />
        <span class="text-sm">"处理完成！请右键或长按保存下方处理后的图片。"</span>
      </div>
      <div class="text-sm font-mono text-gray-600 bg-gray-50 dark:text-gray-300 dark:bg-muted p-3 rounded-lg">
        <div class="grid grid-cols-2 gap-2">
          <div>"文件大小: " {photo::format_size(result.size)}</div>
          <div>"格式: " {result.mime.clone()}</div>
          <div>"宽度: " {result.width} "px"</div>
          <div>"高度: " {result.height} "px"</div>
        </div>
      </div>
      <div class="relative w-full max-w-md mx-auto">
        <div class="relative aspect-auto border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
          <img
            src=result.data_url.clone()
            alt="处理后的照片"
            width=result.width
            height=result.height
            class="w-full h-auto object-contain"
          />
        </div>
      </div>
      <div class="flex justify-center">
        <button
          class=button_class(Variant::Default, Size::Default, "flex items-center gap-2")
          on:click=move |_| r.with_value(|r| name.with_value(|n| photo::download(r, n)))
        >
          <Icon kind=IconKind::Download class="w-4 h-4" />
          "下载处理后的照片"
        </button>
      </div>
    </div>
  }
}
