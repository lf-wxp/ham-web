//! 莫尔斯电码音频合成与播放（Web Audio API）。
//!
//! 用一个共享的 [`AudioContext`] + 振荡器 + 增益节点合成正弦波，
//! 按点/划时长与间隔调度增益包络，实现「滴答」声。
//! 支持停止当前播放、持续侧音（发报练习按住时）、可调音调/音量，以及叠听（重叠双音调）。

use std::cell::RefCell;
use std::time::Duration;

use ham_web_core::koch::{Timing, farnsworth};
use leptos::prelude::{ArcRwSignal, Set, set_timeout};
use wasm_bindgen::JsCast;
use web_sys::{AudioContext, OscillatorType};

pub use ham_web_core::morse::{morse_char_times, morse_symbol_times};

/// 默认音调（Hz）。
pub const DEFAULT_TONE_HZ: f32 = 700.0;
/// 默认音量（0–1）。
pub const DEFAULT_VOLUME: f32 = 1.0;

/// 一组正在发声的节点（振荡器 + 增益），用于停止/断开。
struct Active {
  osc: web_sys::OscillatorNode,
  gain: web_sys::GainNode,
}

impl Active {
  /// 立即停止并断开（忽略已停止/已断开的错误）。
  fn stop_and_disconnect(self) {
    let _ = self.osc.stop();
    let _ = self.osc.disconnect();
    let _ = self.gain.disconnect();
  }
}

