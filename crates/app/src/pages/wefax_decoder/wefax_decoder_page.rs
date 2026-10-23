//! WEFAX 解码器页面：上传 WEFAX 录音（WAV），交由 Web Worker 后台解码出灰度图。

use leptos::prelude::*;
use wasm_bindgen::{Clamped, JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, File, HtmlCanvasElement, ImageData};

use crate::components::common::{DecodeElapsed, PageContainer, PageHeader};
use crate::decode_session::{DecodeConfig, DecodeSession, MAX_UPLOAD_BYTES};
use crate::icons::{Icon, IconKind};
use crate::ui::{Button, ButtonLink, FileDropZone, Size, Variant};
use crate::util::{document, js_error_message, set_title};

use super::wefax_worker::parse_message;
use crate::i18n::{t, tf};

/// 灰度像素 → 离屏 canvas → PNG data URL。
///
/// 峰值内存是灰度数据的 4 倍（`ImageData` 只收 RGBA）：8 kHz 录音按 8 MiB 上限折算约
/// 40 万像素，4× 即 1.6 MB，可以接受；再大的图应该改走 Worker 出图。
fn render_gray(gray: &[u8], width: u32, height: u32) -> Result<String, String> {
  let mut rgba = vec![0u8; gray.len() * 4];
  for (i, &g) in gray.iter().enumerate() {
    let o = i * 4;
    rgba[o] = g;
    rgba[o + 1] = g;
    rgba[o + 2] = g;
    rgba[o + 3] = 255;
  }
  let canvas: HtmlCanvasElement = document()
    .create_element("canvas")
    .map_err(|e| js_error_message(&e))?
    .unchecked_into();
  canvas.set_width(width);
  canvas.set_height(height);
  let ctx: CanvasRenderingContext2d = canvas
    .get_context("2d")
    .map_err(|e| js_error_message(&e))?
    .ok_or_else(|| t("common.canvas-2d-context-not"))?
    .unchecked_into();
  let img = ImageData::new_with_u8_clamped_array_and_sh(Clamped(&rgba), width, height)
    .map_err(|e| js_error_message(&e))?;
  ctx
    .put_image_data(&img, 0.0, 0.0)
    .map_err(|e| js_error_message(&e))?;
  canvas.to_data_url().map_err(|e| js_error_message(&e))
}

#[component]
pub fn WefaxDecoderPage() -> impl IntoView {
  set_title("tools.wefax-decoder");

  let session = DecodeSession::new(DecodeConfig {
    codec: "wefax",
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

  let cancel = move |_| session.cancel();
  let reset = move |_| session.reset();
  let on_file = Callback::new(move |file: File| {
    session.start(file, move || Ok(JsValue::NULL));
  });

  // PNG 编码是主线程同步的大活（整幅 ImageData + `to_data_url`）。放进 `Memo` 只在结果
  // 变化时算一次：下面的渲染区里 `t()` / `tf()` 会订阅语言包版本，若把编码写在渲染闭包里，
  // 切一次语言就会重编码整幅图。
  let image = Memo::new(move |_| {
    // `with` 而不是 `get`：`WorkerPayload` 里带着整幅 `gray`，每次重算都 clone 一份
    // 等于白复制几 MB。
    result.with(|slot| {
      slot
        .as_ref()
        .map(|p| render_gray(&p.gray, p.width, p.height))
    })
  });

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("tools.wefax-decoder")
        subtitle=move || t("tools.wefax-decoder-subtitle")
        actions=ViewFn::from(move || {
          view! {
            <ButtonLink href="/modes" variant=Variant::Outline size=Size::Sm>
              {move || t("radio.digital-modes-reference")}
            </ButtonLink>
          }
        })
      />

      <PageContainer class="space-y-5">
        <section class="border bg-card p-4 text-sm text-muted-foreground">
          <p>
            {move || t("tools.upload-a-wefax-recording")}
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
                    title=Signal::derive(move || t("tools.choose-a-wefax-recording"))
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
            image
              .get()
              .map(|rendered| {
                let (width, height) = result
                  .with(|p| p.as_ref().map_or((0, 0), |p| (p.width, p.height)));
                match rendered {
                  // 渲染失败不能给空 src：浏览器会把空 src 当页面地址再请求一次，只显示破图。
                  // 此时 `result` 仍是 `Some`、上传区已被收起，所以必须给一个出口，
                  // 否则用户只能刷新页面。
                  Err(e) => {
                    view! {
                      <div class="space-y-4">
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
                        <div class="flex justify-center">
                          <Button variant=Variant::Outline on_click=move |_| reset(())>
                            <Icon kind=IconKind::RefreshCw class="size-6" />
                            {move || t("tools.decode-another-file")}
                          </Button>
                        </div>
                      </div>
                    }
                      .into_any()
                  }
                  Ok(url) => {
                    view! {
                      <div class="space-y-4">
                        <div class="bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
                          {crate::i18n::tf(
                            "tools.wefax-result-meta",
                            &[
                              &file_name.get_untracked(),
                              &width.to_string(),
                              &height.to_string(),
                            ],
                          )}
                        </div>
                        <img
                          src=url
                          alt=t("tools.wefax-image-alt")
                          class="w-full border bg-white/50 dark:bg-black/30"
                        />
                        <div class="flex justify-center">
                          <Button variant=Variant::Outline on_click=move |_| reset(())>
                            <Icon kind=IconKind::RefreshCw class="size-6" />
                            {move || t("tools.decode-another-file")}
                          </Button>
                        </div>
                      </div>
                    }
                      .into_any()
                  }
                }
              })
          }}
        </section>
      </PageContainer>
    </div>
  }
}
