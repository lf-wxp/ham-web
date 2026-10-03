//! SSTV 慢扫描电视解码器页面：上传 SSTV 录音，交由 Web Worker 在后台线程解码。

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{File, HtmlCanvasElement, HtmlInputElement, ImageData, MessageEvent, Worker};

use crate::icons::{Icon, IconKind};
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
    .ok_or_else(|| t("Canvas 2D 上下文不可用"))?
    .dyn_into::<web_sys::CanvasRenderingContext2d>()
    .map_err(|_| t("Canvas 2D 上下文不可用"))?;

  let img = ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(rgba), width, height)
    .map_err(|e| js_error_message(&e))?;
  ctx
    .put_image_data(&img, 0.0, 0.0)
    .map_err(|e| js_error_message(&e))?;
  canvas.to_data_url().map_err(|e| js_error_message(&e))
}

/// 创建消息回调（只创建一次并复用）。
fn message_handler(
  result: RwSignal<Option<SstvResult>>,
  error: RwSignal<Option<String>>,
  processing: RwSignal<bool>,
) -> js_sys::Function {
  let cb = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
    let outcome = parse_message(&event.data()).and_then(|p| {
      render_rgba(&p.rgba, p.width, p.height).map(|url| SstvResult {
        width: p.width,
        height: p.height,
        mode: p.mode,
        source_sample_rate: p.source_sample_rate,
        duration_seconds: p.duration_seconds,
        image_url: url,
      })
    });
    match outcome {
      Ok(r) => result.set(Some(r)),
      Err(e) => error.set(Some(if e.is_empty() {
        t("解码失败，请确认是 SSTV 音频（WAV）")
      } else {
        e
      })),
    }
    processing.set(false);
  });
  let f = cb.as_ref().unchecked_ref::<js_sys::Function>().clone();
  cb.forget();
  f
}

fn spawn_worker(onmessage: &js_sys::Function) -> Option<Worker> {
  let w = Worker::new("/sstv-worker/worker.js").ok()?;
  w.set_onmessage(Some(onmessage));
  Some(w)
}

async fn read_file_bytes(file: &File) -> Result<Vec<u8>, String> {
  let blob: web_sys::Blob = file.clone().into();
  let promise = blob.array_buffer();
  let buf = wasm_bindgen_futures::JsFuture::from(promise)
    .await
    .map_err(|e| js_error_message(&e))?;
  let arr: js_sys::ArrayBuffer = buf.unchecked_into();
  Ok(js_sys::Uint8Array::new(&arr).to_vec())
}

fn to_array_buffer(bytes: &[u8]) -> js_sys::ArrayBuffer {
  let ab = js_sys::ArrayBuffer::new(bytes.len() as u32);
  let view = js_sys::Uint8Array::new(&ab);
  view.copy_from(bytes);
  ab
}

