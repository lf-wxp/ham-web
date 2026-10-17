//! 生成一段合成的 FT8 音频样本（WAV），用于手动测试解码器。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-ft8 --release --example generate_sample -- [输出.wav] [消息] [噪声幅度]
//! ```
//! - 消息：默认 `CQ K7IHZ DM43`；
//! - 噪声幅度：默认 0.02，加在 8-FSK 信号上，模拟真实接收的轻微噪声；0 表示无噪声。
//!
//! 采样率 12000 Hz、时长 15 秒（FT8 一个时隙），基频 1000 Hz。

use ham_web_ft8::{HashTable, synthesize};

const RATE: u32 = 12_000;

fn main() {
  let mut args = std::env::args().skip(1);
  let out_path = args.next().unwrap_or_else(|| "ft8-sample.wav".to_owned());
  let message = args.next().unwrap_or_else(|| "CQ K7IHZ DM43".to_owned());
  let noise: f32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(0.02);

  let mut hash = HashTable::new();
  let Some(signal) = synthesize(&message, 1000.0, false, &mut hash) else {
    eprintln!("无法编码消息：{message}");
    std::process::exit(1);
  };
  let mut signal = signal;
  if noise > 0.0 {
    add_noise(&mut signal, noise);
  }
  let wav = make_wav(&signal);
  std::fs::write(&out_path, &wav).expect("写入 WAV 失败");
  println!(
    "已生成 {out_path}：{message}，{} 样本 ≈ {:.1}s，噪声幅度 {noise:.2}",
    signal.len(),
    signal.len() as f32 / RATE as f32
  );
}

/// 叠加均匀白噪声（幅度 `amp`，峰峰值 2·amp）。
fn add_noise(signal: &mut [f32], amp: f32) {
  let mut seed = 0x1234_5678u32;
  for s in signal {
    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    let u = (seed >> 8) as f32 / 16_777_216.0; // 0..1
    *s += amp * (u * 2.0 - 1.0);
  }
}

/// 写为 16 位 PCM 单声道 WAV（采样率 12000 Hz）。
fn make_wav(signal: &[f32]) -> Vec<u8> {
  let data_len = signal.len() * 2;
  let mut out = Vec::with_capacity(44 + data_len);
  out.extend_from_slice(b"RIFF");
  out.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
  out.extend_from_slice(b"WAVE");
  out.extend_from_slice(b"fmt ");
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
