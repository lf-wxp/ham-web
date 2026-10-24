//! NOAA APT 解码器页面：上传 APT 录音，交由 Web Worker 在后台线程解码。

use leptos::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::File;

use crate::components::common::{DecodeElapsed, PageContainer, PageHeader};
use crate::decode_session::{DecodeConfig, DecodeSession};
use crate::icons::{Icon, IconKind};
use crate::ui::{Button, ButtonLink, FileDropZone, Size, Variant};
use crate::util::set_title;

use super::apt_result_view::{AptRender, AptResultView};
use super::apt_worker::parse_message;
use crate::i18n::{t, tf};

/// 允许的最大 WAV 体积（字节）。
///
/// 解码会在 Worker 里把整段音频读入内存并重采样到 20800 Hz（每采样一个 `f32`），
/// 峰值内存可达文件体积的数倍。不设上限时，误选一个几百 MB 的录音会直接把标签页
/// 拖垮；96 MB 足够容纳 44.1 kHz 单声道约 15 分钟 —— 正好是一次典型过境的长度。
const MAX_BYTES: u64 = 96 * 1024 * 1024;

/// 解析 Worker 结果并把像素渲染成三张 PNG data URL；渲染失败当作解码失败。
fn parse_and_render(raw: &JsValue) -> Result<AptRender, String> {
  let p = parse_message(raw)?;
  AptRender::new(
    p.width,
    p.height,
    p.lines,
    p.source_sample_rate,
    p.duration_seconds,
    p.channel_a,
    p.channel_b,
    p.false_color,
  )
}

#[component]
pub fn AptDecoderPage() -> impl IntoView {
  set_title("radio.noaa-apt-decoder");

  // 会话状态机（代际 / 超时 / 取消 / 卸载清理）与其余七个解码页共用 `DecodeSession`。
  let session = DecodeSession::new(DecodeConfig {
    codec: "apt",
    max_bytes: MAX_BYTES,
    transfer: false,
    parse: parse_and_render,
    too_large: |mb, max| tf("tools.file-too-large-about", &[mb, max]),
    timed_out: || t("tools.decoding-timed-out"),
    failed: || t("radio.decoding-failed-make-sure"),
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
  let on_file = Callback::new(move |file: File| session.start(file, || Ok(JsValue::NULL)));

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("radio.noaa-apt-decoder")
        subtitle=move || t("radio.audio-am-demodulation-image")
        actions=ViewFn::from(move || {
          view! {
            <ButtonLink href="/weather-sat" variant=Variant::Outline size=Size::Sm>
              {move || t("radio.receiving-quick-reference")}
            </ButtonLink>
          }
        })
      />

      <PageContainer class="space-y-5">
        <section class="border bg-card p-4 text-sm text-muted-foreground">
          <p>
            {move || t("radio.upload-apt-audio-received")}
            {move || t("common.decoding-runs-in-a")}
            {move || t("common.a-typical-pass-lasts")}
            <a href="/satellites" class="text-primary underline underline-offset-2">{move || t("radio.pass-predictions")}</a>
            {move || t("radio.record-in-advance-the")}
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
                    title=Signal::derive(move || t("tools.choose-an-apt-recording"))
                    icon=IconKind::Satellite
                    hint=Signal::derive(move || t("tools.recommended-record-with-rtl"))
                    disabled=processing
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
                    <Icon kind=IconKind::Loader2 class="size-8 animate-spin text-primary" />
                    <span class="text-sm text-muted-foreground">
                      {move || {
                        if reading.get() {
                          t("tools.reading-file")
                        } else {
                          t("radio.decoding-in-the-background")
                        }
                      }}
                    </span>
                    {move || {
                      (!file_name.get_untracked().is_empty())
                        .then(|| {
                          view! {
                            <span class="max-w-full truncate text-xs text-muted-foreground">{file_name.get_untracked()}</span>
                          }
                        })
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
                <div class="flex items-start gap-2 border border-destructive/40 bg-destructive/10 p-4 text-hp-text">
                  <Icon kind=IconKind::AlertCircle class="mt-0.5 size-6 shrink-0" />
                  <div class="flex-1">
                    <div>{move || t("radio.decoding-failed")}</div>
                    <div class="mt-1 text-sm">{e}</div>
                  </div>
                </div>
              }
            })
          }}

          {move || {
            result.get().map(|r| {
              view! {
                <AptResultView render=r file_name=file_name.get_untracked() on_reset=Callback::new(reset) />
              }
            })
          }}
        </section>
      </PageContainer>
    </div>
  }
}
