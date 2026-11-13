//! MFSK 数字模式解码器页面：上传 MFSK 录音（WAV），交由 Web Worker 后台解码出文本。

use leptos::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::File;

use crate::components::common::{
  DecodeElapsed, DecodeTrackingNote, DecodedPictures, PageContainer, PageHeader,
};
use crate::decode_session::{DecodeConfig, DecodeSession, MAX_UPLOAD_BYTES};
use crate::icons::{Icon, IconKind};
use crate::ui::{
  Button, ButtonLink, Checkbox, Field, FileDropZone, NumberField, NumberKind, SelectOption,
  SimpleSelect, Size, TextValue, Variant,
};
use crate::util::set_title;

use super::mfsk_worker::parse_message;
use crate::i18n::{t, tf};

/// 支持的子模式（16 是默认，也是最常见的）。
const MODES: &[(&str, &str)] = &[
  ("16", "MFSK16"),
  ("8", "MFSK8"),
  ("4", "MFSK4"),
  ("11", "MFSK11"),
  ("22", "MFSK22"),
  ("31", "MFSK31"),
  ("32", "MFSK32"),
  ("64", "MFSK64"),
  ("128", "MFSK128"),
  ("64L", "MFSK64L"),
  ("128L", "MFSK128L"),
];

#[component]
pub fn MfskDecoderPage() -> impl IntoView {
  set_title("tools.mfsk-decoder");

  let session = DecodeSession::new(DecodeConfig {
    codec: "mfsk",
    max_bytes: MAX_UPLOAD_BYTES,
    transfer: false,
    parse: parse_message,
    too_large: |mb, max| tf("tools.file-too-large-downsample", &[mb, max]),
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
  let mode = RwSignal::new("16".to_owned());
  // 中心频率输入框的原文：留空 = 自动检测；`NumberKind::Decimal` 保留敲到一半的内容，解码时才解析校验。
  let center_text = RwSignal::new(String::new());
  // 边带反转：LSB 录音的音调序是反的，听不出来，只能由用户选。
  let reverse = RwSignal::new(false);

  let cancel = move |_| session.cancel();
  let reset = move |_| session.reset();
  let on_file = Callback::new(move |file: File| {
    session.start(file, move || {
      let params = js_sys::Object::new();
      let _ = js_sys::Reflect::set(
        &params,
        &"mode".into(),
        &JsValue::from_str(&mode.get_untracked()),
      );
      let text = center_text.get_untracked();
      let text = text.trim();
      if !text.is_empty() {
        let Some(center) = text
          .parse::<f64>()
          .ok()
          .filter(|v| v.is_finite() && *v > 0.0)
        else {
          return Err(tf("tools.decode-err-center-hz", &[text]));
        };
        let _ = js_sys::Reflect::set(&params, &"centerHz".into(), &JsValue::from_f64(center));
      }
      let _ = js_sys::Reflect::set(
        &params,
        &"reverse".into(),
        &JsValue::from_bool(reverse.get_untracked()),
      );
      Ok(params.into())
    });
  });

  let mode_options: Vec<SelectOption> = MODES
    .iter()
    .map(|(value, label)| SelectOption {
      value: (*value).to_owned(),
      label: TextValue::from((*label).to_owned()),
      group: None,
    })
    .collect();

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("tools.mfsk-decoder")
        subtitle=move || t("tools.mfsk-decoder-subtitle")
        actions=ViewFn::from(move || {
          view! {
            <ButtonLink href="/modes" variant=Variant::Outline size=Size::Sm>
              {move || t("tools.digital-mode-reference")}
            </ButtonLink>
          }
        })
      />

      <PageContainer class="space-y-5">
        <section class="border bg-card p-4 text-sm text-muted-foreground">
          <p>
            {move || t("tools.upload-a-mfsk-recording")}
            {move || t("common.decoding-runs-in-a")}
            {move || t("common.the-audio-is-never")}
          </p>
        </section>

        <section class="border bg-card p-4">
          {{
            let mode_id = crate::util::unique_id("mfsk-mode");
            let mode_for = mode_id.clone();
            let center_id = crate::util::unique_id("mfsk-center");
            let center_for = center_id.clone();
            view! {
              <div class="mb-4 grid gap-4 sm:grid-cols-2">
                <Field
                  label=Signal::derive(move || t("tools.mfsk-submode"))
                  r#for=mode_for
                >
                  <SimpleSelect
                    id=mode_id
                    value=Signal::derive(move || mode.get())
                    on_change=Callback::new(move |v: String| mode.set(v))
                    options=mode_options.clone()
                  />
                  <span class="text-xs text-muted-foreground">
                    {move || t("tools.mfsk-submode-hint")}
                  </span>
                </Field>
                <Field
                  label=Signal::derive(move || t("tools.tone-modem-center"))
                  r#for=center_for
                >
                  <NumberField
                    id=center_id
                    value=Signal::derive(move || center_text.get())
                    kind=NumberKind::Decimal
                    on_change=Callback::new(move |v: String| center_text.set(v))
                    controls=false
                  />
                  <span class="text-xs text-muted-foreground">
                    {move || t("tools.tone-modem-center-hint")}
                  </span>
                </Field>
              </div>
              <label class="mb-4 flex items-start gap-2 text-sm">
                <Checkbox
                  checked=Signal::derive(move || reverse.get())
                  on_change=Callback::new(move |v: bool| reverse.set(v))
                />
                <span>
                  {move || t("tools.tone-modem-reverse")}
                  <span class="block text-xs text-muted-foreground">
                    {move || t("tools.tone-modem-reverse-hint")}
                  </span>
                </span>
              </label>
            }
          }}

          {move || {
            (result.with(Option::is_none) && !processing.get())
              .then(|| {
                view! {
                  <FileDropZone
                    on_file=on_file
                    accept="audio/wav,.wav"
                    title=Signal::derive(move || t("tools.choose-a-mfsk-recording"))
                    icon=IconKind::Waves
                  />
                }
              })
          }}

          {move || {
            (reading.get() || processing.get()).then(|| {
              view! {
                <div
                  role="status"
                  class="flex items-center gap-3 py-8 text-sm text-muted-foreground"
                >
                  <Icon kind=IconKind::Loader2 class="size-6 animate-spin text-primary" />
                  {move || {
                    if reading.get() {
                      t("tools.reading-file")
                    } else {
                      t("radio.decoding-in-the-background")
                    }
                  }}
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
                  class="border border-destructive/40 bg-destructive/5 px-4 py-3 text-sm text-hp-text"
                >
                  {e}
                </div>
              }
            })
          }}

          {move || {
            result.get().map(|p| {
              view! {
                <div class="space-y-3">
                  <div class="flex items-center justify-between gap-3">
                    <p class="min-w-0 truncate text-sm">{file_name.get()}</p>
                    <Button variant=Variant::Outline on_click=reset>
                      {move || t("tools.decode-another-file")}
                    </Button>
                  </div>
                  <pre class="whitespace-pre-wrap break-all bg-inset p-4 font-mono text-sm">
                    {p.text.clone()}
                  </pre>
                  <p class="text-xs text-muted-foreground">
                    {tf(
                      "tools.mfsk-detected",
                      &[&format!("{:.1}", p.center_hz), &p.depth.to_string()],
                    )}
                  </p>
                  <DecodeTrackingNote clock_ppm=p.clock_ppm drift_hz_per_s=p.drift_hz_per_s />
                  <DecodedPictures
                    pictures=p.pictures.clone()
                    skipped=p.picture_skipped
                    file_stem="mfsk-picture"
                  />
                </div>
              }
            })
          }}
        </section>
      </PageContainer>
    </div>
  }
}