thread_local! {
  static AUDIO_CTX: RefCell<Option<AudioContext>> = const { RefCell::new(None) };
  /// 正在播放的摩尔斯音频（可多个，叠听时），新播放前会先停止旧的，避免叠加。
  static ACTIVE: RefCell<Vec<Active>> = const { RefCell::new(Vec::new()) };
  /// 持续侧音（发报练习按住时）。
  static SIDETONE: RefCell<Option<Active>> = const { RefCell::new(None) };
  /// 全局「是否有摩尔斯音频在播放」信号（供波形指示器读取）。
  ///
  /// 用 `ArcRwSignal` 而非 `RwSignal`：`RwSignal` 是 arena 分配的，会随创建时所在的
  /// 响应式 `Owner` 一起被释放，而首次访问可能发生在摩尔斯页组件内部 —— 离开该页面后
  /// 这个「全局」信号就失效了。引用计数信号只要还有引用就一直有效。
  static PLAYING: ArcRwSignal<bool> = ArcRwSignal::new(false);
  /// 播放代数计数，用于失效旧的「播放结束」定时器。
  static PLAY_GEN: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// 全局「是否有摩尔斯音频在播放」信号。
pub fn playing_signal() -> ArcRwSignal<bool> {
  PLAYING.with(|s| s.clone())
}

/// 标记正在播放，并在 `dur` 秒后复位。
fn mark_playing(dur: f64) {
  let s = playing_signal();
  s.set(true);
  let seq = PLAY_GEN.with(|g| {
    let n = g.get().wrapping_add(1);
    g.set(n);
    n
  });
  set_timeout(
    move || {
      if PLAY_GEN.with(|g| g.get()) == seq {
        s.set(false);
      }
    },
    Duration::from_secs_f64(dur.max(0.12)),
  );
}

/// 获取（或创建）共享的 [`AudioContext`]。
fn context() -> Option<AudioContext> {
  AUDIO_CTX.with(|slot| {
    let mut slot = slot.borrow_mut();
    if slot.is_none() {
      *slot = AudioContext::new().ok();
    }
    slot.clone()
  })
}

/// 调度一段摩尔斯音频（不停止已有播放），返回 `(节点, 时长)`。`offset` 为相对当前时刻的延迟。
fn schedule(
  code: &str,
  timing: Timing,
  freq: f32,
  level: f32,
  offset: f64,
) -> Option<(Active, f64)> {
  let ctx = context()?;
  let _ = ctx.resume();

  let dot = timing.dot;
  let osc = ctx.create_oscillator().ok()?;
  let gain = ctx.create_gain().ok()?;
  osc.set_type(OscillatorType::Sine);
  osc.frequency().set_value(freq);
  gain.gain().set_value(0.0);
  if osc.connect_with_audio_node(&gain).is_err() {
    return None;
  }
  if gain.connect_with_audio_node(&ctx.destination()).is_err() {
    return None;
  }

  // 短淡入淡出，避免开关瞬间爆音
  let ramp = (dot * 0.25).min(0.006);
  let t0 = ctx.current_time() + 0.02 + offset;
  let mut t = t0;

  // 按字符（空格分隔）调度：字符内点划间隔 1 个点长，字符 / 单词间隔取 `timing`。
  let mut chars: Vec<(Vec<char>, bool)> = Vec::new();
  for token in code.split(' ').filter(|s| !s.is_empty()) {
    if token == "/" {
      if let Some(last) = chars.last_mut() {
        last.1 = true;
      }
    } else {
      chars.push((
        token.chars().filter(|&c| c == '.' || c == '-').collect(),
        false,
      ));
    }
  }
  for (ci, (marks, word_end)) in chars.iter().enumerate() {
    for (mi, &c) in marks.iter().enumerate() {
      let dur = if c == '.' { dot } else { 3.0 * dot };
      let _ = gain.gain().set_value_at_time(0.0, t);
      let _ = gain.gain().linear_ramp_to_value_at_time(level, t + ramp);
      let _ = gain.gain().set_value_at_time(level, t + dur - ramp);
      let _ = gain.gain().linear_ramp_to_value_at_time(0.0, t + dur);
      t += dur;
      if mi + 1 < marks.len() {
        t += dot; // 字符内点划间隔
      }
    }
    if ci + 1 < chars.len() {
      t += if *word_end {
        timing.word_gap
      } else {
        timing.char_gap
      };
    }
  }

  let _ = osc.start();
  let _ = osc.stop_with_when(t + 0.05);
  auto_disconnect(&osc, &gain);
  Some((Active { osc, gain }, t - t0))
}

/// 播放一段摩尔斯点划序列；`wpm` 为每分钟单词数（点长 = 1200 / wpm 毫秒）。
///
/// `.` 表示点、`-` 表示划，` `（空格）表示字符间隔；其余字符忽略。
pub fn play_morse(code: &str, wpm: f64) {
  play_morse_timed(code, farnsworth(wpm, wpm));
}

/// 按指定时值（如 Farnsworth）播放，返回播放总时长（秒）。
pub fn play_morse_timed(code: &str, timing: Timing) -> f64 {
  play_morse_timed_with(code, timing, DEFAULT_TONE_HZ, DEFAULT_VOLUME)
}

/// 按指定时值、音调与音量播放，返回播放总时长（秒）。
pub fn play_morse_timed_with(code: &str, timing: Timing, freq: f32, level: f32) -> f64 {
  stop_morse();
  let Some((active, dur)) = schedule(code, timing, freq, level, 0.0) else {
    return 0.0;
  };
  ACTIVE.with(|slot| slot.borrow_mut().push(active));
  mark_playing(dur);
  dur
}

/// 叠听：干扰台（较低音调）与目标台（正常音调）重叠播放，返回总时长（秒）。
pub fn play_pileup(decoy: &str, target: &str, timing: Timing, freq: f32, level: f32) -> f64 {
  stop_morse();
  let decoy_dur = schedule(decoy, timing, freq * 0.72, level, 0.0)
    .map(|(a, d)| {
      ACTIVE.with(|slot| slot.borrow_mut().push(a));
      d
    })
    .unwrap_or(0.0);
  let offset = (decoy_dur * 0.55).max(0.25);
  let target_dur = schedule(target, timing, freq, level, offset)
    .map(|(a, d)| {
      ACTIVE.with(|slot| slot.borrow_mut().push(a));
      d
    })
    .unwrap_or(0.0);
  let total = offset + target_dur;
  mark_playing(total);
  total
}

/// 停止当前播放的摩尔斯音频（如有）。
pub fn stop_morse() {
  ACTIVE.with(|slot| {
    for nodes in slot.borrow_mut().drain(..) {
      nodes.stop_and_disconnect();
    }
  });
  playing_signal().set(false);
}

/// 开始一段持续侧音（发报练习按住电键时调用）。重复调用会先停止旧侧音。
pub fn start_tone(freq: f32, level: f32) {
  stop_tone();
  let Some(ctx) = context() else { return };
  let _ = ctx.resume();

  let Ok(osc) = ctx.create_oscillator() else {
    return;
  };
  let Ok(gain) = ctx.create_gain() else { return };
  osc.set_type(OscillatorType::Sine);
  osc.frequency().set_value(freq);
  gain.gain().set_value(0.0);
  if osc.connect_with_audio_node(&gain).is_err() {
    return;
  }
  if gain.connect_with_audio_node(&ctx.destination()).is_err() {
    return;
  }

  // 短淡入，避免爆音
  let t0 = ctx.current_time();
  let ramp = 0.006;
  let _ = gain.gain().set_value_at_time(0.0, t0);
  let _ = gain.gain().linear_ramp_to_value_at_time(level, t0 + ramp);

  let _ = osc.start();
  SIDETONE.with(|slot| {
    *slot.borrow_mut() = Some(Active {
      osc: osc.clone(),
      gain: gain.clone(),
    });
  });
  playing_signal().set(true);
}

/// 停止当前侧音（松开电键时调用）。短淡出后停止。
pub fn stop_tone() {
  SIDETONE.with(|slot| {
    if let Some(nodes) = slot.borrow_mut().take() {
      if let Some(ctx) = context() {
        let now = ctx.current_time();
        let _ = nodes.gain.gain().cancel_scheduled_values(now);
        let _ = nodes
          .gain
          .gain()
          .set_value_at_time(nodes.gain.gain().value(), now);
        let _ = nodes
          .gain
          .gain()
          .linear_ramp_to_value_at_time(0.0, now + 0.008);
        let _ = nodes.osc.stop_with_when(now + 0.02);
        auto_disconnect(&nodes.osc, &nodes.gain);
      } else {
        nodes.stop_and_disconnect();
      }
    }
  });
  playing_signal().set(false);
}

/// 播放结束后断开振荡器与增益节点的连接，避免节点累积等待 GC。
fn auto_disconnect(osc: &web_sys::OscillatorNode, gain: &web_sys::GainNode) {
  let osc_node = osc.clone();
  let gain_node = gain.clone();
  let onended = wasm_bindgen::closure::Closure::once_into_js(move || {
    let _ = osc_node.disconnect();
    let _ = gain_node.disconnect();
  });
  osc.set_onended(Some(onended.as_ref().unchecked_ref()));
}

/// 播放一个指定频率、音量（0–1）与时长的纯音，用于 RST 信号强度等演示。
pub fn play_tone(freq: f32, level: f32, duration: f64) {
  let Some(ctx) = context() else { return };
  let _ = ctx.resume();

  let Ok(osc) = ctx.create_oscillator() else {
    return;
  };
  let Ok(gain) = ctx.create_gain() else { return };
  osc.set_type(OscillatorType::Sine);
  osc.frequency().set_value(freq);
  gain.gain().set_value(0.0);
  if osc.connect_with_audio_node(&gain).is_err() {
    return;
  }
  if gain.connect_with_audio_node(&ctx.destination()).is_err() {
    return;
  }

  let ramp = 0.012;
  let t0 = ctx.current_time() + 0.02;
  let _ = gain.gain().set_value_at_time(0.0, t0);
  let _ = gain.gain().linear_ramp_to_value_at_time(level, t0 + ramp);
  let _ = gain.gain().set_value_at_time(level, t0 + duration - ramp);
  let _ = gain.gain().linear_ramp_to_value_at_time(0.0, t0 + duration);

  let _ = osc.start();
  let _ = osc.stop_with_when(t0 + duration + 0.05);
  auto_disconnect(&osc, &gain);
}
