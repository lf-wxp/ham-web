//! RTTY 无线电传解码器页面：上传 RTTY 录音（WAV），交由 Web Worker 后台解码出文本。

use leptos::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::File;

use crate::components::common::{PageContainer, PageHeader};
use crate::decode_session::{DecodeConfig, DecodeSession};
use crate::icons::{Icon, IconKind};
use crate::ui::{Button, ButtonLink, Field, FileDropZone, NumberField, NumberKind, Size, Variant};
use crate::util::set_title;

use super::rtty_worker::parse_message;
use crate::i18n::{t, tf};

/// 允许的最大 WAV 体积（字节）。几分钟的 12 kHz 单声道音频远小于 8 MB。
const MAX_BYTES: u64 = 8 * 1024 * 1024;

#[component]
pub fn RttyDecoderPage() -> impl IntoView {
  set_title("tools.rtty-decoder");

  let session = DecodeSession::new(DecodeConfig {
    codec: "rtty",
    max_bytes: MAX_BYTES,
    transfer: false,
    parse: parse_message,
    too_large: |mb, max| tf("tools.file-too-large-about-2", &[mb, max]),
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
  // 输入框的原文：用 `NumberKind::Decimal` 保留敲到一半的内容（`2125.`、清空），不做 f64 往返；
  // 解码时才解析校验。
  let mark_hz_text = RwSignal::new("2125".to_owned());
  // 本次解码实际使用的传号频率：结果区显示的是快照，避免解码后再改输入框时显示一个
  // 没有参与本次解码的值。
  let decoded_mark_hz = RwSignal::new(2125.0);

  let cancel = move |_| session.cancel();
  let reset = move |_| session.reset();
  let on_file = Callback::new(move |file: File| {
    session.start(file, move || {
      let Some(mark) = mark_hz_text
        .get_untracked()
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v > 0.0)
      else {
        return Err(tf(
          "tools.decode-err-mark-hz",
          &[mark_hz_text.get_untracked().trim()],
        ));
      };
      decoded_mark_hz.set(mark);
      let params = js_sys::Object::new();
      let _ = js_sys::Reflect::set(&params, &"markHz".into(), &JsValue::from_f64(mark));
      Ok(params.into())
    });
  });

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("tools.rtty-decoder")
        subtitle=move || t("tools.rtty-decoder-subtitle")
        actions=ViewFn::from(move || {
          view! {
            <ButtonLink href="/rtty" variant=Variant::Outline size=Size::Sm>
              {move || t("tools.rtty-reference")}
            </ButtonLink>
          }
        })
      />

      <PageContainer class="space-y-5">
        <section class="border bg-card p-4 text-sm text-muted-foreground">
          <p>
            {move || t("tools.upload-a-rtty-recording")}
            {move || t("common.decoding-runs-in-a")}
            {move || t("common.the-audio-is-never")}
          </p>
        </section>

        <section class="border bg-card p-4">
          {{
            let base_id = crate::util::unique_id("rtty-mark");
            let label_for = base_id.clone();
            view! {
              <Field
                label=Signal::derive(move || t("tools.mark-frequency-hz-default"))
                r#for=label_for
                class="mb-4 sm:max-w-xs"
              >
                <NumberField
                  id=base_id
                  value=Signal::derive(move || mark_hz_text.get())
                  kind=NumberKind::Decimal
                  on_change=Callback::new(move |v: String| mark_hz_text.set(v))
                  controls=false
                />
                <span class="text-xs text-muted-foreground">
                  {move || t("tools.rtty-space-is-mark")}
                </span>
              </Field>
            }
          }}

          {move || {
            (result.with(Option::is_none) && !processing.get())
              .then(|| {
                view! {
                  <FileDropZone
                    on_file=on_file
                    accept=".wav,audio/wav,audio/x-wav,audio/*"
                    title=Signal::derive(move || t("tools.choose-a-rtty-recording"))
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
                    {tf(
                      "tools.rtty-result-meta",
                      &[&file_name.get_untracked(), &format!("{:.0}", decoded_mark_hz.get())],
                    )}
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
