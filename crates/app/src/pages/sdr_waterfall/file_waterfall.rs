use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
  AudioBuffer, AudioContext, CanvasRenderingContext2d, File, HtmlCanvasElement, HtmlInputElement,
  ImageData,
};

use crate::i18n::{t, tf};
use crate::icons::{Icon, IconKind};
use crate::util::{document, js_error_message};

use super::{CEIL_DB, FLOOR_DB};

/// 允许的最大 WAV 体积（字节）。
const MAX_BYTES: u64 = 32 * 1024 * 1024;
/// 离线分析每帧 FFT 长度（2 的幂，决定瀑布图宽度 = FRAME / 2）。
const FILE_FRAME: usize = 2048;
/// 离线瀑布图行数。
const FILE_HEIGHT: usize = 512;
/// 频谱图高度（像素）。
const FILE_SPECTRUM_H: usize = 160;

/// 离线分析结果。
#[derive(Clone)]
struct AnalysisResult {
  waterfall_url: String,
  spectrum_url: String,
  duration_seconds: f64,
  sample_rate: f32,
  width: u32,
  height: u32,
}

/// RGBA 像素 → 离屏 canvas → PNG data URL。
fn render_rgba(rgba: &[u8], width: u32, height: u32) -> Result<String, String> {
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
  let img = ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(rgba), width, height)
    .map_err(|e| js_error_message(&e))?;
  ctx
    .put_image_data(&img, 0.0, 0.0)
    .map_err(|e| js_error_message(&e))?;
  canvas.to_data_url().map_err(|e| js_error_message(&e))
}

async fn read_file_bytes(file: &File) -> Result<Vec<u8>, String> {
  let blob: web_sys::Blob = file.clone().into();
  let promise = blob.array_buffer();
  let buf = JsFuture::from(promise)
    .await
    .map_err(|e| js_error_message(&e))?;
  let arr: js_sys::ArrayBuffer = buf.unchecked_into();
  Ok(js_sys::Uint8Array::new(&arr).to_vec())
}

/// 用浏览器原生解码器把 WAV 字节解码为单声道 f32 样本。
async fn decode_audio(bytes: &[u8]) -> Result<(Vec<f32>, f32), String> {
  let ctx = AudioContext::new().map_err(|e| js_error_message(&e))?;
  let ab = js_sys::ArrayBuffer::new(bytes.len() as u32);
  js_sys::Uint8Array::new(&ab).copy_from(bytes);
  let promise = ctx
    .decode_audio_data(&ab)
    .map_err(|e| js_error_message(&e))?;
  let decoded = JsFuture::from(promise)
    .await
    .map_err(|e| js_error_message(&e))?;
  let buffer: AudioBuffer = decoded
    .dyn_into()
    .map_err(|_| t("radio.audio-decoding-failed-make"))?;
  let channel = buffer
    .get_channel_data(0)
    .map_err(|e| js_error_message(&e))?;
  let samples = channel.to_vec();
  let rate = buffer.sample_rate();
  let _ = ctx.close();
  Ok((samples, rate))
}

async fn analyze_file(file: &File) -> Result<AnalysisResult, String> {
  let bytes = read_file_bytes(file).await?;
  let (samples, sample_rate) = decode_audio(&bytes).await?;
  let (rgba, avg_db) =
    ham_web_core::spectrum::analyze_waterfall(&samples, FILE_FRAME, FILE_HEIGHT, FLOOR_DB, CEIL_DB)
      .ok_or_else(|| t("radio.audio-too-short-to"))?;
  let width = (FILE_FRAME / 2) as u32;
  let waterfall_url = render_rgba(&rgba, width, FILE_HEIGHT as u32)?;
  let spec_rgba =
    ham_web_core::spectrum::spectrum_to_rgba(&avg_db, FILE_SPECTRUM_H, FLOOR_DB, CEIL_DB);
  let spectrum_url = render_rgba(&spec_rgba, width, FILE_SPECTRUM_H as u32)?;
  Ok(AnalysisResult {
    waterfall_url,
    spectrum_url,
    duration_seconds: samples.len() as f64 / f64::from(sample_rate),
    sample_rate,
    width,
    height: FILE_HEIGHT as u32,
  })
}

