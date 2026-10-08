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
use crate::ui::{Field, NumberField};

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
        t("tools.decoding-failed-make-sure-2")
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
  set_title("tools.wspr-decoder");

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
        "tools.file-too-large-about-2",
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
              error.set(Some(t("radio.could-not-create-the")));
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
            <h1 class="text-base font-semibold leading-tight">{move || t("tools.wspr-decoder")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("tools.weak-signal-propagation-reports")}</div>
          </div>
          <a
            href="/wspr"
            class="rounded-md border px-2.5 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
          >
            {move || t("tools.wspr-reference")}
          </a>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-5 px-4 py-5">
        <section class="rounded-xl border bg-card p-4 text-sm text-muted-foreground">
          <p>
            {move || t("tools.upload-a-wspr-recording")}
            {move || t("common.decoding-runs-in-a")}
            {move || t("common.the-audio-is-never")}
          </p>
        </section>

        <section class="rounded-xl border bg-card p-4">
          {{
            // `Field` 的 `r#for` 与控件 `id` 要配对；在块里现生成，避免被外层 `move ||` 闭包
            // 捕获（捕获会让闭包退化成 `FnOnce`）。
            let base_id = crate::util::unique_id("wspr-base");
            let label_for = base_id.clone();
            view! {
              <Field
                label=Signal::derive(move || t("tools.base-frequency-hz-default"))
                r#for=label_for
                class="mb-4 sm:max-w-xs"
              >
                <NumberField
                  id=base_id
                  value=Signal::derive(move || base_hz.get().to_string())
                  on_change=Callback::new(move |v: String| {
                    if let Ok(v) = v.trim().parse::<f64>() {
                      base_hz.set(v);
                    }
                  })
                  controls=false
                />
                <span class="text-xs text-muted-foreground">
                  {move || t("tools.wspr-signals-sit-in")}
                </span>
              </Field>
            }
          }}

          {move || {
            (result.with(Option::is_none) && !processing.get())
              .then(|| {
                view! {
                  <label
                    for="wspr-file"
                    class="flex flex-col items-center justify-center gap-1 rounded-xl border-2 border-dashed p-8 text-center transition-all duration-200 hover:bg-muted/40 cursor-pointer"
                  >
                    <Icon kind=IconKind::Waves class="mb-1 h-8 w-8 text-muted-foreground" />
                    <span class="text-sm font-medium">{move || t("tools.choose-a-wspr-recording")}</span>
                    <span class="text-xs text-muted-foreground">{move || t("tools.click-to-choose-or-2")}</span>
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
                          t("tools.reading-file")
                        } else {
                          t("radio.decoding-in-the-background")
                        }
                      }}
                    </span>
                    <button
                      type="button"
                      class="rounded-md border px-3 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                      on:click=cancel
                    >
                      {move || t("exam.cancel")}
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
                    <div class="font-medium">{move || t("radio.decoding-failed")}</div>
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
                      <span class="rounded-md bg-muted/60 px-2 py-1">{move || t("log.callsign")}{"："}{p.callsign}</span>
                      <span class="rounded-md bg-muted/60 px-2 py-1">{move || t("log.grid")}{"："}{p.locator}</span>
                      <span class="rounded-md bg-muted/60 px-2 py-1">{move || t("contest.power")}{"："}{p.power} dBm</span>
                    </div>
                  </div>
                  <div class="flex justify-center">
                    <button
                      type="button"
                      class="inline-flex items-center gap-2 rounded-md border px-4 py-2 text-sm font-medium transition-colors hover:bg-accent"
                      on:click=move |_| reset(())
                    >
                      <Icon kind=IconKind::RefreshCw class="h-4 w-4" />
                      {move || t("tools.decode-another-file")}
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
