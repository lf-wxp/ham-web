//! 生成一段合成的 NAVTEX 音频样本（WAV），用于手动测试解码器。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-navtex --release --example generate_sample -- [输出.wav] [文本] [噪声]
//! ```
//! - 文本：默认 `GALE WARNING IN SOUTHERN ADRIATIC 25 KNOTS`；
//! - 噪声幅度：默认 0.02；0 表示无噪声。

use ham_web_navtex::synthesize;

fn main() {
  let mut args = std::env::args().skip(1);
  let out_path = args
    .next()
    .unwrap_or_else(|| "navtex-sample.wav".to_owned());
  let text = args
    .next()
    .unwrap_or_else(|| "GALE WARNING IN SOUTHERN ADRIATIC 25 KNOTS".to_owned());
  let noise: f32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(0.02);

  let signal = synthesize(&text, 3.0, noise);
  let wav = make_wav(&signal, 8000);
  std::fs::write(&out_path, &wav).expect("写入 WAV 失败");
  println!(
    "已生成 {out_path}：{text}，{} 样本 ≈ {:.1}s，噪声幅度 {noise:.2}",
    signal.len(),
    signal.len() as f32 / 8000.0
  );
}

/// 16 位 PCM 单声道 WAV。
fn make_wav(signal: &[f32], rate: u32) -> Vec<u8> {
  let data_len = signal.len() * 2;
  let mut out = Vec::with_capacity(44 + data_len);
  out.extend_from_slice(b"RIFF");
  out.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
  out.extend_from_slice(b"WAVEfmt ");
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