/// 离线上传 WAV 做频谱 / 瀑布分析。
#[component]
pub(super) fn FileWaterfall() -> impl IntoView {
  let processing = RwSignal::new(false);
  let error = RwSignal::new(None::<String>);
  let result = RwSignal::new(None::<AnalysisResult>);
  let file_name = RwSignal::new(String::new());

  let on_file = Callback::new(move |file: File| {
    let size = file.size() as u64;
    if size > MAX_BYTES {
      let mb = size as f64 / 1024.0 / 1024.0;
      error.set(Some(tf(
        "radio.file-too-large-about",
        &[&format!("{mb:.0}"), &format!("{}", MAX_BYTES / 1024 / 1024)],
      )));
      return;
    }
    processing.set(true);
    error.set(None);
    result.set(None);
    file_name.set(file.name());
    spawn_local(async move {
      match analyze_file(&file).await {
        Ok(r) => {
          result.set(Some(r));
          processing.set(false);
        }
        Err(e) => {
          error.set(Some(e));
          processing.set(false);
        }
      }
    });
  });

  let reset = move |_| {
    result.set(None);
    error.set(None);
    file_name.set(String::new());
  };

  view! {
    <section class="rounded-xl border bg-card">
      <div class="border-b px-4 py-3">
        <h2 class="text-sm font-semibold">{move || t("radio.offline-analysis-upload-wav")}</h2>
        <p class="text-xs text-muted-foreground">
          {move || t("radio.upload-an-audio-recording")}
        </p>
      </div>
      <div class="space-y-4 p-4">
        {move || {
          (result.with(Option::is_none) && !processing.get())
            .then(|| view! {
              <label
                for="waterfall-file"
                class="flex flex-col items-center justify-center gap-1 rounded-xl border-2 border-dashed p-8 text-center transition-all duration-200 hover:bg-muted/40 cursor-pointer"
              >
                <Icon kind=IconKind::AudioLines class="mb-1 h-8 w-8 text-muted-foreground" />
                <span class="text-sm font-medium">{move || t("radio.choose-an-audio-file")}</span>
                <span class="text-xs text-muted-foreground">{move || t("radio.run-cargo-make-spectrum")}</span>
              </label>
              <input
                id="waterfall-file"
                type="file"
                accept=".wav,audio/wav,audio/x-wav,audio/*"
                class="hidden"
                on:change=move |e| {
                  let input: HtmlInputElement = event_target(&e);
                  if let Some(f) = input.files().and_then(|f| f.get(0)) {
                    on_file.run(f);
                  }
                  input.set_value("");
                }
              />
            })
        }}

        {move || {
          processing.get().then(|| view! {
            <div class="flex flex-col items-center justify-center gap-3 py-8">
              <Icon kind=IconKind::Loader2 class="h-8 w-8 animate-spin text-primary" />
              <span class="text-sm text-muted-foreground">{move || t("radio.analysing-spectrum")}</span>
            </div>
          })
        }}

        {move || {
          error.get().map(|e| view! {
            <div class="flex items-start gap-2 rounded-lg border border-red-200 bg-red-50 p-4 text-red-800 dark:border-red-900 dark:bg-red-950/40 dark:text-red-200">
              <Icon kind=IconKind::AlertCircle class="mt-0.5 h-5 w-5 shrink-0" />
              <div class="text-sm">{e}</div>
            </div>
          })
        }}

        {move || {
          result.get().map(|r| view! {
            <div class="space-y-4">
              <div class="flex flex-wrap gap-x-1.5 gap-y-1 rounded-lg bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
                <span class="max-w-[16rem] truncate font-medium text-foreground">{file_name.get_untracked()}</span>
                <span>"·"</span>
                <span>{format!("{:.0} Hz", r.sample_rate)}</span>
                <span>"·"</span>
                <span>{format!("{:.1} s", r.duration_seconds)}</span>
                <span>"·"</span>
                <span>{format!("{} × {}", r.width, r.height)}</span>
              </div>
              <div>
                <div class="mb-1 text-xs font-medium text-muted-foreground">{move || t("radio.waterfall-newest-at-the")}</div>
                <img src=r.waterfall_url.clone() alt=t("tools.spectrum-waterfall") class="w-full rounded-md border" />
              </div>
              <div>
                <div class="mb-1 text-xs font-medium text-muted-foreground">{move || t("radio.average-spectrum")}</div>
                <img src=r.spectrum_url.clone() alt=t("radio.average-spectrum") class="w-full rounded-md border" />
              </div>
              <div class="flex justify-center gap-2">
                <a
                  href=r.waterfall_url.clone()
                  download="waterfall.png"
                  class="inline-flex items-center gap-2 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
                >
                  <Icon kind=IconKind::Download class="h-4 w-4" />
                  {move || t("radio.download-waterfall-png")}
                </a>
                <button
                  type="button"
                  class="inline-flex items-center gap-2 rounded-md border px-4 py-2 text-sm font-medium transition-colors hover:bg-accent"
                  on:click=reset
                >
                  <Icon kind=IconKind::RefreshCw class="h-4 w-4" />
                  {move || t("radio.analyse-another-file")}
                </button>
              </div>
            </div>
          })
        }}
      </div>
    </section>
  }
}
