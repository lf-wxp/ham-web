//! 生成一段合成的 SSTV 音频样本（WAV），用于手动测试解码器。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-sstv --release --example generate_sample -- [输出.wav] [VIS码] [噪声幅度]
//! ```
//! - VIS 码：默认 44（Martin M1）；支持 44/40（Martin）、60/56/76（Scottie）、8/12（Robot）。
//! - 噪声幅度：默认 0.02，加在 FM 信号上模拟真实接收；0 表示无噪声。
//!
//! 图像内容（便于验证解码）：水平灰度渐变（左黑右白），RGB 与 Robot 的 Y 通道均为同一渐变。

use std::f32::consts::TAU;

use ham_web_sstv::{BLACK_HZ, ColorKind, MODES, WHITE_HZ, WORK_RATE};

fn main() {
  let mut args = std::env::args().skip(1);
  let out_path = args.next().unwrap_or_else(|| "sstv-sample.wav".to_owned());
  let vis: u8 = args.next().and_then(|s| s.parse().ok()).unwrap_or(44);
  let noise: f32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(0.02);

  let mode = MODES.iter().find(|m| m.vis_code == vis).unwrap_or_else(|| {
    let list = MODES
      .iter()
      .map(|m| format!("{}（{}）", m.name, m.vis_code))
      .collect::<Vec<_>>()
      .join("、");
    panic!("未知 VIS 码 {vis}，支持：{list}");
  });

  let freq = build_freq(mode);
  let mut signal = fm_modulate(&freq);
  if noise > 0.0 {
    add_noise(&mut signal, noise);
  }
  let wav = make_wav(&signal);
  std::fs::write(&out_path, &wav).expect("写入 WAV 失败");
  println!(
    "已生成 {out_path}：模式 {}，{}×{}，{} 样本 ≈ {:.1}s，噪声幅度 {noise:.2}",
    mode.name,
    mode.width,
    mode.height,
    signal.len(),
    signal.len() as f32 / WORK_RATE as f32
  );
}

/// 构造整段频率序列：leader + VIS + 逐行（sync + porch + 扫描）。
fn build_freq(mode: &ham_web_sstv::Mode) -> Vec<f32> {
  let mut freq = Vec::new();
  push_tone(&mut freq, 1900.0, 0.2); // leader
  push_vis(&mut freq, mode.vis_code);

  let pixel_s = mode.pixel_us / 1_000_000.0;
  for _row in 0..mode.height {
    push_tone(&mut freq, 1200.0, mode.sync_ms / 1000.0); // sync
    if mode.porch_ms > 0.0 {
      push_tone(&mut freq, BLACK_HZ, mode.porch_ms / 1000.0); // porch
    }
    match mode.color {
      ColorKind::Rgb => {
        for col in 0..mode.width {
          let f = BLACK_HZ + (col as f32 / (mode.width - 1) as f32) * (WHITE_HZ - BLACK_HZ);
          for _ in 0..3 {
            push_tone(&mut freq, f, pixel_s); // R、G、B 同值
          }
        }
      }
      ColorKind::Yc => {
        for col in 0..mode.width {
          let f = BLACK_HZ + (col as f32 / (mode.width - 1) as f32) * (WHITE_HZ - BLACK_HZ);
          push_tone(&mut freq, f, pixel_s); // Y
        }
        // 两路色度置零（1500 Hz），输出应为灰色渐变。
        for _ in 0..(mode.width / 2) {
          push_tone(&mut freq, 1500.0, pixel_s);
        }
        for _ in 0..(mode.width / 2) {
          push_tone(&mut freq, 1500.0, pixel_s);
        }
      }
    }
  }
  freq
}

fn push_tone(freq: &mut Vec<f32>, hz: f32, seconds: f32) {
  let n = (seconds * WORK_RATE as f32).round() as usize;
  freq.extend(std::iter::repeat_n(hz, n));
}

/// VIS：起始（1200 Hz，300 ms）+ 8 位（各 30 ms，1100=0 / 1300=1，LSB 先发）+ 停止（1200，30 ms）。
fn push_vis(freq: &mut Vec<f32>, code: u8) {
  push_tone(freq, 1200.0, 0.300);
  let parity = code.count_ones() % 2;
  let full = code | ((parity as u8) << 7);
  for b in 0..8 {
    let f = if (full >> b) & 1 == 1 { 1300.0 } else { 1100.0 };
    push_tone(freq, f, 0.030);
  }
  push_tone(freq, 1200.0, 0.030);
}

/// 频率序列 → FM 调制样本（相位累加）。
fn fm_modulate(freq: &[f32]) -> Vec<f32> {
  let mut out = Vec::with_capacity(freq.len());
  let mut phase = 0.0f32;
  for &f in freq {
    out.push(phase.sin());
    phase = (phase + TAU * f / WORK_RATE as f32) % TAU;
  }
  out
}

/// 叠加均匀白噪声（幅度 `amp`）。
fn add_noise(signal: &mut [f32], amp: f32) {
  let mut seed = 0x1234_5678u32;
  for s in signal {
    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    let u = (seed >> 8) as f32 / 16_777_216.0;
    *s += amp * (u * 2.0 - 1.0);
  }
}

/// 16 位 PCM 单声道 WAV（采样率 [`WORK_RATE`]）。
fn make_wav(signal: &[f32]) -> Vec<u8> {
  let data_len = signal.len() * 2;
  let mut out = Vec::with_capacity(44 + data_len);
  out.extend_from_slice(b"RIFF");
  out.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
  out.extend_from_slice(b"WAVEfmt ");
  out.extend_from_slice(&16u32.to_le_bytes());
  out.extend_from_slice(&1u16.to_le_bytes());
  out.extend_from_slice(&1u16.to_le_bytes());
  out.extend_from_slice(&WORK_RATE.to_le_bytes());
  out.extend_from_slice(&(WORK_RATE * 2).to_le_bytes());
  out.extend_from_slice(&2u16.to_le_bytes());
  out.extend_from_slice(&16u16.to_le_bytes());
  out.extend_from_slice(b"data");
  out.extend_from_slice(&(data_len as u32).to_le_bytes());
  for &s in signal {
    out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
  }
  out
}
