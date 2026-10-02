//! 麦克风 CW 解码：带通滤波 + 包络检测得到按键时序，交给核心解码器还原文字。

use std::cell::RefCell;
use std::rc::Rc;

use ham_web_core::cw_decoder::Decoder;
use js_sys::{Object, Reflect};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
  AnalyserNode, AudioContext, BiquadFilterNode, BiquadFilterType, MediaStream,
  MediaStreamConstraints,
};

use crate::i18n::{t, tf};
use crate::ui::{Size, Variant, button_class};
use crate::util::{copy_text, js_error_message, window};

/// 轮询周期（毫秒）。时序以音频时钟为准，定时器迟到不影响点划判断。
const TICK_MS: i32 = 10;
/// 分析缓冲：4096 点在 48 kHz 下约 85 ms，定时器迟到不超过这个时长就不会丢样本。
const FFT_SIZE: u32 = 4096;
/// 包络检测的块长（秒）。
const BLOCK_SECS: f64 = 0.005;
/// 带通滤波器 Q 值：600 Hz 时带宽约 40 Hz。
const FILTER_Q: f32 = 15.0;
/// 峰值至少高出底噪这么多倍才认为有信号。
const MIN_SNR: f32 = 4.0;
/// 每隔多少个周期刷新一次电平表。
const METER_EVERY: u32 = 4;

/// 运行中的音频链路。
struct Session {
  ctx: AudioContext,
  stream: MediaStream,
  filter: BiquadFilterNode,
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

/// 自适应门限的包络检测：跟踪底噪与峰值，取二者之间带回差的门限。
#[derive(Default)]
struct Envelope {
  floor: f32,
  peak: f32,
  on: bool,
}

impl Envelope {
  fn update(&mut self, level: f32) -> bool {
    if self.floor == 0.0 || level < self.floor {
      self.floor = level;
    } else {
      self.floor += (level - self.floor) * 0.002;
    }
    if level > self.peak {
      self.peak = level;
    } else {
      self.peak -= (self.peak - self.floor) * 0.0015;
    }
    let span = self.peak - self.floor;
    let threshold = self.floor + span * if self.on { 0.35 } else { 0.5 };
    self.on = self.peak > self.floor * MIN_SNR && level > threshold;
    self.on
  }

