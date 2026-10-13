//! WSPR 弱信号传播报告解码器页面：上传 WSPR 录音，交由 Web Worker 后台解码。

use leptos::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::File;

use crate::components::common::{PageContainer, PageHeader};
use crate::decode_session::{DecodeConfig, DecodeSession};
use crate::icons::{Icon, IconKind};
use crate::util::set_title;

use super::wspr_worker::parse_message;
use crate::i18n::{t, tf};
use crate::ui::{Button, ButtonLink, Field, FileDropZone, NumberField, NumberKind, Size, Variant};

/// 允许的最大 WAV 体积（字节）。110.6 秒的 48 kHz 单声道约 10.6 MB，32 MB 足够。
const MAX_BYTES: u64 = 32 * 1024 * 1024;

#[component]
pub fn WsprDecoderPage() -> impl IntoView {
  set_title("tools.wspr-decoder");

  let session = DecodeSession::new(DecodeConfig {
    codec: "wspr",
    max_bytes: MAX_BYTES,
    transfer: false,
    parse: parse_message,
    too_large: |mb, max| tf("tools.file-too-large-downsample", &[mb, max]),
    timed_out: || t("tools.decoding-timed-out"),
    failed: || t("tools.decoding-failed-wspr"),
  });
  let DecodeSession {
    processing,
    result,
    error,
    file_name,
    reading,
    ..
  } = session;
  // 输入框的原文：用 `NumberKind::Decimal` 保留敲到一半的内容，不做 f64 往返；解码时才解析校验。
  let base_hz_text = RwSignal::new("1500".to_owned());
  // 本次解码实际使用的中心频率：结果区显示的是快照，而不是解码后再改输入框的值。
  let decoded_base_hz = RwSignal::new(1500.0);

  let cancel = move |_| session.cancel();
  let reset = move |_| session.reset();
  let on_file = Callback::new(move |file: File| {
    session.start(file, move || {
      let Some(base) = base_hz_text
        .get_untracked()
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v > 0.0)
      else {
        return Err(tf(
          "tools.decode-err-base-hz",
          &[base_hz_text.get_untracked().trim()],
        ));
      };
      decoded_base_hz.set(base);
      let params = js_sys::Object::new();
      let _ = js_sys::Reflect::set(&params, &"baseHz".into(), &JsValue::from_f64(base));
      Ok(params.into())
    });
  });

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("tools.wspr-decoder")
        subtitle=move || t("tools.weak-signal-propagation-reports")
        actions=ViewFn::from(move || {
          view! {
            <ButtonLink href="/wspr" variant=Variant::Outline size=Size::Sm>
              {move || t("tools.wspr-reference")}
            </ButtonLink>
          }
        })
      />

      <PageContainer class="space-y-5">
        <section class="border bg-card p-4 text-sm text-muted-foreground">
          <p>
            {move || t("tools.upload-a-wspr-recording")}
            {move || t("common.decoding-runs-in-a")}
            {move || t("common.the-audio-is-never")}
          </p>
        </section>

        <section class="border bg-card p-4">
          {{
            // `Field` 的 `r#for` 与控件 `id` 要配对；在块里现生成，避免被外层 `move ||` 闭包
            // 捕获（捕获会让闭包退化成 `FnOnce`）。
            let base_id = crate::util::unique_id("wspr-base");
            let label_for = base_id.clone();
            view! {
              <Field
                label=Signal::derive(move || t("tools.base-frequency-hz-default"))
                r#for=label_for
                class="mb-4 sm:max-w-xs"
              >
                <NumberField
                  id=base_id
                  value=Signal::derive(move || base_hz_text.get())
                  kind=NumberKind::Decimal
                  on_change=Callback::new(move |v: String| base_hz_text.set(v))
                  controls=false
                />
                <span class="text-xs text-muted-foreground">
                  {move || t("tools.wspr-signals-sit-in")}
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
                    title=Signal::derive(move || t("tools.choose-a-wspr-recording"))
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
                  <div class="flex flex-col items-center justify-center gap-3 py-8">
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
                    <Button variant=Variant::Outline size=Size::Sm on_click=move |_| cancel(())>
                      {move || t("exam.cancel")}
                    </Button>
                  </div>
                }
              })
          }}

          {move || {
            error.get().map(|e| {
              view! {
                <div class="flex items-start gap-2 border border-red-200 bg-red-50 p-4 text-red-800 dark:border-red-900 dark:bg-red-950/40 dark:text-red-200">
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
                  <div class="flex flex-wrap gap-x-1.5 gap-y-1 bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
                    <span class="max-w-[16rem] truncate font-medium text-foreground">{file_name.get_untracked()}</span>
                    <span>"·"</span>
                    <span>{format!("{} Hz", decoded_base_hz.get())}</span>
                  </div>
                  <div class="border bg-card p-6 text-center">
                    <div class="text-3xl font-semibold tracking-tight">{p.text}</div>
                    <div class="mt-3 flex flex-wrap justify-center gap-2 text-sm">
                      <span class="bg-muted/60 px-2 py-1">{move || tf("tools.wspr-callsign", &[p.callsign.as_str()])}</span>
                      <span class="bg-muted/60 px-2 py-1">{move || tf("tools.wspr-locator", &[p.locator.as_str()])}</span>
                      <span class="bg-muted/60 px-2 py-1">{move || tf("tools.wspr-power", &[p.power.to_string().as_str()])}</span>
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
