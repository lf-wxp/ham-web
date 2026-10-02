//! NOAA APT 解码器页面：上传 APT 录音，交由 Web Worker 在后台线程解码。

use leptos::prelude::*;
use leptos::task::spawn_local;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{File, MessageEvent, Worker};

use crate::icons::{Icon, IconKind};
use crate::util::{js_error_message, set_title};

use super::apt_result_view::{AptRender, AptResultView};
use super::apt_uploader::AptUploader;
use super::apt_worker::parse_message;
use crate::i18n::t;

/// 读取文件字节（通过 Blob.array_buffer）。
async fn read_file_bytes(file: &File) -> Result<Vec<u8>, String> {
  let blob: web_sys::Blob = file.clone().into();
  let promise = blob.array_buffer();
  let buf = wasm_bindgen_futures::JsFuture::from(promise)
    .await
    .map_err(|e| js_error_message(&e))?;
  let arr: js_sys::ArrayBuffer = buf.unchecked_into();
  Ok(js_sys::Uint8Array::new(&arr).to_vec())
}

/// 字节 → 可转移的 `ArrayBuffer`（供 `postMessage` 发送）。
fn to_array_buffer(bytes: &[u8]) -> js_sys::ArrayBuffer {
  let ab = js_sys::ArrayBuffer::new(bytes.len() as u32);
  let view = js_sys::Uint8Array::new(&ab);
  view.copy_from(bytes);
  ab
}

#[component]
pub fn AptDecoderPage() -> impl IntoView {
  set_title(&t("NOAA APT 解码器"));

  let processing = RwSignal::new(false);
  let result = RwSignal::new(None::<AptRender>);
  let error = RwSignal::new(None::<String>);
  let file_name = RwSignal::new(String::new());

  // 惰性创建解码 Worker（页面生命周期内复用），并注册一次性消息回调。
  let worker = SendWrapper::new(Worker::new("/apt-worker/worker.js").ok());
  if let Some(w) = worker.as_ref() {
    let cb = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
      let outcome = parse_message(&event.data()).and_then(|p| {
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
      });
      match outcome {
        Ok(render) => result.set(Some(render)),
        Err(e) => error.set(Some(if e.is_empty() {
          t("解码失败，请确认是 APT 音频（WAV）")
        } else {
          e
        })),
      }
      processing.set(false);
    });
    w.set_onmessage(Some(cb.as_ref().unchecked_ref()));
    cb.forget();
  }

  on_cleanup({
    let worker = worker.clone();
    move || {
      if let Some(w) = worker.as_ref() {
        w.terminate();
      }
    }
  });

  let on_file = {
    let worker = worker.clone();
    Callback::new(move |file: File| {
      processing.set(true);
      error.set(None);
      result.set(None);
      file_name.set(file.name());
      let worker = worker.clone();
      spawn_local(async move {
        match read_file_bytes(&file).await {
          Ok(bytes) => {
            let ab = to_array_buffer(&bytes);
            match worker.as_ref() {
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
            error.set(Some(e));
            processing.set(false);
          }
        }
      });
    })
  };

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
            <h1 class="text-base font-semibold leading-tight">{move || t("NOAA APT 解码器")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("音频 AM 解调 + 图像重建 · 后台线程离线处理")}</div>
          </div>
          <a
            href="/weather-sat"
            class="rounded-md border px-2.5 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
          >
            {move || t("接收速查")}
          </a>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-5 px-4 py-5">
        <section class="rounded-xl border bg-card p-4 text-sm text-muted-foreground">
          <p>
            {move || t("上传一段从 NOAA 气象卫星接收的 APT 音频（137 MHz FM 解调后的 WAV），即可在浏览器本地解调 2400 Hz 副载波并重建可见光 / 红外云图。")}
            "解码在 Web Worker 后台线程完成，不阻塞页面。"
            "典型过境约 10–15 分钟，建议配合"
            <a href="/satellites" class="text-primary underline underline-offset-2">{move || t("过境预报")}</a>
            {move || t("提前录制。音频不会上传到服务器。")}
          </p>
        </section>

        <section class="rounded-xl border bg-card p-4">
          {move || {
            (result.with(Option::is_none) && !processing.get())
              .then(|| view! { <AptUploader on_file=on_file disabled=processing /> })
          }}

          {move || {
            processing
              .get()
              .then(|| {
                view! {
                  <div class="flex flex-col items-center justify-center gap-3 py-8">
                    <Icon kind=IconKind::Loader2 class="h-8 w-8 animate-spin text-primary" />
                    <span class="text-sm text-muted-foreground">{move || t("正在后台解码，请稍候…")}</span>
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
                <AptResultView render=r file_name=file_name.get_untracked() on_reset=Callback::new(reset) />
              }
            })
          }}
        </section>
      </div>
    </div>
  }
}
