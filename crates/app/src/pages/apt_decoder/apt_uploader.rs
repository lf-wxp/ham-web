//! 上传区：点击选择或拖拽 WAV 音频文件。

use leptos::prelude::*;
use web_sys::{DragEvent, File, HtmlInputElement};

use crate::i18n::t;
use crate::icons::{Icon, IconKind};

#[component]
pub(super) fn AptUploader(
  on_file: Callback<File>,
  #[prop(into)] disabled: Signal<bool>,
) -> impl IntoView {
  let drag_over = RwSignal::new(false);

  let submit = move |file: File| {
    if disabled.get_untracked() {
      return;
    }
    on_file.run(file);
  };

  let zone_class = move || {
    let state = if disabled.get() {
      "opacity-50 cursor-not-allowed"
    } else {
      "hover:bg-muted/40 cursor-pointer"
    };
    let border = if drag_over.get() {
      "border-primary bg-primary/5"
    } else {
      "border-border"
    };
    format!(
      "flex flex-col items-center justify-center gap-1 rounded-xl border-2 border-dashed p-8 text-center transition-all duration-200 {state} {border}"
    )
  };

  view! {
    <div class="space-y-2">
      <label
        for="apt-file"
        class=zone_class
        on:dragover=move |e: DragEvent| e.prevent_default()
        on:dragenter=move |e: DragEvent| {
          e.prevent_default();
          drag_over.set(true);
        }
        on:dragleave=move |_| drag_over.set(false)
        on:drop=move |e: DragEvent| {
          e.prevent_default();
          drag_over.set(false);
          if let Some(f) = e
            .data_transfer()
            .and_then(|dt| dt.files())
            .and_then(|f| f.get(0))
          {
            submit(f);
          }
        }
      >
        <Icon kind=IconKind::Satellite class="mb-1 h-8 w-8 text-muted-foreground" />
        <span class="text-sm font-medium">{move || t("选择 APT 录音（WAV）")}</span>
        <span class="text-xs text-muted-foreground">{move || t("点击选择或拖拽音频文件到此处")}</span>
        <span class="mt-1 text-xs text-muted-foreground">
          {move || t("推荐：RTL-SDR + SDR# / rtl_fm 录制，11025 / 20800 Hz 单声道 WAV")}
        </span>
      </label>
      <input
        id="apt-file"
        type="file"
        accept=".wav,audio/wav,audio/x-wav,audio/*"
        disabled=move || disabled.get()
        class="hidden"
        on:change=move |e| {
          let input: HtmlInputElement = event_target(&e);
          if let Some(f) = input.files().and_then(|f| f.get(0)) {
            submit(f);
          }
          input.set_value("");
        }
      />
    </div>
  }
}
