//! Feld Hell 解码器页面：上传 Feld Hell 录音（WAV），交由 Web Worker 后台解码出文本。

use leptos::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::File;

use crate::components::common::{DecodeElapsed, PageContainer, PageHeader};
use crate::decode_session::{DecodeConfig, DecodeSession, MAX_UPLOAD_BYTES};
use crate::icons::{Icon, IconKind};
use crate::ui::{Button, ButtonLink, FileDropZone, Size, Variant};
use crate::util::set_title;

use super::hell_worker::parse_message;
use crate::i18n::t;

#[component]
pub fn HellDecoderPage() -> impl IntoView {
  set_title("tools.hell-decoder");

  let session = DecodeSession::new(DecodeConfig {
    codec: "hell",
    max_bytes: MAX_UPLOAD_BYTES,
    transfer: false,
    parse: parse_message,
    too_large: |mb, max| crate::i18n::tf("tools.file-too-large-downsample", &[mb, max]),
    timed_out: || t("tools.decoding-timed-out"),
    failed: || t("tools.decoding-failed-generic"),
  });
  let DecodeSession {
    processing,
    result,
    error,
    file_name,
    reading,
    ..
  } = session;

  let cancel = move |_| session.cancel();
  let reset = move |_| session.reset();
  // Feld Hell 无参数：页面侧发 `null`。
  let on_file = Callback::new(move |file: File| {
    session.start(file, move || Ok(JsValue::NULL));
  });

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("tools.hell-decoder")
        subtitle=move || t("tools.hell-decoder-subtitle")
        actions=ViewFn::from(move || {
          view! {
            <ButtonLink href="/modes" variant=Variant::Outline size=Size::Sm>
              {move || t("radio.digital-modes-reference")}
            </ButtonLink>
          }
        })
      />

      <PageContainer class="space-y-5">
        <section class="border bg-card p-4 text-sm text-muted-foreground">
          <p>
            {move || t("tools.upload-a-hell-recording")}
            {move || t("common.decoding-runs-in-a")}
            {move || t("common.the-audio-is-never")}
          </p>
        </section>

        <section class="border bg-card p-4">
          {move || {
            (result.with(Option::is_none) && !processing.get())
              .then(|| {
                view! {
                  <FileDropZone
                    on_file=on_file
                    accept=".wav,audio/wav,audio/x-wav,audio/*"
                    title=Signal::derive(move || t("tools.choose-a-hell-recording"))
                    icon=IconKind::Waves
                  />
                }
              })
          }}

          {move || {
            processing
              .get()
              .then(|| {
                view! {
                  <div role="status" class="flex flex-col items-center justify-center gap-3 py-8">
                    <Icon kind=IconKind::Loader2 class="h-8 w-8 animate-spin text-primary" />
                    <span class="text-sm text-muted-foreground">
                      {move || {
                        if reading.get() {
                          t("tools.reading-file")
                        } else {
                          t("radio.decoding-in-the-background")
                        }
                      }}
                    </span>
                    <DecodeElapsed ms=session.elapsed_ms />
                    <Button variant=Variant::Outline on_click=cancel>
                      {move || t("exam.cancel")}
                    </Button>
                  </div>
                }
              })
          }}

          {move || {
            error.get().map(|e| {
              view! {
                <div
                  role="alert"
                  class="flex items-start gap-2 border border-destructive/40 bg-destructive/5 p-4 text-destructive"
                >
                  <Icon kind=IconKind::AlertCircle class="mt-0.5 h-5 w-5 shrink-0" />
                  <div class="flex-1">
                    <div class="font-medium">{move || t("radio.decoding-failed")}</div>
                    <div class="mt-1 text-sm">{e}</div>
                  </div>
                </div>
              }
            })
          }}

          {move || {
            result.get().map(|p| {
              view! {
                <div class="space-y-4">
                  <div class="bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
                    {crate::i18n::tf("radio.decoded", &[&file_name.get_untracked()])}
                  </div>
                  <div class="border bg-card p-6">
                    <div class="whitespace-pre-wrap break-words font-mono text-lg leading-relaxed">
                      {p.text}
                    </div>
                  </div>
                  <div class="flex justify-center">
                    <Button variant=Variant::Outline on_click=move |_| reset(())>
                      <Icon kind=IconKind::RefreshCw class="size-6" />
                      {move || t("tools.decode-another-file")}
                    </Button>
                  </div>
                </div>
              }
            })
          }}
        </section>
      </PageContainer>
    </div>
  }
}
