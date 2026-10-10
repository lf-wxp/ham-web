use std::cell::RefCell;
use std::rc::Rc;

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

use crate::components::common::PageContainer;
use crate::i18n::{t, tf};
use crate::ui::{Button, Size, Slider, Variant};
use crate::util::{js_error_message, set_title, window};

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
}

impl Session {
  fn stop(self) {
    window().clear_interval_with_handle(self.interval);
    for t in self.stream.get_tracks() {
      if let Ok(t) = t.dyn_into::<web_sys::MediaStreamTrack>() {
        t.stop();
      }
    }
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

  let stop = move || {
    if let Some(s) = session.try_update_value(Option::take).flatten() {
      s.stop();
    }
    status.set(Status::Idle);
  };

  let start = move || {
    error.set(None);
    status.set(Status::Starting);
    spawn_local(async move {
      let result: Result<Session, JsValue> = async {
        let devices = window().navigator().media_devices()?;
        let stream: MediaStream =
          JsFuture::from(devices.get_user_media_with_constraints(&audio_constraints())?)
            .await?
            .dyn_into()?;
        let ctx = AudioContext::new()?;
        let _ = ctx.resume();
        let source = ctx.create_media_stream_source(&stream)?;
        let analyser: AnalyserNode = ctx.create_analyser()?;
        analyser.set_fft_size(FFT_SIZE);
        analyser.set_smoothing_time_constant(0.0);
        // getFloatFrequencyData 返回的已经是 dB 值；min/maxDecibels 只影响
        // getByteFrequencyData 的字节缩放，对本路径不生效，故下面不能再取一次对数。
        source.connect_with_audio_node(&analyser)?;

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

        sample_rate.set(f64::from(ctx.sample_rate()));

        let buf = Rc::new(RefCell::new(vec![0u8; BINS * WATERFALL_H as usize * 4]));
        let mut freq = vec![0f32; BINS];

        let tick = Closure::<dyn FnMut()>::new(move || {
          analyser.get_float_frequency_data(&mut freq);
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
              let _ = wctx.put_image_data(&img, 0.0, 0.0);
            }
          }
        });
        let interval = window().set_interval_with_callback_and_timeout_and_arguments_0(
          tick.as_ref().unchecked_ref(),
          TICK_MS,
        )?;
        Ok(Session {
          ctx,
          stream,
          interval,
          _tick: tick,
        })
      }
      .await;
      match result {
        Ok(s) if status.try_get_untracked() != Some(Status::Starting) => s.stop(),
        Ok(s) => {
          session.set_value(Some(s));
          status.set(Status::Listening);
        }
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
      <PageContainer class="space-y-5">
        <section class="rounded-xl border bg-card">
          <div class="flex flex-wrap items-center gap-3 border-b px-4 py-3">
            <div class="mr-auto">
              <h1 class="text-base font-semibold leading-tight">{move || t("tools.sdr-waterfall")}</h1>
              <p class="text-xs text-muted-foreground">
                {move || t("tools.feed-the-radio-or")}
              </p>
            </div>
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
          </div>

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

            <div>
              <div class="mb-1 text-xs font-medium text-muted-foreground">{move || t("tools.live-spectrum")}</div>
              <canvas
                node_ref=spectrum_ref
                width="1024"
                height="160"
                class="h-40 w-full rounded-md bg-black"
                aria-label=move || t("tools.live-spectrum")
              ></canvas>
            </div>

            <div>
              <div class="mb-1 text-xs font-medium text-muted-foreground">{move || t("tools.waterfall-scrolling-down-over")}</div>
              <canvas
                node_ref=waterfall_ref
                width="1024"
                height="256"
                class="h-64 w-full rounded-md bg-black"
                aria-label=move || t("tools.spectrum-waterfall")
              ></canvas>
            </div>

            <p class="text-xs text-muted-foreground">
              {move || t("tools.darker-to-brighter-colours")}
            </p>
          </div>
        </section>
        <FileWaterfall />
      </PageContainer>
    </div>
  }
}
