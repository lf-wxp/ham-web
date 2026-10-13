//! Olivia 数字模式解码器页面：上传 Olivia 录音（WAV），交由 Web Worker 后台解码出文本。

use leptos::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::File;

use crate::components::common::{PageContainer, PageHeader};
use crate::decode_session::{DecodeConfig, DecodeSession};
use crate::icons::{Icon, IconKind};
use crate::ui::{
  Button, ButtonLink, Field, FileDropZone, NativeSelect, SelectOption, Size, TextValue, Variant,
};
use crate::util::set_title;

use super::olivia_worker::parse_message;
use crate::i18n::{t, tf};

/// 允许的最大 WAV 体积（字节）。
const MAX_BYTES: u64 = 8 * 1024 * 1024;

/// 支持的子模式（规范命名 = 音调数/带宽；中心频率是各子模式惯例值）。
const MODES: &[(&str, &str)] = &[
  ("32/1000@1500", "32/1000"),
  ("16/500@1000", "16/500"),
  ("8/250@750", "8/250"),
  ("4/125@750", "4/125"),
];

#[component]
pub fn OliviaDecoderPage() -> impl IntoView {
  set_title("tools.olivia-decoder");

  let session = DecodeSession::new(DecodeConfig {
    codec: "olivia",
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
  let mode = RwSignal::new("32/1000@1500".to_owned());

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
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("tools.olivia-decoder")
        subtitle=move || t("tools.olivia-decoder-subtitle")
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
            {move || t("tools.upload-an-olivia-recording")}
            {move || t("common.decoding-runs-in-a")}
            {move || t("common.the-audio-is-never")}
          </p>
        </section>

        <section class="border bg-card p-4">
          {{
            let mode_id = crate::util::unique_id("olivia-mode");
            let label_for = mode_id.clone();
            view! {
              <Field
                label=Signal::derive(move || t("tools.olivia-submode"))
                r#for=label_for
                class="mb-4 sm:max-w-xs"
              >
                <NativeSelect
                  id=mode_id
                  value=Signal::derive(move || mode.get())
                  on_change=Callback::new(move |v: String| mode.set(v))
                  options=mode_options.clone()
                />
                <span class="text-xs text-muted-foreground">
                  {move || t("tools.olivia-submode-hint")}
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
                    accept="audio/wav,.wav"
                    title=Signal::derive(move || t("tools.choose-an-olivia-recording"))
                    icon=IconKind::Radio
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
                  class="border border-destructive/40 bg-destructive/5 px-4 py-3 text-sm text-destructive"
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
                    <p class="min-w-0 truncate text-sm font-medium">{file_name.get()}</p>
                    <Button variant=Variant::Outline on_click=reset>
                      {move || t("tools.decode-another-file")}
                    </Button>
                  </div>
                  <pre class="whitespace-pre-wrap break-all bg-muted/50 p-4 font-mono text-sm">
                    {p.text.clone()}
                  </pre>
                </div>
              }
            })
          }}
        </section>
      </PageContainer>
    </div>
  }
}
