//! NOAA APT 解码器页面：上传 APT 录音，交由 Web Worker 在后台线程解码。

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{File, MessageEvent, Worker};

use crate::icons::{Icon, IconKind};
use crate::util::{js_error_message, set_title};

use super::apt_result_view::{AptRender, AptResultView};
use super::apt_uploader::AptUploader;
use super::apt_worker::parse_message;
use crate::i18n::{t, tf};

/// 允许的最大 WAV 体积（字节）。
///
/// 解码会在 Worker 里把整段音频读入内存并重采样到 20800 Hz（每采样一个 `f32`），
/// 峰值内存可达文件体积的数倍。不设上限时，误选一个几百 MB 的录音会直接把标签页
/// 拖垮；96 MB 足够容纳 44.1 kHz 单声道约 15 分钟 —— 正好是一次典型过境的长度。
const MAX_BYTES: u64 = 96 * 1024 * 1024;

/// 创建消息回调。
///
/// **只创建一次并在所有 Worker 之间复用**：每次取消都新建并 `forget` 一个 `Closure`
/// 会随操作次数累积泄漏。过期结果靠「换文件 / 取消时 `terminate` 旧 Worker」来丢弃
/// —— Worker 内的解码是同步的，无法中途打断，只能整体丢弃。
fn message_handler(
  result: RwSignal<Option<AptRender>>,
  error: RwSignal<Option<String>>,
  processing: RwSignal<bool>,
) -> js_sys::Function {
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
  let f = cb.as_ref().unchecked_ref::<js_sys::Function>().clone();
  // 回调与页面同生命周期常驻，故意不回收（只此一份）。
  cb.forget();
  f
}

/// 创建一个解码 Worker 并挂好消息回调。
///
/// 单独成函数是因为「取消」需要 `terminate` 掉正在解码的 Worker 再重建一个 ——
/// Worker 内的解码是同步的，无法中途被打断，只能整体丢弃。
fn spawn_worker(onmessage: &js_sys::Function) -> Option<Worker> {
  let w = Worker::new("/apt-worker/worker.js").ok()?;
  w.set_onmessage(Some(onmessage));
  Some(w)
}

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
  // 当前阶段：读取文件与后台解码分开显示，让用户知道卡在哪一步。
  let reading = RwSignal::new(false);

  // 解码批次号。每次开始新任务或取消都会 +1，读取阶段仍在 await 的任务据此判断自己
  // 是否已经作废；过期 Worker 则靠「换文件 / 取消时 terminate」整体丢弃。
  let generation = StoredValue::new(0u32);
  // 回调本身放进 `StoredValue`：`js_sys::Function` 不是 `Copy`，直接捕获会让
  // `cancel` / `on_file` 变成只能调用一次的 `FnOnce`。
  let onmessage = StoredValue::new(message_handler(result, error, processing));

  // Worker 放在 `StoredValue` 里：取消时要 terminate 掉旧的再换一个新的。
  let worker = StoredValue::new(spawn_worker(&onmessage.get_value()));

  on_cleanup(move || {
    if let Some(w) = worker.get_value() {
      w.terminate();
    }
  });

  // 取消当前解码：丢弃正在解码的 Worker，重建一个备用。
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
        "文件过大（约 {} MB），上限 {} MB。解码需要把整段音频载入内存，建议先降采样到 8–16 kHz 单声道，或只截取过境那一段。",
        &[&format!("{mb:.0}"), &format!("{}", MAX_BYTES / 1024 / 1024)],
      )));
      return;
    }
    // 每一批解码都换一个新 Worker：上一次的解码在 Worker 里是同步的、无法打断，
    // 只能整体丢弃，否则它会与这一批抢同一组信号。
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
          // 读取期间用户可能已经取消或换了文件：这一批作废，别再占用新的 Worker。
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
            {move || t("解码在 Web Worker 后台线程完成，不阻塞页面。")}
            {move || t("典型过境约 10–15 分钟，建议配合")}
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
                    <span class="text-sm text-muted-foreground">
                      {move || {
                        if reading.get() {
                          t("正在读取文件…")
                        } else {
                          t("正在后台解码，请稍候…")
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
                <AptResultView render=r file_name=file_name.get_untracked() on_reset=Callback::new(reset) />
              }
            })
          }}
        </section>
      </div>
    </div>
  }
}
