use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use ham_web_core::mode_audio;
use ham_web_core::noise_floor;
use ham_web_core::spectrum::waterfall_color;
use js_sys::{Object, Reflect};
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
  AnalyserNode, AudioContext, CanvasRenderingContext2d, HtmlCanvasElement, ImageData, MediaStream,
  MediaStreamConstraints,
};

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::{t, tf};
use crate::share_score::download_bytes;
use crate::ui::{
  Button, Field, Input, Size, Slider, Table, TableBody, TableCell, TableHead, TableHeader,
  TableRow, ToastKind, Variant, push, push_error,
};
use crate::util::{js_error_message, set_title, storage, window};

use super::file_waterfall::FileWaterfall;
use super::{CEIL_DB, FLOOR_DB};

/// FFT 点数。
const FFT_SIZE: u32 = 2048;
/// 频率 bin 数（FFT 的一半），也是画布逻辑宽度。
const BINS: usize = 1024;
/// 频谱画布逻辑高度（px）。
const SPECTRUM_H: u32 = 160;
/// 瀑布画布逻辑高度（px）。
const WATERFALL_H: u32 = 256;
/// 刷新周期（ms，约 20 fps，兼顾流畅与每帧像素拷贝开销）。
const TICK_MS: i32 = 50;
/// 录音最长时长（秒）：到顶后停止累积，已录部分仍可导出。文案见 `tools.record-hint`。
const REC_MAX_SECS: f64 = 120.0;
/// 本底记录在 localStorage 里的键。
const NOISE_KEY: &str = "noise-floor-samples";
/// 最多保留多少条（超出丢最早的）。
const NOISE_MAX: usize = 100;

/// Unix 毫秒 → `YYYY-MM-DD HH:MM`（UTC，与记录口径一致）。
fn format_time(at_ms: f64) -> String {
  let text = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(at_ms))
    .to_iso_string()
    .as_string()
    .unwrap_or_default();
  text.chars().take(16).collect::<String>().replace('T', " ")
}

/// dB → 频谱画布纵坐标（顶部为高电平）。
fn db_to_y(db: f32, height: f32) -> f32 {
  let span = CEIL_DB - FLOOR_DB;
  let t = ((db - FLOOR_DB) / span).clamp(0.0, 1.0);
  (1.0 - t) * height
}

fn audio_constraints() -> MediaStreamConstraints {
  // 关闭回声消除 / 降噪 / 自动增益：它们会改变信号的频域特征。
  let audio = Object::new();
  for key in ["echoCancellation", "noiseSuppression", "autoGainControl"] {
    let _ = Reflect::set(&audio, &key.into(), &JsValue::FALSE);
  }
  let c = MediaStreamConstraints::new();
  c.set_audio(&audio);
  c
}

/// 运行中的音频链路。
struct Session {
  ctx: AudioContext,
  stream: MediaStream,
  interval: i32,
  _tick: Closure<dyn FnMut()>,
  /// 录音分支：script processor 必须接到 destination 才会被驱动，
  /// 中间串一个零增益汇点，否则麦克风会直接灌进扬声器（啸叫）。
  processor: web_sys::ScriptProcessorNode,
  sink: web_sys::GainNode,
  _on_audio: Closure<dyn FnMut(web_sys::AudioProcessingEvent)>,
}

/// 停掉麦克风的所有轨道（指示灯随之熄灭）。
fn stop_tracks(stream: &MediaStream) {
  for t in stream.get_tracks() {
    if let Ok(t) = t.dyn_into::<web_sys::MediaStreamTrack>() {
      t.stop();
    }
  }
}

