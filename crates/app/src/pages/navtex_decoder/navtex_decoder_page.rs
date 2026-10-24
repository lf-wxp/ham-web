//! NAVTEX 解码器页面：上传 NAVTEX 录音（WAV），交由 Web Worker 后台解码出海事安全报文。

use leptos::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::File;

use crate::components::common::{DecodeElapsed, PageContainer, PageHeader};
use crate::decode_session::{DecodeConfig, DecodeSession};
use crate::icons::{Icon, IconKind};
use crate::ui::{Button, ButtonLink, FileDropZone, Size, Variant};
use crate::util::set_title;

use super::navtex_worker::parse_message;
use crate::i18n::{t, tf};

/// 允许的最大 WAV 体积（字节）。
const MAX_BYTES: u64 = 8 * 1024 * 1024;

#[component]
pub fn NavtexDecoderPage() -> impl IntoView {
  set_title("tools.navtex-decoder");

  let session = DecodeSession::new(DecodeConfig {
    codec: "navtex",
    max_bytes: MAX_BYTES,
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

  let cancel = move |_| session.cancel();
  let reset = move |_| session.reset();
  let on_file = Callback::new(move |file: File| session.start(file, || Ok(JsValue::UNDEFINED)));

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("tools.navtex-decoder")
        subtitle=move || t("tools.navtex-decoder-subtitle")
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
            {move || t("tools.upload-a-navtex-recording")}
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
                    accept="audio/wav,.wav"
                    title=Signal::derive(move || t("tools.choose-a-navtex-recording"))
                    icon=IconKind::Anchor
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
            result.get().map(|messages| {
              let count = messages.len();
              view! {
                <div class="space-y-3">
                  <div class="flex items-center justify-between gap-3">
                    <p class="min-w-0 truncate text-sm">{file_name.get()}</p>
                    <Button variant=Variant::Outline on_click=reset>
                      {move || t("tools.decode-another-file")}
                    </Button>
                  </div>
                  <p class="text-xs text-muted-foreground">
                    {tf("tools.navtex-message-count", &[&count.to_string()])}
                  </p>
                  {messages
                    .into_iter()
                    .map(|m| {
                      let header = format!("{}{} {:02}", m.transmitter, m.category, m.serial);
                      view! {
                        <div class="border bg-inset p-4">
                          <div class="mb-2 flex flex-wrap items-center gap-2 text-sm">
                            <span class="font-mono">{header}</span>
                            <span
                              class=format!(
                                "px-1.5 py-0.5 text-xs {}",
                                if m.accepted {
                                  "bg-win/15 text-win-text"
                                } else {
                                  "bg-destructive/10 text-hp-text"
                                }
                              )
                            >
                              {if m.accepted {
                                t("tools.navtex-accepted")
                              } else {
                                t("tools.navtex-rejected")
                              }}
                            </span>
                            <span class="text-xs text-muted-foreground">
                              {tf(
                                "tools.navtex-stats",
                                &[
                                  &m.chars.to_string(),
                                  &m.corrected.to_string(),
                                  &m.errors.to_string(),
                                ],
                              )}
                            </span>
                          </div>
                          <pre class="whitespace-pre-wrap break-all font-mono text-sm">{m.text.clone()}</pre>
                        </div>
                      }
                    })
                    .collect_view()}
                </div>
              }
            })
          }}
        </section>
      </PageContainer>
    </div>
  }
}
