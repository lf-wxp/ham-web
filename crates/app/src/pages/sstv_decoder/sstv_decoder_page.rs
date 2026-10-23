//! SSTV 慢扫描电视解码器页面：上传 SSTV 录音，交由 Web Worker 在后台线程解码。

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{File, HtmlCanvasElement, ImageData};

use crate::components::common::{DecodeElapsed, PageContainer, PageHeader};
use crate::decode_session::{DecodeConfig, DecodeSession};
use crate::icons::{Icon, IconKind};
use crate::ui::{Button, ButtonLink, FileDropZone, Size, Variant};
use crate::util::{document, js_error_message, set_title};

use super::sstv_worker::parse_message;
use crate::i18n::{t, tf};

/// 允许的最大 WAV 体积（字节）。SSTV 单张图像约 1–2 分钟，32 MB 足够。
const MAX_BYTES: u64 = 32 * 1024 * 1024;

/// 渲染结果：RGB 图像（PNG data URL）+ 元信息。
#[derive(Clone)]
struct SstvResult {
  width: u32,
  height: u32,
  mode: String,
  source_sample_rate: u32,
  duration_seconds: f64,
  image_url: String,
}

/// RGBA 像素 → 离屏 canvas → PNG data URL。
fn render_rgba(rgba: &[u8], width: u32, height: u32) -> Result<String, String> {
  let canvas: HtmlCanvasElement = document()
    .create_element("canvas")
    .map_err(|e| js_error_message(&e))?
    .unchecked_into();
  canvas.set_width(width);
  canvas.set_height(height);
  let ctx = canvas
    .get_context("2d")
    .map_err(|e| js_error_message(&e))?
    .ok_or_else(|| t("common.canvas-2d-context-not"))?
    .dyn_into::<web_sys::CanvasRenderingContext2d>()
    .map_err(|_| t("common.canvas-2d-context-not"))?;

  let img = ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(rgba), width, height)
    .map_err(|e| js_error_message(&e))?;
  ctx
    .put_image_data(&img, 0.0, 0.0)
    .map_err(|e| js_error_message(&e))?;
  canvas.to_data_url().map_err(|e| js_error_message(&e))
}

/// 创建消息回调（只创建一次并复用）。
/// 解析 Worker 结果并把 RGBA 像素画成 PNG data URL；渲染失败当作解码失败。
fn parse_and_render(raw: &JsValue) -> Result<SstvResult, String> {
  let p = parse_message(raw)?;
  let image_url = render_rgba(&p.rgba, p.width, p.height)?;
  Ok(SstvResult {
    width: p.width,
    height: p.height,
    mode: p.mode,
    source_sample_rate: p.source_sample_rate,
    duration_seconds: p.duration_seconds,
    image_url,
  })
}

#[component]
pub fn SstvDecoderPage() -> impl IntoView {
  set_title("tools.sstv-decoder");

  let session = DecodeSession::new(DecodeConfig {
    codec: "sstv",
    max_bytes: MAX_BYTES,
    transfer: false,
    parse: parse_and_render,
    too_large: |mb, max| tf("tools.file-too-large-image-segment", &[mb, max]),
    timed_out: || t("tools.decoding-timed-out"),
    failed: || t("tools.decoding-failed-make-sure"),
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
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("tools.sstv-decoder")
        subtitle=move || t("tools.audio-fm-demodulation-vis")
        actions=ViewFn::from(move || {
          view! {
            <ButtonLink href="/sstv" variant=Variant::Outline size=Size::Sm>
              {move || t("tools.sstv-reference")}
            </ButtonLink>
          }
        })
      />

      <PageContainer class="space-y-5">
        <section class="border bg-card p-4 text-sm text-muted-foreground">
          <p>
            {move || t("tools.upload-an-sstv-audio")}
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
                    title=Signal::derive(move || t("tools.choose-an-sstv-recording"))
                    icon=IconKind::Camera
                    hint=Signal::derive(move || t("tools.supports-martin-m1-m2"))
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
                    <DecodeElapsed ms=session.elapsed_ms />
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
            result.get().map(|r| {
              view! {
                <div class="space-y-4">
                  <div class="flex flex-wrap gap-x-1.5 gap-y-1 bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
                    <span class="max-w-[16rem] truncate font-medium text-foreground">{file_name.get_untracked()}</span>
                    <span>"·"</span>
                    <span class="font-medium text-foreground">{r.mode}</span>
                    <span>"·"</span>
                    <span>{format!("{} × {}", r.width, r.height)}</span>
                    <span>"·"</span>
                    <span>{format!("{} Hz", r.source_sample_rate)}</span>
                    <span>"·"</span>
                    <span>{format!("{:.1} s", r.duration_seconds)}</span>
                  </div>
                  <img
                    src=r.image_url.clone()
                    alt=t("tools.sstv-decode-result")
                    class="mx-auto w-full max-w-full border"
                  />
                  <div class="flex justify-center gap-2">
                    <ButtonLink href=r.image_url.clone() download="sstv.png">
                      <Icon kind=IconKind::Download class="size-6" />
                      {move || t("tools.download-png")}
                    </ButtonLink>
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
