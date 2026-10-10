//! PSK31 解码器：上传 PSK31 录音（WAV），在浏览器本地解调为文本。
//!
//! 解码复用 `ham-web-core::psk31`（纯 Rust DSP），WAV 解析复用 `ham-web-apt`，音频不上传服务器。

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use web_sys::{DragEvent, File, HtmlInputElement};

use crate::components::common::{PageContainer, PageHeader};
use crate::icons::{Icon, IconKind};
use crate::ui::{Button, NumberField, Size, Variant};
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
  set_title("radio.psk31-decoder");

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
              error.set(Some(t("radio.no-text-decoded-make")));
            } else {
              result.set(text);
            }
          }
          Err(_) => error.set(Some(t("radio.not-a-valid-wav"))),
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
      <PageHeader
        title=t("radio.psk31-decoder")
        subtitle=t("radio.bpsk-31-25-baud")
        actions=ViewFn::from(move || {
          view! {
            <a
              href="/modes"
              class="rounded-md border px-2.5 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
            >
              {t("radio.digital-modes-reference")}
            </a>
          }
        })
      />

      <PageContainer class="space-y-5">
        <section class="rounded-xl border bg-card p-4 text-sm text-muted-foreground">
          <p>
            {t("radio.upload-a-psk31-audio")}
            " "
            {t("radio.the-carrier-is-usually")}
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
                      <span class="text-sm font-medium">{t("radio.select-psk31-recording-wav")}</span>
                      <span class="text-xs text-muted-foreground">{t("tools.click-to-choose-or-2")}</span>
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
                      <span class="text-xs text-muted-foreground">{t("radio.carrier-frequency-hz")}</span>
                      <NumberField
                        value=Signal::derive(move || center.get().to_string())
                        on_change=Callback::new(move |v: String| {
                          if let Ok(v) = v.trim().parse::<f64>() {
                            center.set(v);
                          }
                        })
                        step=10.0
                        min=200.0
                        max=3000.0
                        class="w-28"
                        controls=false
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
                  <span class="text-sm text-muted-foreground">{t("radio.decoding-please-wait")}</span>
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
                    <div class="font-medium">{t("radio.decoding-failed")}</div>
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
                    <span class="text-sm text-muted-foreground">{tf("radio.decoded", &[&file_name.get().to_string()])}</span>
                    <Button
                      variant=Variant::Outline
                      size=Size::Sm
                      on_click=Callback::new(move |_| reset(()))
                    >
                      {t("radio.choose-another")}
                    </Button>
                  </div>
                  <pre class="whitespace-pre-wrap break-words rounded-lg border bg-muted/40 p-4 text-sm font-mono">{text}</pre>
                </div>
              }
            })
          }}
        </section>
      </PageContainer>
    </div>
  }
}
