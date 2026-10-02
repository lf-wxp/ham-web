//! 端到端集成测试：合成一段完整的 APT 信号并解码，验证图像方向与尺寸。

use ham_web_apt::{
  CHANNEL_WORDS, IMAGE_WORDS, LINE_WORDS, SPACE_WORDS, SYNC_A_HZ, SYNC_B_HZ, SYNC_WORDS,
  TELEMETRY_WORDS, WORD_RATE, WORK_RATE,
};

const RATE: usize = WORD_RATE as usize; // 4160

/// 构造 `lines` 行的视频包络（字速率 4160 Hz）。
fn build_envelope(lines: usize) -> Vec<f32> {
  let mut env = Vec::with_capacity(lines * LINE_WORDS);
  for _ in 0..lines {
    // sync A：1040 Hz 正弦
    for i in 0..SYNC_WORDS {
      env.push(0.5 + 0.5 * (std::f32::consts::TAU * SYNC_A_HZ * i as f32 / RATE as f32).sin());
    }
    // space：黑参考
    env.extend(std::iter::repeat_n(0.9, SPACE_WORDS));
    // 图像 A：黑 → 白 斜坡
    for i in 0..IMAGE_WORDS {
      env.push(0.9 - 0.8 * (i as f32 / (IMAGE_WORDS - 1) as f32));
    }
    // 遥测 A：黑 → 白 楔形
    for i in 0..TELEMETRY_WORDS {
      env.push(0.9 - 0.8 * (i as f32 / (TELEMETRY_WORDS - 1) as f32));
    }
    // sync B：832 Hz 正弦
    for i in 0..SYNC_WORDS {
      env.push(0.5 + 0.5 * (std::f32::consts::TAU * SYNC_B_HZ * i as f32 / RATE as f32).sin());
    }
    // space B
    env.extend(std::iter::repeat_n(0.9, SPACE_WORDS));
    // 图像 B：白 → 黑 斜坡
    for i in 0..IMAGE_WORDS {
      env.push(0.1 + 0.8 * (i as f32 / (IMAGE_WORDS - 1) as f32));
    }
    // 遥测 B
    for i in 0..TELEMETRY_WORDS {
      env.push(0.9 - 0.8 * (i as f32 / (TELEMETRY_WORDS - 1) as f32));
    }
  }
  env
}

/// 线性上采样到工作采样率。
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

/// 用 2400 Hz 副载波 AM 调制并量化，写为 16 位 PCM WAV。
fn make_wav(signal: &[f32], rate: u32) -> Vec<u8> {
  let data_len = signal.len() * 2;
  let mut out = Vec::with_capacity(44 + data_len);
  out.extend_from_slice(b"RIFF");
  out.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
  out.extend_from_slice(b"WAVE");
  out.extend_from_slice(b"fmt ");
  out.extend_from_slice(&16u32.to_le_bytes());
  out.extend_from_slice(&1u16.to_le_bytes());
  out.extend_from_slice(&1u16.to_le_bytes());
  out.extend_from_slice(&rate.to_le_bytes());
  out.extend_from_slice(&(rate * 2).to_le_bytes());
  out.extend_from_slice(&2u16.to_le_bytes());
  out.extend_from_slice(&16u16.to_le_bytes());
  out.extend_from_slice(b"data");
  out.extend_from_slice(&(data_len as u32).to_le_bytes());
  for &s in signal {
    out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
  }
  out
}

#[test]
fn decodes_synthetic_apt_image() {
  let lines = 8;
  let env = build_envelope(lines);
  let up = upsample(&env, (WORK_RATE / WORD_RATE) as usize);

  // AM 调制到 2400 Hz 副载波
  let carrier = 2400.0f32;
  let signal: Vec<f32> = up
    .iter()
    .enumerate()
    .map(|(i, &e)| e * (std::f32::consts::TAU * carrier * i as f32 / WORK_RATE as f32).cos())
    .collect();

  let wav = make_wav(&signal, WORK_RATE);
  let img = ham_web_apt::decode(&wav).expect("解码应成功");

  assert_eq!(img.width, IMAGE_WORDS as u32);
  assert!(
    img.lines >= lines - 1,
    "应检测到接近 {} 行，实际 {} 行",
    lines,
    img.lines
  );

  let w = img.width as usize;
  // 通道 A：左黑右白；通道 B：左白右黑
  assert!(img.channel_a[0] < img.channel_a[w - 1], "通道 A 应为黑→白");
  assert!(img.channel_b[0] > img.channel_b[w - 1], "通道 B 应为白→黑");
}

#[test]
fn rejects_non_wav_input() {
  assert!(ham_web_apt::decode(b"not a wav at all").is_err());
}

#[test]
fn constants_are_consistent() {
  assert_eq!(
    SYNC_WORDS + SPACE_WORDS + IMAGE_WORDS + TELEMETRY_WORDS,
    CHANNEL_WORDS
  );
  assert_eq!(CHANNEL_WORDS * 2, LINE_WORDS);
}
