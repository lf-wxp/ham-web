//! 莫尔斯电码音频合成与播放（Web Audio API）。
//!
//! 用一个共享的 [`AudioContext`] + 振荡器 + 增益节点合成正弦波，
//! 按点/划时长与间隔调度增益包络，实现「滴答」声。

use std::cell::RefCell;

use wasm_bindgen::JsCast;
use web_sys::{AudioContext, OscillatorType};

/// 默认音调（Hz）。
const FREQ: f32 = 700.0;

thread_local! {
  static AUDIO_CTX: RefCell<Option<AudioContext>> = const { RefCell::new(None) };
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

/// 播放一段摩尔斯点划序列；`wpm` 为每分钟单词数（点长 = 1200 / wpm 毫秒）。
///
/// `.` 表示点、`-` 表示划，` `（空格）表示字符间隔；其余字符忽略。
pub fn play_morse(code: &str, wpm: f64) {
  let Some(ctx) = context() else { return };
  // 首次播放时解除浏览器自动播放限制（对已运行的上下文幂等）
  let _ = ctx.resume();

  let dot = 1.2 / wpm; // 一个「点」的时长（秒）
  let Ok(osc) = ctx.create_oscillator() else {
    return;
  };
  let Ok(gain) = ctx.create_gain() else { return };
  osc.set_type(OscillatorType::Sine);
  osc.frequency().set_value(FREQ);
  gain.gain().set_value(0.0);
  if osc.connect_with_audio_node(&gain).is_err() {
    return;
  }
  if gain.connect_with_audio_node(&ctx.destination()).is_err() {
    return;
  }

  // 短淡入淡出，避免开关瞬间爆音
  let ramp = (dot * 0.25).min(0.006);
  let t0 = ctx.current_time() + 0.02;
  let mut t = t0;

  // 按字符（空格分隔）调度，字符内点划间隔 1 单位、字符间隔 3 单位
  let chars: Vec<Vec<char>> = code
    .split(' ')
    .filter(|s| !s.is_empty())
    .map(|s| s.chars().filter(|&c| c == '.' || c == '-').collect())
    .collect();
  for (ci, marks) in chars.iter().enumerate() {
    for (mi, &c) in marks.iter().enumerate() {
      let dur = if c == '.' { dot } else { 3.0 * dot };
      let _ = gain.gain().set_value_at_time(0.0, t);
      let _ = gain.gain().linear_ramp_to_value_at_time(1.0, t + ramp);
      let _ = gain.gain().set_value_at_time(1.0, t + dur - ramp);
      let _ = gain.gain().linear_ramp_to_value_at_time(0.0, t + dur);
      t += dur;
      if mi + 1 < marks.len() {
        t += dot; // 字符内点划间隔
      }
    }
    if ci + 1 < chars.len() {
      t += 3.0 * dot; // 字符间隔
    }
  }

  let _ = osc.start();
  let _ = osc.stop_with_when(t + 0.05);
  auto_disconnect(&osc, &gain);
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
