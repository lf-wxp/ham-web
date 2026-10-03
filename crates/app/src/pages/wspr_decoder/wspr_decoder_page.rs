//! WSPR 弱信号传播报告解码器页面：上传 WSPR 录音，交由 Web Worker 后台解码。

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use web_sys::{File, HtmlInputElement, MessageEvent, Worker};

use crate::icons::{Icon, IconKind};
use crate::util::{js_error_message, set_title};

use super::wspr_worker::{WorkerPayload, parse_message};
use crate::i18n::{t, tf};

/// 允许的最大 WAV 体积（字节）。110.6 秒的 48 kHz 单声道约 10.6 MB，32 MB 足够。
const MAX_BYTES: u64 = 32 * 1024 * 1024;

fn message_handler(
  result: RwSignal<Option<WorkerPayload>>,
  error: RwSignal<Option<String>>,
  processing: RwSignal<bool>,
) -> js_sys::Function {
  let cb = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
    match parse_message(&event.data()) {
      Ok(p) => result.set(Some(p)),
      Err(e) => error.set(Some(if e.is_empty() {
        t("解码失败，请确认是 WSPR 音频（WAV）且基准频率正确")
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
  let w = Worker::new("/wspr-worker/worker.js").ok()?;
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

/// 组装 `{ bytes: ArrayBuffer, baseHz: number }` 消息。
fn to_message(bytes: &[u8], base_hz: f64) -> js_sys::Object {
  let ab = js_sys::ArrayBuffer::new(bytes.len() as u32);
  let view = js_sys::Uint8Array::new(&ab);
  view.copy_from(bytes);
  let obj = js_sys::Object::new();
  let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("bytes"), &JsValue::from(ab));
  let _ = js_sys::Reflect::set(
    &obj,
    &JsValue::from_str("baseHz"),
    &JsValue::from_f64(base_hz),
  );
  obj
}

#[component]
pub fn WsprDecoderPage() -> impl IntoView {
  set_title(&t("WSPR 解码器"));

  let processing = RwSignal::new(false);
  let result = RwSignal::new(None::<WorkerPayload>);
  let error = RwSignal::new(None::<String>);
  let file_name = RwSignal::new(String::new());
  let reading = RwSignal::new(false);
  let base_hz = RwSignal::new(1500.0);

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
        "文件过大（约 {} MB），上限 {} MB。请先降采样到 8–16 kHz 单声道。",
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
          let msg = to_message(&bytes, base_hz.get());
          match worker.get_value() {
            Some(w) => {
              if let Err(e) = w.post_message(&msg) {
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
            <h1 class="text-base font-semibold leading-tight">{move || t("WSPR 解码器")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("弱信号传播报告 · 4-FSK 解调 · 后台线程离线处理")}</div>
          </div>
          <a
            href="/wspr"
            class="rounded-md border px-2.5 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
          >
            {move || t("WSPR 速查")}
          </a>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-5 px-4 py-5">
        <section class="rounded-xl border bg-card p-4 text-sm text-muted-foreground">
          <p>
            {move || t("上传一段 WSPR 弱信号传播报告录音（WAV），在浏览器本地解调并解码出呼号 / 网格 / 功率。")}
            {move || t("解码在 Web Worker 后台线程完成，不阻塞页面。")}
            {move || t("音频不会上传到服务器。")}
          </p>
        </section>

        <section class="rounded-xl border bg-card p-4">
          <label class="mb-4 flex flex-col gap-1.5 text-sm sm:max-w-xs">
            <span class="text-xs text-muted-foreground">{move || t("基准频率（Hz，默认 1500）")}</span>
            <input
              type="number"
              prop:value=move || base_hz.get().to_string()
              on:input=move |e| {
                if let Ok(v) = event_target_value(&e).parse::<f64>() {
                  base_hz.set(v);
                }
              }
              class="h-10 rounded-lg border bg-background px-3 text-sm tabular-nums outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            />
            <span class="text-xs text-muted-foreground">
              {move || t("WSPR 信号位于 1400–1600 Hz 音频窗口，通常取电台拨号频率对应的音频 1500 Hz。")}
            </span>
          </label>

          {move || {
            (result.with(Option::is_none) && !processing.get())
              .then(|| {
                view! {
                  <label
                    for="wspr-file"
                    class="flex flex-col items-center justify-center gap-1 rounded-xl border-2 border-dashed p-8 text-center transition-all duration-200 hover:bg-muted/40 cursor-pointer"
                  >
                    <Icon kind=IconKind::Waves class="mb-1 h-8 w-8 text-muted-foreground" />
                    <span class="text-sm font-medium">{move || t("选择 WSPR 录音（WAV）")}</span>
                    <span class="text-xs text-muted-foreground">{move || t("点击选择或拖拽音频文件到此处")}</span>
                  </label>
                  <input
                    id="wspr-file"
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
            result.get().map(|p| {
              view! {
                <div class="space-y-4">
                  <div class="flex flex-wrap gap-x-1.5 gap-y-1 rounded-lg bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
                    <span class="max-w-[16rem] truncate font-medium text-foreground">{file_name.get_untracked()}</span>
                    <span>"·"</span>
                    <span>{format!("{} Hz", base_hz.get())}</span>
                  </div>
                  <div class="rounded-lg border bg-card p-6 text-center">
                    <div class="text-3xl font-semibold tracking-tight">{p.text}</div>
                    <div class="mt-3 flex flex-wrap justify-center gap-2 text-sm">
                      <span class="rounded-md bg-muted/60 px-2 py-1">{move || t("呼号")}{"："}{p.callsign}</span>
                      <span class="rounded-md bg-muted/60 px-2 py-1">{move || t("网格")}{"："}{p.locator}</span>
                      <span class="rounded-md bg-muted/60 px-2 py-1">{move || t("功率")}{"："}{p.power} dBm</span>
                    </div>
                  </div>
                  <div class="flex justify-center">
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