impl Session {
  fn stop(self) {
    window().clear_interval_with_handle(self.interval);
    // 先摘掉录音回调再断开节点：`ctx.close()` 是异步的，队列里还排着的 `audioprocess`
    // 事件在 `_on_audio` 随 `self` 被 drop 之后会去调一个已释放的闭包；回调里还会读
    // 页面的信号，组件卸载后读已释放的信号会 panic。
    self.processor.set_onaudioprocess(None);
    let _ = self.processor.disconnect();
    let _ = self.sink.disconnect();
    stop_tracks(&self.stream);
    let _ = self.ctx.close();
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
  Idle,
  Starting,
  Listening,
}

#[component]
pub fn SdrWaterfallPage() -> impl IntoView {
  set_title("tools.sdr-waterfall");
  let supported = window().navigator().media_devices().is_ok();
  let status = RwSignal::new(Status::Idle);
  let error = RwSignal::new(None::<String>);
  let gain = RwSignal::new(0.0f32);
  let sample_rate = RwSignal::new(0.0f64);
  let session = StoredValue::new_local(None::<Session>);
  let spectrum_ref = NodeRef::<html::Canvas>::new();
  let waterfall_ref = NodeRef::<html::Canvas>::new();

  // 最近一帧频谱（dB）：只在「记录本底」时读，不必逐帧刷到 DOM。
  let latest = RwSignal::new(Vec::<f32>::new());
  let noise_label = RwSignal::new(String::new());
  let noise_samples =
    RwSignal::new(storage::get_json::<Vec<noise_floor::Sample>>(NOISE_KEY).unwrap_or_default());
  let last_floor = RwSignal::new(None::<noise_floor::Floor>);

  // 录音缓冲：只在「开始录音」之后累积，停止时整体转成 WAV。
  // 不能用 `Rc<RefCell>`：`Closure` 要求 `Send + Sync`（wasm 单线程也一样）。
  let recorder = RwSignal::new(Vec::<f32>::new());
  let recording = RwSignal::new(false);

  // `spawn_local` 的续体与定时器回调不会随组件卸载取消：`await` / 延时之后再读组件里的
  // 信号或 `NodeRef`，而页面已经离开（信号随之释放），读会 panic 并带走整个 wasm 实例。
  // 守卫必须在组件体里创建（见 `util::mount_guard`），这里存进 `StoredValue` 以便在
  // 多个 `Copy` 闭包里取用。
  let alive = StoredValue::new(crate::util::mount_guard());

  // 结束录音并把缓冲转成 WAV 下载；没在录音时什么都不做。
  // 停止监听也要走这里：否则「录音中 → 停止 → 再开始」会把两次会话的样本拼进同一个
  // WAV，而录音按钮在非监听态是 disabled，用户没有机会自己收尾。
  let finish_recording = move || {
    if !recording.get_untracked() {
      return;
    }
    recording.set(false);
    // `take` 而不是 `get` 再 `set(Vec::new())`：缓冲最大约 23 MB，不必先整份克隆。
    let samples = recorder.try_update(std::mem::take).unwrap_or_default();
    let rate = sample_rate.get_untracked();
    if samples.is_empty() || rate <= 0.0 {
      push_error(t("tools.record-empty"));
      return;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let rate = rate as u32;
    download_bytes(
      &mode_audio::to_wav(&samples, rate),
      "audio/wav",
      "recording.wav",
    );
  };

  let stop = move || {
    finish_recording();
    if let Some(s) = session.try_update_value(Option::take).flatten() {
      s.stop();
    }
    status.set(Status::Idle);
  };

  // 落一条记录；拿不到可用频谱时返回 `false`（由调用方决定重试还是提示）。
  let save_floor = move || -> bool {
    let spectrum = latest.get_untracked();
    let Some(floor) = noise_floor::estimate(&spectrum) else {
      return false;
    };
    last_floor.set(Some(floor));
    let typed = noise_label.get_untracked().trim().to_owned();
    let label = if typed.is_empty() {
      t("tools.noise-floor-unnamed").to_string()
    } else {
      typed
    };
    noise_samples.update(|list| {
      list.push(noise_floor::Sample {
        label,
        at_ms: js_sys::Date::now(),
        floor_db: floor.floor_db,
        occupied_ratio: floor.occupied_ratio,
      });
      if list.len() > NOISE_MAX {
        list.remove(0);
      }
    });
    storage::set_json(NOISE_KEY, &noise_samples.get_untracked());
    true
  };

  let record = Callback::new(move |_| {
    if save_floor() {
      return;
    }
    // 刚开始监听的一小段里，音频上下文可能还没真正跑起来（输出全静音）：
    // 300 ms 后再试一次，而不是让按钮看着像坏了。
    // 回调里读 `latest` 等信号：这 300 ms 内用户离开了页面的话，信号已经释放，要先问守卫。
    let alive = alive.get_value();
    set_timeout(
      move || {
        if !alive() {
          return;
        }
        if !save_floor() {
          push_error(t("tools.noise-floor-no-spectrum"));
        }
      },
      Duration::from_millis(300),
    );
  });

  let recorder_for_audio = recorder;

  // 录音开关：开始只清缓冲，停止时把缓冲整体转成 WAV。
  let toggle_record = Callback::new(move |_| {
    if recording.get_untracked() {
      finish_recording();
    } else {
      recorder.set(Vec::new());
      recording.set(true);
    }
  });

  let start = move || {
    error.set(None);
    status.set(Status::Starting);
    // 守卫要在事件回调里取（此时组件必然还挂着），再带进异步续体。
    let alive = alive.get_value();
    spawn_local(async move {
      let result: Result<Session, JsValue> = async {
        let devices = window().navigator().media_devices()?;
        let stream: MediaStream =
          JsFuture::from(devices.get_user_media_with_constraints(&audio_constraints())?)
            .await?
            .dyn_into()?;
        // 授权弹窗开着的时候用户可能已经离开了页面：这时信号和画布引用都已释放，
        // 再读会 panic，麦克风也得立刻放掉。
        if !alive() {
          stop_tracks(&stream);
          return Err(JsValue::UNDEFINED);
        }
        // 先备好画布，再去建音频链路：画布没就绪是最常见的失败，放在前面就不会出现
        // 「麦克风已经打开又得回头关掉」。
        let canvases =
          (|| -> Result<(CanvasRenderingContext2d, CanvasRenderingContext2d), JsValue> {
            let spectrum_canvas: HtmlCanvasElement = spectrum_ref
              .get()
              .ok_or_else(|| JsValue::from_str(&t("common.spectrum-canvas-not-ready")))?;
            let waterfall_canvas: HtmlCanvasElement = waterfall_ref
              .get()
              .ok_or_else(|| JsValue::from_str(&t("common.waterfall-canvas-not-ready")))?;
            spectrum_canvas.set_width(FFT_SIZE / 2);
            spectrum_canvas.set_height(SPECTRUM_H);
            waterfall_canvas.set_width(FFT_SIZE / 2);
            waterfall_canvas.set_height(WATERFALL_H);
            let sctx: CanvasRenderingContext2d = spectrum_canvas
              .get_context("2d")?
              .ok_or_else(|| JsValue::from_str(&t("common.2d-context-not-available")))?
              .unchecked_into();
            let wctx: CanvasRenderingContext2d = waterfall_canvas
              .get_context("2d")?
              .ok_or_else(|| JsValue::from_str(&t("common.2d-context-not-available")))?
              .unchecked_into();
            Ok((sctx, wctx))
          })();
        let (sctx, wctx) = match canvases {
          Ok(pair) => pair,
          Err(e) => {
            stop_tracks(&stream);
            return Err(e);
          }
        };
        let ctx = match AudioContext::new() {
          Ok(ctx) => ctx,
          Err(e) => {
            stop_tracks(&stream);
            return Err(e);
          }
        };
        // 自动播放策略可能让 `resume` 被拒绝：这里只是「顺手唤醒」，失败了下面的
        // analyser 仍能读到（用户点按钮本身就算一次手势），不必打断启动流程。
        let _ = ctx.resume();
        // 实际采样率由浏览器决定（44.1k / 48k 都可能），下面的录音上限与时间换算都用它。
        let rate = f64::from(ctx.sample_rate());
        let (build_ctx, build_stream) = (ctx.clone(), stream.clone());
        // 音频图的每一步都可能失败；失败时由下面统一放掉流和上下文，这里只管往下建。
        let build = move || -> Result<Session, JsValue> {
          let (ctx, stream) = (build_ctx, build_stream);
          let source = ctx.create_media_stream_source(&stream)?;
          let analyser: AnalyserNode = ctx.create_analyser()?;
          analyser.set_fft_size(FFT_SIZE);
          analyser.set_smoothing_time_constant(0.0);
          // getFloatFrequencyData 返回的已经是 dB 值；min/maxDecibels 只影响
          // getByteFrequencyData 的字节缩放，对本路径不生效，故下面不能再取一次对数。
          source.connect_with_audio_node(&analyser)?;

          // 录音分支：`ScriptProcessorNode` 只有接到 destination 才会被驱动，
          // 但直连会把麦克风灌进扬声器，所以串一个零增益汇点。
          let processor = ctx.create_script_processor_with_buffer_size(4096)?;
          let sink = ctx.create_gain()?;
          sink.gain().set_value(0.0);
          source.connect_with_audio_node(&processor)?;
          processor.connect_with_audio_node(&sink)?;
          sink.connect_with_audio_node(&ctx.destination())?;
          let recorder_signal = recorder_for_audio;
          // 录音上限：缓冲是 `Vec<f32>`，48 kHz 下每分钟约 11.5 MB，导出时还要再翻一倍。
          // 到顶就自动结束并导出已录的部分，而不是把标签页撑爆；同时要告诉用户，
          // 否则界面还停在「录音中」，后面的声音其实没录进去。
          #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
          let max_samples = (rate * REC_MAX_SECS) as usize;
          let on_audio = Closure::<dyn FnMut(web_sys::AudioProcessingEvent)>::new(
            move |event: web_sys::AudioProcessingEvent| {
              if !recording.get_untracked() {
                return;
              }
              let Ok(buffer) = event.input_buffer() else {
                return;
              };
              let Ok(channel) = buffer.get_channel_data(0) else {
                return;
              };
              let full = recorder_signal.try_update(|buffer| {
                let room = max_samples.saturating_sub(buffer.len());
                buffer.extend_from_slice(&channel[..channel.len().min(room)]);
                buffer.len() >= max_samples
              });
              if full == Some(true) {
                finish_recording();
                push(
                  ToastKind::Info,
                  tf(
                    "tools.record-limit-reached",
                    &[&format!("{REC_MAX_SECS:.0}")],
                  ),
                );
              }
            },
          );

          sample_rate.set(f64::from(ctx.sample_rate()));

          let buf = Rc::new(RefCell::new(vec![0u8; BINS * WATERFALL_H as usize * 4]));
          let mut freq = vec![0f32; BINS];

          let tick = Closure::<dyn FnMut()>::new(move || {
            analyser.get_float_frequency_data(&mut freq);
            // 每帧都留一份给「记录本底」：只在点击时读一次，成本只是一次 4 KB 拷贝。
            latest.set(freq.clone());
            let g = gain.get_untracked();

            // 频谱折线。
            sctx.set_fill_style_str("#000");
            sctx.fill_rect(0.0, 0.0, f64::from(FFT_SIZE / 2), f64::from(SPECTRUM_H));
            sctx.begin_path();
            for (i, &v) in freq.iter().enumerate() {
              #[allow(clippy::cast_precision_loss)]
              let x = i as f64;
              // v 已是 dB（见上面的 min/maxDecibels 设置），不能再取一次对数。
              let y = f64::from(db_to_y(v + g, SPECTRUM_H as f32));
              if i == 0 {
                sctx.move_to(x, y);
              } else {
                sctx.line_to(x, y);
              }
            }
            sctx.set_stroke_style_str("#22d3ee");
            sctx.set_line_width(1.0);
            sctx.stroke();

            // 瀑布图：新行写入顶部，旧行下移一行。
            {
              let mut b = buf.borrow_mut();
              let row = BINS * 4;
              let total = BINS * WATERFALL_H as usize * 4;
              b.copy_within(0..(total - row), row);
              for (i, &v) in freq.iter().enumerate() {
                let (r, gr, bl) = waterfall_color(v + g, FLOOR_DB, CEIL_DB);
                let o = i * 4;
                b[o] = r;
                b[o + 1] = gr;
                b[o + 2] = bl;
                b[o + 3] = 255;
              }
              if let Ok(img) = ImageData::new_with_u8_clamped_array_and_sh(
                wasm_bindgen::Clamped(b.as_slice()),
                FFT_SIZE / 2,
                WATERFALL_H,
              ) {
                // 每帧一次绘制：失败只丢这一帧的条纹，下一帧会覆盖（逐帧弹错会把页面刷爆）。
                let _ = wctx.put_image_data(&img, 0.0, 0.0);
              }
            }
          });
          let interval = window().set_interval_with_callback_and_timeout_and_arguments_0(
            tick.as_ref().unchecked_ref(),
            TICK_MS,
          )?;
          // 录音回调挂在最后一步：此后不再有会失败的操作，失败路径上就不会留下一个
          // 指向已释放闭包的回调。
          processor.set_onaudioprocess(Some(on_audio.as_ref().unchecked_ref()));
          Ok(Session {
            ctx,
            stream,
            interval,
            _tick: tick,
            processor,
            sink,
            _on_audio: on_audio,
          })
        };
        let built = build();
        if built.is_err() {
          stop_tracks(&stream);
          let _ = ctx.close();
        }
        built
      }
      .await;
      match result {
        Ok(s) if !alive() || status.try_get_untracked() != Some(Status::Starting) => s.stop(),
        Ok(s) => {
          session.set_value(Some(s));
          status.set(Status::Listening);
        }
        // 授权期间离开了页面：没有可通知的对象，也不能再写信号。
        Err(_) if !alive() => {}
        Err(e) => {
          let denied = Reflect::get(&e, &"name".into())
            .ok()
            .and_then(|n| n.as_string())
            .is_some_and(|n| n == "NotAllowedError");
          error.set(Some(if denied {
            t("morse.no-microphone-permission-allow")
          } else {
            js_error_message(&e)
          }));
          status.set(Status::Idle);
        }
      }
    });
  };

  on_cleanup(move || {
    if let Some(s) = session.try_update_value(Option::take).flatten() {
      s.stop();
    }
  });

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      // 页头走 `PageHeader`（全站唯一来源）：标题 / 副标题随语言刷新，开始-停止按钮放 `actions`。
      // 手写 `<h1>` 会让吸顶偏移与排版和别的页面慢慢漂开，而且点阵字体没有 `font-semibold`。
      <PageHeader
        title=move || t("tools.sdr-waterfall")
        subtitle=move || t("tools.feed-the-radio-or")
        actions=ViewFn::from(move || {
          view! {
            <Button
              variant=Variant::Default
              size=Size::Sm
              disabled=Signal::derive(move || !supported || status.get() == Status::Starting)
              on_click=Callback::new(move |_| if status.get_untracked() == Status::Idle { start() } else { stop() })
            >
              {move || match status.get() {
                Status::Idle => t("tools.start"),
                Status::Starting => t("morse.requesting-microphone"),
                Status::Listening => t("common.stop"),
              }}
            </Button>
          }
        })
      />

      <PageContainer class="space-y-5">
        <section class="pxl-card border bg-card">
          <div class="space-y-4 p-4">
            {(!supported).then(|| view! {
              <p role="alert" class="text-sm text-amber-800 dark:text-amber-300">
                {move || t("morse.this-browser-cannot-access")}
              </p>
            })}
            {move || error.get().map(|e| view! { <p role="alert" class="text-sm text-destructive">{e}</p> })}

            <div class="flex flex-wrap items-center gap-x-4 gap-y-2 text-xs text-muted-foreground">
              <label class="inline-flex items-center gap-2">
                {move || t("tools.gain")}
                <Slider
                  value=Signal::derive(move || f64::from(gain.get()))
                  on_change=Callback::new(move |v: f64| gain.set(v as f32))
                  min=-40.0
                  max=40.0
                  step=1.0
                  class="w-32"
                  aria_label=Signal::derive(move || t("tools.display-gain-db"))
                  aria_valuetext=Signal::derive(move || format!("{:+} dB", gain.get().round()))
                />
                <span class="w-12 tabular-nums text-foreground">
                  {move || tf("common.db", &[&format!("{:+}", gain.get())])}
                </span>
              </label>
              <span class="tabular-nums">
                {move || {
                  let nyquist = sample_rate.get() / 2.0;
                  if nyquist > 0.0 {
                    tf("common.hz-range", &[&format!("{nyquist:.0}")])
                  } else {
                    t("tools.frequency-range").to_string()
                  }
                }}
              </span>
            </div>

            <div class="flex flex-wrap items-center gap-3">
              <Button
                variant=Variant::Outline
                size=Size::Sm
                disabled=Signal::derive(move || status.get() != Status::Listening)
                on_click=toggle_record
              >
                {move || {
                  if recording.get() {
                    t("tools.record-stop")
                  } else {
                    t("tools.record-start")
                  }
                }}
              </Button>
              <span class="text-xs text-muted-foreground">{move || t("tools.record-hint")}</span>
            </div>

            <div>
              <div class="mb-1 text-xs font-medium text-muted-foreground">{move || t("tools.live-spectrum")}</div>
              // 裸 `<canvas>` 没有隐式 role，`aria-label` 不会被读屏暴露，必须显式 `role="img"`。
              <canvas
                node_ref=spectrum_ref
                width="1024"
                height="160"
                class="h-40 w-full bg-black"
                role="img"
                aria-label=move || t("tools.live-spectrum")
              ></canvas>
            </div>

            <div>
              <div class="mb-1 text-xs font-medium text-muted-foreground">{move || t("tools.waterfall-scrolling-down-over")}</div>
              <canvas
                node_ref=waterfall_ref
                width="1024"
                height="256"
                class="h-64 w-full bg-black"
                role="img"
                aria-label=move || t("tools.spectrum-waterfall")
              ></canvas>
            </div>

            <p class="text-xs text-muted-foreground">
              {move || t("tools.darker-to-brighter-colours")}
            </p>
          </div>
        </section>
        <section class="pxl-card border bg-card" aria-labelledby="noise-floor-heading">
          <h2 id="noise-floor-heading" class="border-b px-4 py-3 text-sm font-semibold">
            {move || t("tools.noise-floor")}
          </h2>
          <div class="space-y-3 p-4">
            <p class="text-xs text-muted-foreground">{move || t("tools.noise-floor-hint")}</p>

            <div class="flex flex-wrap items-end gap-3">
              <Field label=Signal::derive(|| t("tools.noise-floor-label")) r#for="sdr-noise-label">
                <Input
                  id="sdr-noise-label"
                  value=noise_label
                  on_change=Callback::new(move |v: String| noise_label.set(v))
                />
              </Field>
              <Button variant=Variant::Default size=Size::Sm on_click=record>
                {move || t("tools.noise-floor-record")}
              </Button>
            </div>

            {move || {
              last_floor
                .get()
                .map(|f| {
                  view! {
                    <p class="bg-muted/40 px-3 py-2 text-sm tabular-nums">
                      {tf("tools.noise-floor-floor", &[&format!("{:.1}", f.floor_db)])}
                      " · "
                      {tf(
                        "tools.noise-floor-occupied",
                        &[&format!("{:.0}", f64::from(f.occupied_ratio) * 100.0)],
                      )}
                    </p>
                  }
                })
            }}
            {move || {
              noise_floor::last_delta(&noise_samples.get())
                .map(|d| {
                  view! {
                    <p class="text-xs text-muted-foreground">
                      {tf("tools.noise-floor-delta", &[&format!("{d:+.1}")])}
                    </p>
                  }
                })
            }}

            {move || {
              let list = noise_samples.get();
              if list.is_empty() {
                return view! {
                  <p class="bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
                    {move || t("tools.noise-floor-empty")}
                  </p>
                }
                  .into_any();
              }
              let body = list
                .iter()
                .rev()
                .map(|s| {
                  let time = format_time(s.at_ms);
                  let label = s.label.clone();
                  let floor = format!("{:.1} dB", s.floor_db);
                  let occupied = format!("{:.0}%", f64::from(s.occupied_ratio) * 100.0);
                  view! {
                    <TableRow>
                      <TableCell class="border px-3 py-2 align-top whitespace-nowrap tabular-nums">
                        {time}
                      </TableCell>
                      <TableCell class="border px-3 py-2 align-top">{label}</TableCell>
                      <TableCell class="border px-3 py-2 align-top whitespace-nowrap tabular-nums">
                        {floor}
                      </TableCell>
                      <TableCell class="border px-3 py-2 align-top whitespace-nowrap tabular-nums text-muted-foreground">
                        {occupied}
                      </TableCell>
                    </TableRow>
                  }
                })
                .collect_view();
              view! {
                <Table
                  class="min-w-[520px] text-sm"
                  aria_label=Signal::derive(|| t("tools.noise-floor"))
                  tabindex="0"
                  role="region"
                >
                  <TableHead>
                    <TableRow>
                      <TableHeader class="border px-3 py-2">{move || t("tools.noise-floor-time")}</TableHeader>
                      <TableHeader class="border px-3 py-2">{move || t("tools.noise-floor-label")}</TableHeader>
                      <TableHeader class="border px-3 py-2">{move || t("tools.noise-floor-floor-head")}</TableHeader>
                      <TableHeader class="border px-3 py-2">{move || t("tools.noise-floor-occupied-head")}</TableHeader>
                    </TableRow>
                  </TableHead>
                  <TableBody>{body}</TableBody>
                </Table>
              }
                .into_any()
            }}
          </div>
        </section>
        <FileWaterfall />
      </PageContainer>
    </div>
  }
}
