//! 生成一段合成的 WSPR 音频样本（WAV），用于手动测试解码器。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-wspr --release --example generate_sample -- [输出.wav] [呼号] [网格] [功率] [噪声]
//! ```
//! - 呼号 / 网格 / 功率：默认 AA0NT / EM18 / 20；
//! - 噪声幅度：默认 0.02，加在 4-FSK 信号上；0 表示无噪声。
//!
//! 采样率 12000 Hz（每符号 8192 样本，恰好让 4 个音调对齐 DFT bin）。

use ham_web_wspr::{encode, synthesize};

const RATE: u32 = 12_000;

fn main() {
  let mut args = std::env::args().skip(1);
  let out_path = args.next().unwrap_or_else(|| "wspr-sample.wav".to_owned());
  let call = args.next().unwrap_or_else(|| "AA0NT".to_owned());
  let locator = args.next().unwrap_or_else(|| "EM18".to_owned());
  let power: u8 = args.next().and_then(|s| s.parse().ok()).unwrap_or(20);
  let noise: f32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(0.02);

  let Some(symbols) = encode(&call, &locator, power) else {
    eprintln!("无效的呼号或网格（网格须为 4 位 Maidenhead）：{call} {locator}");
    std::process::exit(1);
  };
  let mut signal = synthesize(&symbols, 1500.0, RATE);
  if noise > 0.0 {
    add_noise(&mut signal, noise);
  }
  let wav = make_wav(&signal);
  std::fs::write(&out_path, &wav).expect("写入 WAV 失败");
  println!(
    "已生成 {out_path}：{call} {locator} {power}，{} 样本 ≈ {:.1}s，噪声幅度 {noise:.2}",
    signal.len(),
    signal.len() as f32 / RATE as f32
  );
}

fn add_noise(signal: &mut [f32], amp: f32) {
  let mut seed = 0x1234_5678u32;
  for s in signal {
    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    let u = (seed >> 8) as f32 / 16_777_216.0;
    *s += amp * (u * 2.0 - 1.0);
  }
}

/// 16 位 PCM 单声道 WAV。
fn make_wav(signal: &[f32]) -> Vec<u8> {
  let data_len = signal.len() * 2;
  let mut out = Vec::with_capacity(44 + data_len);
  out.extend_from_slice(b"RIFF");
  out.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
  out.extend_from_slice(b"WAVEfmt ");
  out.extend_from_slice(&16u32.to_le_bytes());
  out.extend_from_slice(&1u16.to_le_bytes());
  out.extend_from_slice(&1u16.to_le_bytes());
  out.extend_from_slice(&RATE.to_le_bytes());
  out.extend_from_slice(&(RATE * 2).to_le_bytes());
  out.extend_from_slice(&2u16.to_le_bytes());
  out.extend_from_slice(&16u16.to_le_bytes());
  out.extend_from_slice(b"data");
  out.extend_from_slice(&(data_len as u32).to_le_bytes());
  for &s in signal {
    out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
  }
  out
}
