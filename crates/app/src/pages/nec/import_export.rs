//! `.nec` 文本的导入 / 导出面板（默认折叠）。
//!
//! 解析与序列化都在核心（`ham_web_core::nec`），这里只管收字、发字。

use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::Textarea;

/// 导入 / 导出面板。导入解析失败的原因由页面写进提示（这里不猜）。
#[component]
pub(super) fn ImportExportSection(
  nec_text: RwSignal<String>,
  on_import: Callback<(), ()>,
  on_export: Callback<(), ()>,
) -> impl IntoView {
  view! {
    <details class="rounded-xl border bg-card">
      <summary class="cursor-pointer px-4 py-3 text-sm font-semibold">
        {move || t("tools.nec-import-export")}
      </summary>
      <div class="space-y-2 px-4 pb-4">
        <Textarea
          value=nec_text
          on_change=Callback::new(move |v: String| nec_text.set(v))
          placeholder=Signal::derive(move || t("tools.nec-paste-nec-here"))
          aria_label=Signal::derive(move || t("tools.nec-paste-nec-here"))
          class="h-32 font-mono text-xs"
        />
        <div class="flex flex-wrap gap-2">
          <button
            type="button"
            class="cursor-pointer rounded-md border bg-background px-3 py-1 text-xs font-medium hover:bg-accent"
            on:click=move |_| on_import.run(())
          >
            {move || t("tools.nec-import")}
          </button>
          <button
            type="button"
            class="cursor-pointer rounded-md border bg-background px-3 py-1 text-xs font-medium hover:bg-accent"
            on:click=move |_| on_export.run(())
          >
            {move || t("tools.nec-export-current")}
          </button>
        </div>
      </div>
    </details>
  }
}