  /// 当前电平在 [底噪, 峰值] 中的相对位置。
  fn relative(&self, level: f32) -> f32 {
    let span = self.peak - self.floor;
    if span <= f32::EPSILON {
      0.0
    } else {
      ((level - self.floor) / span).clamp(0.0, 1.0)
    }
  }
}

fn audio_constraints() -> MediaStreamConstraints {
  // 关闭回声消除 / 降噪 / 自动增益：它们会把稳定的单音当作噪声压掉
  let audio = Object::new();
  for key in ["echoCancellation", "noiseSuppression", "autoGainControl"] {
    let _ = Reflect::set(&audio, &key.into(), &JsValue::FALSE);
  }
  let c = MediaStreamConstraints::new();
  c.set_audio(&audio);
  c
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
  Idle,
  Starting,
  Listening,
}

#[component]
pub fn CwDecoder() -> impl IntoView {
  let supported = window().navigator().media_devices().is_ok();
  let status = RwSignal::new(Status::Idle);
  let error = RwSignal::new(None::<String>);
  let tone = RwSignal::new(600u32);
  let text = RwSignal::new(String::new());
  let pending = RwSignal::new(String::new());
  let target = RwSignal::new(String::new());
  let wpm = RwSignal::new(0.0f64);
  let level = RwSignal::new(0.0f32);
  let keyed = RwSignal::new(false);
  let session = StoredValue::new_local(None::<Session>);
  let decoder = StoredValue::new_local(Rc::new(RefCell::new(Decoder::new())));

  let stop = move || {
    if let Some(s) = session.try_update_value(Option::take).flatten() {
      s.stop();
    }
    keyed.set(false);
    level.set(0.0);
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
        let filter = ctx.create_biquad_filter()?;
        filter.set_type(BiquadFilterType::Bandpass);
        #[allow(clippy::cast_precision_loss)]
        filter.frequency().set_value(tone.get_untracked() as f32);
        filter.q().set_value(FILTER_Q);
        let analyser: AnalyserNode = ctx.create_analyser()?;
        analyser.set_fft_size(FFT_SIZE);
        analyser.set_smoothing_time_constant(0.0);
        source.connect_with_audio_node(&filter)?;
        filter.connect_with_audio_node(&analyser)?;

        let dec = decoder.get_value();
        let mut env = Envelope::default();
        let mut buf = vec![0f32; FFT_SIZE as usize];
        let clock = ctx.clone();
        let rate = f64::from(ctx.sample_rate());
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let block = ((rate * BLOCK_SECS) as usize).max(1);
        // 已处理到的音频时间（秒）与上次状态切换的时间
        let mut processed = clock.current_time();
        let mut last_change = processed;
        let mut ticks = 0u32;
        let tick = Closure::<dyn FnMut()>::new(move || {
          let now = clock.current_time();
          #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
          let mut fresh = ((now - processed) * rate) as usize;
          if fresh > buf.len() {
            // 标签页被挂起过：丢弃缓冲之外的部分
            #[allow(clippy::cast_precision_loss)]
            let skipped = (fresh - buf.len()) as f64 / rate;
            processed += skipped;
            fresh = buf.len();
          }
          let blocks = fresh / block;
          if blocks == 0 {
            return;
          }
          analyser.get_float_time_domain_data(&mut buf);
          let start = buf.len() - blocks * block;
          let mut d = dec.borrow_mut();
          let mut rms = 0.0;
          for (i, chunk) in buf[start..].chunks_exact(block).enumerate() {
            #[allow(clippy::cast_precision_loss)]
            {
              rms = (chunk.iter().map(|x| x * x).sum::<f32>() / block as f32).sqrt();
            }
            #[allow(clippy::cast_precision_loss)]
            let t = processed + ((i + 1) * block) as f64 / rate;
            let was_on = env.on;
            let on = env.update(rms);
            if on != was_on {
              d.key(was_on, (t - last_change) * 1000.0);
              last_change = t;
            }
          }
          #[allow(clippy::cast_precision_loss)]
          {
            processed += (blocks * block) as f64 / rate;
          }
          if !env.on {
            d.idle((processed - last_change) * 1000.0);
          }
          if keyed.get_untracked() != env.on {
            keyed.set(env.on);
          }
          if text.with_untracked(|t| t != d.text()) {
            text.set(d.text().to_owned());
            wpm.set(d.wpm());
          }
          if pending.with_untracked(|p| p != d.pending()) {
            pending.set(d.pending().to_owned());
          }
          ticks = ticks.wrapping_add(1);
          if ticks.is_multiple_of(METER_EVERY) {
            level.set(env.relative(rms));
          }
        });
        let interval = window().set_interval_with_callback_and_timeout_and_arguments_0(
          tick.as_ref().unchecked_ref(),
          TICK_MS,
        )?;
        Ok(Session {
          ctx,
          stream,
          filter,
          interval,
          _tick: tick,
        })
      }
      .await;
      match result {
        // 用户在等待授权时离开了页面或点了停止
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
            t("没有麦克风权限，请在地址栏允许后重试。")
          } else {
            js_error_message(&e)
          }));
          status.set(Status::Idle);
        }
      }
    });
  };

  let set_tone = move |hz: u32| {
    tone.set(hz);
    session.with_value(|s| {
      if let Some(s) = s {
        #[allow(clippy::cast_precision_loss)]
        s.filter.frequency().set_value(hz as f32);
      }
    });
  };

  let clear = move || {
    decoder.with_value(|d| d.borrow_mut().clear());
    text.set(String::new());
    pending.set(String::new());
  };

  on_cleanup(move || {
    if let Some(s) = session.try_update_value(Option::take).flatten() {
      s.stop();
    }
  });

  view! {
    <section class="rounded-xl border bg-card" aria-labelledby="cw-decoder-title">
      <div class="flex flex-wrap items-center gap-3 border-b px-4 py-3">
        <div class="mr-auto">
          <h2 id="cw-decoder-title" class="text-sm font-semibold">{move || t("CW 解码（麦克风）")}</h2>
          <p class="text-xs text-muted-foreground">{move || t("把麦克风靠近电台扬声器，自动识别速度并实时解码。")}</p>
        </div>
        <button
          type="button"
          class=button_class(Variant::Default, Size::Sm, "")
          prop:disabled=move || !supported || status.get() == Status::Starting
          on:click=move |_| if status.get_untracked() == Status::Idle { start() } else { stop() }
        >
          {move || match status.get() {
            Status::Idle => t("开始监听"),
            Status::Starting => t("请求麦克风…"),
            Status::Listening => t("停止"),
          }}
        </button>
      </div>
      <div class="space-y-3 p-4">
        {(!supported).then(|| view! {
          <p role="alert" class="text-sm text-amber-800 dark:text-amber-300">{move || t("当前浏览器无法访问麦克风（需要 HTTPS 与较新的浏览器）。")}</p>
        })}
        {move || error.get().map(|e| view! { <p role="alert" class="text-sm text-destructive">{e}</p> })}
        <div class="flex flex-wrap items-center gap-x-4 gap-y-2 text-xs text-muted-foreground">
          <label class="inline-flex items-center gap-2">
            {move || t("音调")}
            <input
              type="range"
              min="300"
              max="1200"
              step="10"
              aria-label=move || t("音调频率")
              prop:value=move || tone.get().to_string()
              on:input=move |e| set_tone(event_target_value(&e).parse().unwrap_or(600))
            />
            <span class="w-14 tabular-nums text-foreground">{move || format!("{} Hz", tone.get())}</span>
          </label>
          <span class="inline-flex items-center gap-1.5">
            <span
              class=move || if keyed.get() { "h-2.5 w-2.5 rounded-full bg-emerald-500" } else { "h-2.5 w-2.5 rounded-full bg-muted-foreground/30" }
              aria-hidden="true"
            ></span>
            {move || t("电键")}
          </span>
          <span class="inline-flex items-center gap-1.5">
            {move || t("电平")}
            <span class="h-1.5 w-24 overflow-hidden rounded-full bg-muted" aria-hidden="true">
              <span class="block h-full bg-primary transition-[width] duration-75" style:width=move || format!("{:.0}%", level.get() * 100.0)></span>
            </span>
          </span>
          <span class="tabular-nums" data-testid="cw-wpm">
            {move || if text.with(String::is_empty) { t("速度 —") } else { tf("速度 ≈ {} WPM", &[&format!("{:.0}", wpm.get())]) }}
          </span>
        </div>
        <div
          role="log"
          aria-label=move || t("解码结果")
          class="min-h-24 whitespace-pre-wrap break-all rounded-lg bg-muted/40 p-3 font-mono text-sm leading-relaxed"
        >
          {move || text.get()}
          <span class="text-muted-foreground">{move || pending.get()}</span>
          {move || (text.with(String::is_empty) && pending.with(String::is_empty)).then(|| view! {
            <span class="text-muted-foreground">{move || if status.get() == Status::Listening { t("正在监听…") } else { t("解码结果会显示在这里") }}</span>
          })}
        </div>
        <div class="space-y-2">
          <label class="flex items-center gap-2 text-xs text-muted-foreground">
            {move || t("对照目标")}
            <input
              type="text"
              prop:value=move || target.get()
              on:input=move |e| target.set(event_target_value(&e))
              placeholder=move || t("输入目标文本，拍发后对照")
              class="h-8 min-w-0 flex-1 rounded-md border bg-background px-2 text-sm font-mono uppercase outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
            />
          </label>
          {move || {
            let t: String = target
              .get()
              .chars()
              .filter(|c| !c.is_whitespace())
              .map(|c| c.to_ascii_uppercase())
              .collect();
            let d: String = text
              .get()
              .chars()
              .filter(|c| !c.is_whitespace())
              .map(|c| c.to_ascii_uppercase())
              .collect();
            if t.is_empty() {
              return ().into_any();
            }
            let matched = t.chars().zip(d.chars()).take_while(|(a, b)| a == b).count();
            let total = t.chars().count();
            let pct = if total == 0 { 0.0 } else { matched as f64 / total as f64 * 100.0 };
            view! {
              <div class="flex items-center gap-2 text-xs text-muted-foreground">
                <span class="tabular-nums">{tf("对照 {} / {}", &[&(matched).to_string(), &(total).to_string()])}</span>
                <div class="h-1.5 flex-1 overflow-hidden rounded-full bg-muted">
                  <div class="h-full rounded-full bg-emerald-500" style=format!("width: {pct:.0}%")></div>
                </div>
              </div>
            }.into_any()
          }}
        </div>
        <div class="flex gap-2">
          <button type="button" class=button_class(Variant::Outline, Size::Sm, "") on:click=move |_| clear()>{move || t("清空")}</button>
          <button type="button" class=button_class(Variant::Outline, Size::Sm, "") on:click=move |_| text.with_untracked(|t| copy_text(t.trim()))>{move || t("复制")}</button>
        </div>
      </div>
    </section>
  }
}
