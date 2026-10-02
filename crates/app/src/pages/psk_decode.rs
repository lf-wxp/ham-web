//! PSK31 解码器：上传 PSK31 录音（WAV），在浏览器本地解调为文本。
//!
//! 解码复用 `ham-web-core::psk31`（纯 Rust DSP），WAV 解析复用 `ham-web-apt`，音频不上传服务器。

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use web_sys::{DragEvent, File, HtmlInputElement};

use crate::icons::{Icon, IconKind};
use crate::ui::{Size, Variant, button_class, input_class};
use crate::util::{js_error_message, set_title};

use crate::i18n::{t, tf};

/// 读取文件字节（通过 Blob.array_buffer）。
async fn read_file_bytes(file: &File) -> Result<Vec<u8>, String> {
  let blob: web_sys::Blob = file.clone().into();
  let promise = blob.array_buffer();
  let buf = wasm_bindgen_futures::JsFuture::from(promise)
    .await
    .map_err(|e| js_error_message(&e))?;
  let arr: js_sys::ArrayBuffer = buf.unchecked_into();
  Ok(js_sys::Uint8Array::new(&arr).to_vec())
}

#[component]
pub fn PskDecodePage() -> impl IntoView {
  set_title(&t("PSK31 解码器"));

  let processing = RwSignal::new(false);
  let result = RwSignal::new(String::new());
  let error = RwSignal::new(None::<String>);
  let file_name = RwSignal::new(String::new());
  let center = RwSignal::new(1000.0f64);
  let drag_over = RwSignal::new(false);

  let decode = move |file: File| {
    processing.set(true);
    error.set(None);
    result.set(String::new());
    file_name.set(file.name());
    let c = center.get_untracked();
    spawn_local(async move {
      match read_file_bytes(&file).await {
        Ok(bytes) => match ham_web_apt::parse_wav(&bytes) {
          Ok((samples, rate)) => {
            let text = ham_web_core::psk31::decode_psk31(&samples, rate, c as f32);
            if text.is_empty() {
              error.set(Some(t(
                "未解出文本，请确认是 PSK31 音频（BPSK），或调整载波频率。",
              )));
            } else {
              result.set(text);
            }
          }
          Err(_) => error.set(Some(t("不是有效的 WAV 文件"))),
        },
        Err(e) => error.set(Some(e)),
      }
      processing.set(false);
    });
  };

  let submit = move |file: File| {
    if !processing.get_untracked() {
      decode(file);
    }
  };

  let reset = move |_| {
    result.set(String::new());
    error.set(None);
    file_name.set(String::new());
  };

  let zone_class = move || {
    let state = if processing.get() {
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
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{t("PSK31 解码器")}</h1>
            <div class="text-xs text-muted-foreground">{t("BPSK 31.25 波特 · Varicode · 纯本地解码")}</div>
          </div>
          <a
            href="/modes"
            class="rounded-md border px-2.5 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
          >
            {t("数字模式速查")}
          </a>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-5 px-4 py-5">
        <section class="rounded-xl border bg-card p-4 text-sm text-muted-foreground">
          <p>
            {t("上传一段 PSK31 音频（短波下边带解调后的 WAV），即可在浏览器本地解调出文本。")}
            " "
            {t("载波频率通常为 1000 Hz，若解不出文字可微调。音频不会上传到服务器。")}
          </p>
        </section>

        <section class="rounded-xl border bg-card p-4">
          {move || {
            (result.with(String::is_empty) && error.with(Option::is_none) && !processing.get())
              .then(|| {
                view! {
                  <div class="space-y-3">
                    <label for="psk-file" class=zone_class
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
                      <Icon kind=IconKind::AudioLines class="mb-1 h-8 w-8 text-muted-foreground" />
                      <span class="text-sm font-medium">{t("选择 PSK31 录音（WAV）")}</span>
                      <span class="text-xs text-muted-foreground">{t("点击选择或拖拽音频文件到此处")}</span>
                    </label>
                    <input
                      id="psk-file"
                      type="file"
                      accept=".wav,audio/wav,audio/x-wav,audio/*"
                      disabled=move || processing.get()
                      class="hidden"
                      on:change=move |e| {
                        let input: HtmlInputElement = event_target(&e);
                        if let Some(f) = input.files().and_then(|f| f.get(0)) {
                          submit(f);
                        }
                        input.set_value("");
                      }
                    />
                    <label class="flex items-center gap-2 text-sm">
                      <span class="text-xs text-muted-foreground">{t("载波频率（Hz）")}</span>
                      <input
                        type="number"
                        step="10"
                        min="200"
                        max="3000"
                        prop:value=move || center.get().to_string()
                        on:input=move |e| {
                          if let Ok(v) = event_target_value(&e).parse::<f64>() {
                            center.set(v);
                          }
                        }
                        class=input_class("h-9 w-28")
                      />
                    </label>
                  </div>
                }
              })
          }}

          {move || {
            processing.get().then(|| {
              view! {
                <div class="flex flex-col items-center justify-center gap-3 py-8">
                  <Icon kind=IconKind::Loader2 class="h-8 w-8 animate-spin text-primary" />
                  <span class="text-sm text-muted-foreground">{t("正在解码，请稍候…")}</span>
                </div>
              }
            })
          }}

          {move || {
            error.get().map(|e| {
              view! {
                <div class="flex items-start gap-2 rounded-lg border border-red-200 bg-red-50 p-4 text-red-800 dark:border-red-900 dark:bg-red-950/40 dark:text-red-200">
                  <Icon kind=IconKind::AlertCircle class="mt-0.5 h-5 w-5 shrink-0" />
                  <div class="flex-1">
                    <div class="font-medium">{t("解码失败")}</div>
                    <div class="mt-1 text-sm">{e}</div>
                  </div>
                </div>
              }
            })
          }}

          {move || {
            let text = result.get();
            (!text.is_empty()).then(|| {
              view! {
                <div class="space-y-3">
                  <div class="flex items-center justify-between gap-2">
                    <span class="text-sm text-muted-foreground">{tf("已解码 · {}", &[&file_name.get().to_string()])}</span>
                    <button type="button" class=button_class(Variant::Outline, Size::Sm, "") on:click=reset>
                      {t("重新选择")}
                    </button>
                  </div>
                  <pre class="whitespace-pre-wrap break-words rounded-lg border bg-muted/40 p-4 text-sm font-mono">{text}</pre>
                </div>
              }
            })
          }}
        </section>
      </div>
    </div>
  }
}