#[component]
pub fn SstvDecoderPage() -> impl IntoView {
  set_title(&t("SSTV 解码器"));

  let processing = RwSignal::new(false);
  let result = RwSignal::new(None::<SstvResult>);
  let error = RwSignal::new(None::<String>);
  let file_name = RwSignal::new(String::new());
  let reading = RwSignal::new(false);

  let generation = StoredValue::new(0u32);
  let onmessage = StoredValue::new(message_handler(result, error, processing));
  let worker = StoredValue::new(spawn_worker(&onmessage.get_value()));

  on_cleanup(move || {
    if let Some(w) = worker.get_value() {
      w.terminate();
    }
  });

  let cancel = move |_| {
    if let Some(w) = worker.get_value() {
      w.terminate();
    }
    let n = generation.get_value();
    generation.set_value(n + 1);
    worker.set_value(spawn_worker(&onmessage.get_value()));
    processing.set(false);
    reading.set(false);
    error.set(None);
    file_name.set(String::new());
  };

  let on_file = Callback::new(move |file: File| {
    let size = file.size() as u64;
    if size > MAX_BYTES {
      let mb = size as f64 / 1024.0 / 1024.0;
      error.set(Some(tf(
        "文件过大（约 {} MB），上限 {} MB。请先降采样到 8–16 kHz 单声道，或只截取图像那一段。",
        &[&format!("{mb:.0}"), &format!("{}", MAX_BYTES / 1024 / 1024)],
      )));
      return;
    }
    if let Some(w) = worker.get_value() {
      w.terminate();
    }
    let batch = generation.get_value() + 1;
    generation.set_value(batch);
    worker.set_value(spawn_worker(&onmessage.get_value()));
    processing.set(true);
    reading.set(true);
    error.set(None);
    result.set(None);
    file_name.set(file.name());
    spawn_local(async move {
      match read_file_bytes(&file).await {
        Ok(bytes) => {
          if generation.get_value() != batch {
            return;
          }
          reading.set(false);
          let ab = to_array_buffer(&bytes);
          match worker.get_value() {
            Some(w) => {
              if let Err(e) = w.post_message(&ab) {
                error.set(Some(js_error_message(&e)));
                processing.set(false);
              }
            }
            None => {
              error.set(Some(t("无法创建解码 Worker，请刷新页面重试")));
              processing.set(false);
            }
          }
        }
        Err(e) => {
          if generation.get_value() != batch {
            return;
          }
          error.set(Some(e));
          processing.set(false);
          reading.set(false);
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
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("SSTV 解码器")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("音频 FM 解调 + VIS 识别 · 后台线程离线处理")}</div>
          </div>
          <a
            href="/sstv"
            class="rounded-md border px-2.5 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
          >
            {move || t("SSTV 速查")}
          </a>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-5 px-4 py-5">
        <section class="rounded-xl border bg-card p-4 text-sm text-muted-foreground">
          <p>
            {move || t("上传一段 SSTV 慢扫描电视音频（WAV），即可在浏览器本地解调并重建图像，支持 Martin / Scottie / Robot 系列模式。")}
            {move || t("解码在 Web Worker 后台线程完成，不阻塞页面。")}
            {move || t("音频不会上传到服务器。")}
          </p>
        </section>

        <section class="rounded-xl border bg-card p-4">
          {move || {
            (result.with(Option::is_none) && !processing.get())
              .then(|| {
                view! {
                  <label
                    for="sstv-file"
                    class="flex flex-col items-center justify-center gap-1 rounded-xl border-2 border-dashed p-8 text-center transition-all duration-200 hover:bg-muted/40 cursor-pointer"
                  >
                    <Icon kind=IconKind::Camera class="mb-1 h-8 w-8 text-muted-foreground" />
                    <span class="text-sm font-medium">{move || t("选择 SSTV 录音（WAV）")}</span>
                    <span class="text-xs text-muted-foreground">{move || t("点击选择或拖拽音频文件到此处")}</span>
                    <span class="mt-1 text-xs text-muted-foreground">
                      {move || t("支持 Martin M1/M2、Scottie S1/S2/DX、Robot 36/72；推荐 8–16 kHz 单声道 WAV")}
                    </span>
                  </label>
                  <input
                    id="sstv-file"
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
                          t("正在读取文件…")
                        } else {
                          t("正在后台解码，请稍候…")
                        }
                      }}
                    </span>
                    <button
                      type="button"
                      class="rounded-md border px-3 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                      on:click=cancel
                    >
                      {move || t("取消")}
                    </button>
                  </div>
                }
              })
          }}

          {move || {
            error.get().map(|e| {
              view! {
                <div class="flex items-start gap-2 rounded-lg border border-red-200 bg-red-50 p-4 text-red-800 dark:border-red-900 dark:bg-red-950/40 dark:text-red-200">
                  <Icon kind=IconKind::AlertCircle class="mt-0.5 h-5 w-5 shrink-0" />
                  <div class="flex-1">
                    <div class="font-medium">{move || t("解码失败")}</div>
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
                  <div class="flex flex-wrap gap-x-1.5 gap-y-1 rounded-lg bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
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
                    alt=t("SSTV 解码结果")
                    class="mx-auto w-full max-w-full rounded-lg border"
                  />
                  <div class="flex justify-center gap-2">
                    <a
                      href=r.image_url.clone()
                      download="sstv.png"
                      class="inline-flex items-center gap-2 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
                    >
                      <Icon kind=IconKind::Download class="h-4 w-4" />
                      {move || t("下载 PNG")}
                    </a>
                    <button
                      type="button"
                      class="inline-flex items-center gap-2 rounded-md border px-4 py-2 text-sm font-medium transition-colors hover:bg-accent"
                      on:click=move |_| reset(())
                    >
                      <Icon kind=IconKind::RefreshCw class="h-4 w-4" />
                      {move || t("解码其他文件")}
                    </button>
                  </div>
                </div>
              }
            })
          }}
        </section>
      </div>
    </div>
  }
}
