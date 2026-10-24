//! PSK31 解码器：上传 PSK31 录音（WAV），在浏览器本地解调为文本。
//!
//! 解码复用 `ham-web-core::psk31`（纯 Rust DSP），WAV 解析复用 `ham-web-apt`，音频不上传服务器。

use leptos::prelude::*;
use leptos::task::spawn_local;
use web_sys::File;

use crate::components::common::{PageContainer, PageHeader};
use crate::icons::{Icon, IconKind};
use crate::ui::{Button, FileDropZone, NumberField, Size, Variant};
use crate::util::set_title;

use crate::i18n::{t, tf};

#[component]
pub fn PskDecodePage() -> impl IntoView {
  set_title("radio.psk31-decoder");

  let processing = RwSignal::new(false);
  let result = RwSignal::new(String::new());
  let error = RwSignal::new(None::<String>);
  let file_name = RwSignal::new(String::new());
  let center = RwSignal::new(1000.0f64);

  let decode = move |file: File| {
    processing.set(true);
    error.set(None);
    result.set(String::new());
    file_name.set(file.name());
    let c = center.get_untracked();
    spawn_local(async move {
      match crate::decode_worker::read_file_bytes(&file).await {
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

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=t("radio.psk31-decoder")
        subtitle=t("radio.bpsk-31-25-baud")
        actions=ViewFn::from(move || {
          view! {
            <a
              href="/modes"
              class="border px-2.5 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
            >
              {t("radio.digital-modes-reference")}
            </a>
          }
        })
      />

      <PageContainer class="space-y-5">
        <section class="pxl-card border bg-card p-4 text-sm text-muted-foreground">
          <p>
            {t("radio.upload-a-psk31-audio")}
            " "
            {t("radio.the-carrier-is-usually")}
          </p>
        </section>

        <section class="pxl-card border bg-card p-4">
          {move || {
            (result.with(String::is_empty) && error.with(Option::is_none) && !processing.get())
              .then(|| {
                view! {
                  <div class="space-y-3">
                    <FileDropZone
                      on_file=Callback::new(move |file: File| submit(file))
                      accept=".wav,audio/wav,audio/x-wav,audio/*"
                      title=Signal::derive(move || t("radio.select-psk31-recording-wav"))
                      icon=IconKind::AudioLines
                      disabled=processing
                      id="psk-file"
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
                <div class="flex items-start gap-2 border border-destructive/40 bg-destructive/10 p-4 text-hp-text">
                  <Icon kind=IconKind::AlertCircle class="mt-0.5 size-6 shrink-0" />
                  <div class="flex-1">
                    <div>{t("radio.decoding-failed")}</div>
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
                  <pre class="whitespace-pre-wrap break-words border bg-inset p-4 text-sm font-mono">{text}</pre>
                </div>
              }
            })
          }}
        </section>
      </PageContainer>
    </div>
  }
}
