//! 生成一段合成的 NOAA APT 音频样本（WAV），用于手动测试解码器。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-apt --release --example generate_sample -- [输出.wav] [行数] [噪声幅度]
//! ```
//! - 行数：默认 400（约 200 秒）；每行 0.5 秒。
//! - 噪声幅度：默认 0.02，加在 2400 Hz 副载波上，模拟真实接收的轻微噪声；0 表示无噪声。
//!
//! 图像内容（便于验证解码）：
//! - 通道 A：水平渐变（左黑右白），验证灰度方向与楔形校准；
//! - 通道 B：纵向渐变（首行黑、末行白），验证行序与两通道区分。

use ham_web_apt::{
  IMAGE_WORDS, LINE_WORDS, SPACE_WORDS, SYNC_A_HZ, SYNC_B_HZ, SYNC_WORDS, TELEMETRY_WORDS,
  WORD_RATE, WORK_RATE,
};

const RATE: usize = WORD_RATE as usize; // 4160

fn main() {
  let mut args = std::env::args().skip(1);
  let out_path = args.next().unwrap_or_else(|| "apt-sample.wav".to_owned());
  let lines: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(400);
  let noise: f32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(0.02);

  let env = build_envelope(lines);
  let up = upsample(&env, 5);
  let mut signal = modulate(&up);
  if noise > 0.0 {
    ham_web_dsp::add_noise(&mut signal, noise);
  }
  let wav = ham_web_wav::encode(&signal, WORK_RATE);
  std::fs::write(&out_path, &wav).expect("写入 WAV 失败");
  println!(
    "已生成 {out_path}：{lines} 行 ≈ {:.1}s，采样率 {WORK_RATE} Hz，噪声幅度 {noise:.2}",
    lines as f64 * 0.5
  );
}

/// 构造 `lines` 行的视频包络（字速率 4160 Hz），包含完整的 sync / space / 图像 / 遥测。
fn build_envelope(lines: usize) -> Vec<f32> {
  let mut env = Vec::with_capacity(lines * LINE_WORDS);
  for line in 0..lines {
    // 通道 B 的整体亮度随行号从黑到白（纵向渐变）。
    let row_b = 0.9 - 0.8 * (line as f32 / (lines.max(1) - 1) as f32);

    for i in 0..SYNC_WORDS {
      env.push(0.5 + 0.5 * (std::f32::consts::TAU * SYNC_A_HZ * i as f32 / RATE as f32).sin());
    }
    env.extend(std::iter::repeat_n(0.9, SPACE_WORDS));
    // 图像 A：水平渐变（黑 → 白）。
    for col in 0..IMAGE_WORDS {
      env.push(0.9 - 0.8 * (col as f32 / (IMAGE_WORDS - 1) as f32));
    }
    for i in 0..TELEMETRY_WORDS {
      env.push(0.9 - 0.8 * (i as f32 / (TELEMETRY_WORDS - 1) as f32));
    }

    for i in 0..SYNC_WORDS {
      env.push(0.5 + 0.5 * (std::f32::consts::TAU * SYNC_B_HZ * i as f32 / RATE as f32).sin());
    }
    env.extend(std::iter::repeat_n(0.9, SPACE_WORDS));
    // 图像 B：整行同一亮度（随行号变化）。
    env.extend(std::iter::repeat_n(row_b, IMAGE_WORDS));
    for i in 0..TELEMETRY_WORDS {
      env.push(0.9 - 0.8 * (i as f32 / (TELEMETRY_WORDS - 1) as f32));
    }
  }
  env
}

/// 线性上采样到工作采样率（×5：4160 → 20800）。
fn upsample(env: &[f32], factor: usize) -> Vec<f32> {
  let mut out = Vec::with_capacity(env.len() * factor);
  for i in 0..env.len() * factor {
    let pos = i as f64 / factor as f64;
    let p0 = pos.floor() as usize;
    let p1 = (p0 + 1).min(env.len() - 1);
    let frac = (pos - p0 as f64) as f32;
    out.push(env[p0] + (env[p1] - env[p0]) * frac);
  }
  out
}

/// 用 2400 Hz 副载波 AM 调制。
fn modulate(up: &[f32]) -> Vec<f32> {
  let carrier = 2400.0f32;
  let rate = WORK_RATE as usize;
  up.iter()
    .enumerate()
    .map(|(i, &e)| e * (std::f32::consts::TAU * carrier * i as f32 / rate as f32).cos())
    .collect()
}
