//! LRPT 解码器页面：上传气象卫星过境录音（288 kHz 立体声 I/Q WAV），交由
//! Web Worker 后台解码出三通道图像（R/G/B → 合成彩图）。

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{File, HtmlCanvasElement, ImageData};

use crate::components::common::{DecodeElapsed, PageContainer, PageHeader};
use crate::decode_session::{DecodeConfig, DecodeSession};
use crate::icons::{Icon, IconKind};
use crate::ui::{Button, ButtonLink, FileDropZone, Size, Variant};
use crate::util::{document, js_error_message, set_title};

use super::lrpt_worker::{WorkerPayload, parse_message};
use crate::i18n::{t, tf};

/// 允许的最大录音体积（字节）。过境录音动辄上百 MB，这里按解码耗时设一个
/// 诚实的上限：4.6 MB 的 4 秒样本在浏览器里要算一阵子，完整过境需要更长。
const MAX_BYTES: u64 = 128 * 1024 * 1024;

/// RGB 像素 → 离屏 canvas → PNG data URL。
fn render_rgb(rgb: &[u8], width: u32, height: u32) -> Result<String, String> {
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

  // ImageData 只接受 RGBA（4×w×h），Worker 返回的是 RGB（3×w×h）——
  // 补上 alpha 通道（SSTV 的 RGBA 路径不需要这一步）。
  // 尾部不足 3 字节的残缺像素（协议损坏）直接丢弃，防 chunks(3) 末块越界。
  let rgb = &rgb[..rgb.len() - rgb.len() % 3];
  let pixels = width as usize * height as usize;
  let mut rgba = Vec::with_capacity(pixels * 4);
  for px in rgb.chunks(3) {
    rgba.push(px[0]);
    rgba.push(px[1]);
    rgba.push(px[2]);
    rgba.push(255);
  }
  let img =
    ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(&rgba), width, height)
      .map_err(|e| js_error_message(&e))?;
  ctx
    .put_image_data(&img, 0.0, 0.0)
    .map_err(|e| js_error_message(&e))?;
  canvas.to_data_url().map_err(|e| js_error_message(&e))
}

/// 解析 Worker 结果并把像素画成 PNG data URL；渲染失败当作解码失败。
fn parse_and_render(raw: &JsValue) -> Result<WorkerPayload, String> {
  let mut p = parse_message(raw)?;
  p.image_url = render_rgb(&p.rgb, p.width, p.height)?;
  // 像素已画进 PNG：释放 `width*height*3` 的原始缓冲（视图每次重绘都会 clone
  // 整个 payload，留着它是白白复制几十 MB）。
  p.rgb = Vec::new();
  Ok(p)
}

#[component]
pub fn LrptDecoderPage() -> impl IntoView {
  set_title("tools.lrpt-decoder");

  let session = DecodeSession::new(DecodeConfig {
    codec: "lrpt",
    max_bytes: MAX_BYTES,
    // 128MB 级录音把读到的 ArrayBuffer 直接转移给 Worker，主线程不留副本。
    transfer: true,
    parse: parse_and_render,
    too_large: |mb, max| tf("tools.lrpt-file-too-large", &[mb, max]),
    // 128 MB 上限的完整过境录音解码可以很慢，但超过时限基本可认定 worker 卡死（OOM 等）。
    timed_out: || t("tools.lrpt-timed-out"),
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
        title=move || t("tools.lrpt-decoder")
        subtitle=move || t("tools.lrpt-decoder-subtitle")
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
            {move || t("tools.upload-an-lrpt-recording")}
            {move || t("common.decoding-runs-in-a")}
            {move || t("common.the-audio-is-never")}
          </p>
          <p class="mt-2 text-xs">{move || t("tools.lrpt-experimental-note")}</p>
        </section>

        <section class="border bg-card p-4">
          {move || {
            (result.with(Option::is_none) && !processing.get())
              .then(|| {
                view! {
                  <FileDropZone
                    on_file=on_file
                    accept="audio/wav,.wav"
                    title=Signal::derive(move || t("tools.choose-an-lrpt-recording"))
                    icon=IconKind::RadioTower
                  />
                }
              })
          }}

          {move || {
            (reading.get() || processing.get()).then(|| {
              view! {
                <div
                  role="status"
                  class="flex flex-wrap items-center gap-3 py-8 text-sm text-muted-foreground"
                >
                  <Icon kind=IconKind::Loader2 class="size-6 animate-spin text-primary" />
                  {move || {
                    if reading.get() {
                      t("tools.reading-file")
                    } else {
                      t("radio.decoding-in-the-background")
                    }
                  }}
                  <span class="text-xs">{move || t("tools.lrpt-takes-a-while")}</span>
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
              let dims = format!("{} × {}", p.width, p.height);
              view! {
                <div class="space-y-3">
                  <div class="flex flex-wrap items-center justify-between gap-3">
                    <div class="min-w-0">
                      <p class="truncate text-sm">{file_name.get()}</p>
                      <p class="text-xs text-muted-foreground">{dims}</p>
                    </div>
                    <div class="flex items-center gap-2">
                      <ButtonLink
                        href=p.image_url.clone()
                        download="lrpt.png"
                        variant=Variant::Outline
                        size=Size::Sm
                      >
                        {move || t("tools.download-png")}
                      </ButtonLink>
                      <Button variant=Variant::Outline on_click=reset>
                        {move || t("tools.decode-another-file")}
                      </Button>
                    </div>
                  </div>
                  <img
                    src=p.image_url.clone()
                    alt=t("tools.lrpt-decoder")
                    class="w-full border bg-inset"
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
