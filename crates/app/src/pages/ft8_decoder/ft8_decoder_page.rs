//! FT8 / FT4 弱信号数字模式解码器页面：上传录音（WAV，12000 Hz，建议从时隙起点开始），
//! 交由 Web Worker 按时隙逐个解码出消息列表。

use leptos::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::File;

use crate::components::common::{PageContainer, PageHeader};
use crate::decode_session::{DecodeConfig, DecodeSession};
use crate::icons::{Icon, IconKind};
use crate::ui::{
  Button, ButtonLink, Field, FileDropZone, NativeSelect, SelectOption, Size, Variant,
};
use crate::util::set_title;

use super::ft8_worker::parse_message;
use crate::i18n::{t, tf, tp};

/// 允许的最大 WAV 体积（字节）。12 kHz 16 位单声道约 23 KB/s，8 MB 约能装 23 个 FT8 时隙。
const MAX_BYTES: u64 = 8 * 1024 * 1024;

#[component]
pub fn Ft8DecoderPage() -> impl IntoView {
  set_title("tools.ft8-decoder");

  let session = DecodeSession::new(DecodeConfig {
    codec: "ft8",
    max_bytes: MAX_BYTES,
    transfer: false,
    parse: parse_message,
    too_large: |mb, max| tf("tools.ft8-file-too-large", &[mb, max]),
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
  // 协议选择：FT8 / FT4。
  let protocol = RwSignal::new("ft8".to_owned());
  // 本次解码实际使用的协议：结果区显示快照，避免解码后再改下拉时显示没参与解码的值。
  let decoded_protocol = RwSignal::new("FT8".to_owned());

  let cancel = move |_| session.cancel();
  let reset = move |_| session.reset();
  let on_file = Callback::new(move |file: File| {
    session.start(file, move || {
      let proto = protocol.get_untracked();
      decoded_protocol.set(if proto == "ft4" {
        "FT4".to_owned()
      } else {
        "FT8".to_owned()
      });
      let params = js_sys::Object::new();
      let _ = js_sys::Reflect::set(&params, &"protocol".into(), &JsValue::from_str(&proto));
      Ok(params.into())
    });
  });

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("tools.ft8-decoder")
        subtitle=move || t("tools.ft8-decoder-subtitle")
        actions=ViewFn::from(move || {
          view! {
            <ButtonLink href="/ft8" variant=Variant::Outline size=Size::Sm>
              {move || t("tools.ft8-reference")}
            </ButtonLink>
          }
        })
      />

      <PageContainer class="space-y-5">
        <section class="border bg-card p-4 text-sm text-muted-foreground">
          <p>
            {move || t("tools.upload-an-ft8-recording")}
            {move || t("common.decoding-runs-in-a")}
            {move || t("common.the-audio-is-never")}
          </p>
        </section>

        <section class="border bg-card p-4">
          {{
            let base_id = crate::util::unique_id("ft8-protocol");
            let label_for = base_id.clone();
            let options: Vec<SelectOption> = ["ft8", "ft4"]
              .into_iter()
              .map(|p| SelectOption::new(p.to_owned(), p.to_uppercase()))
              .collect();
            view! {
              <Field
                label=Signal::derive(move || t("common.protocol"))
                r#for=label_for
                class="mb-4 sm:max-w-xs"
              >
                <NativeSelect
                  id=base_id
                  value=protocol
                  on_change=Callback::new(move |v: String| protocol.set(v))
                  options=options
                  aria_label=Signal::derive(move || t("common.protocol"))
                />
              </Field>
            }
          }}

          {move || {
            (result.with(Option::is_none) && !processing.get())
              .then(|| {
                view! {
                  <FileDropZone
                    on_file=on_file
                    accept=".wav,audio/wav,audio/x-wav"
                    title=Signal::derive(move || t("tools.choose-an-ft8-recording"))
                    icon=IconKind::Waves
                    id="ft8-file"
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
              let unsupported = p.unsupported;
              let slots = p.slots;
              view! {
                <div class="space-y-4">
                  <div class="bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
                    {tf(
                      "tools.ft8-result-meta",
                      &[&file_name.get_untracked(), &decoded_protocol.get(), &format!("{}", p.messages.len())],
                    )}
                    {(slots > 1)
                      .then(|| {
                        view! {
                          <span>{" · "}{tf("tools.ft8-slots", &[&slots.to_string()])}</span>
                        }
                      })}
                  </div>
                  {(unsupported > 0)
                    .then(|| {
                      view! {
                        <div class="bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
                          {tp(
                            "tools.ft8-unsupported",
                            unsupported,
                            &[&unsupported.to_string()],
                          )}
                        </div>
                      }
                    })}
                  {if p.messages.is_empty() {
                    view! {
                      <div class="border bg-card p-6 text-sm text-muted-foreground">
                        {move || t("tools.ft8-no-messages")}
                      </div>
                    }
                    .into_any()
                  } else {
                    view! {
                      <div class="border bg-card">
                        <div class="divide-y">
                          {p
                            .messages
                            .iter()
                            .map(|m| {
                              let freq = format!("{:.0}", m.freq_hz);
                              let time = format!("{:.1}", m.time_sec);
                              let sync = format!("{:+.1}", m.sync_db);
                              let text = m.text.clone();
                              view! {
                                <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1 border-b px-3 py-2 text-sm last:border-0">
                                  <span class="font-mono font-semibold text-primary">{text}</span>
                                  <span class="ml-auto shrink-0 font-mono text-xs text-muted-foreground">
                                    {tf("tools.ft8-message-meta", &[&freq, &time, &sync])}
                                  </span>
                                </div>
                              }
                            })
                            .collect_view()}
                        </div>
                      </div>
                    }
                    .into_any()
                  }}
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
