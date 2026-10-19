//! 生成一段合成的 RTTY 音频样本（WAV），用于手动测试解码器。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-rtty --release --example generate_sample -- [输出.wav] [文本] [噪声]
//! ```
//! - 文本：默认 `CQ CQ DE BG1AAA 599 K`；
//! - 噪声幅度：默认 0.02；0 表示无噪声；
//! - 采样率 12000 Hz（每符号 264 样本），传号 2125 Hz。

use ham_web_rtty::{BAUD, SHIFT_HZ, encode_text};

const RATE: u32 = 12_000;
const MARK_HZ: f64 = 2125.0;

fn main() {
  let mut args = std::env::args().skip(1);
  let out_path = args.next().unwrap_or_else(|| "rtty-sample.wav".to_owned());
  let text = args
    .next()
    .unwrap_or_else(|| "CQ CQ DE BG1AAA 599 K".to_owned());
  let noise: f32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(0.02);

  let bits = encode_text(&text);
  let spb = (f64::from(RATE) / BAUD).round() as usize;
  let half_symbol = (spb as f64 / 2.0).round() as usize;
  let mut signal = Vec::new();
  let push = |samples: &mut Vec<f32>, f: f64, count: usize| {
    for _ in 0..count {
      let t = samples.len() as f64 / f64::from(RATE);
      samples.push((2.0 * std::f64::consts::PI * f * t).sin() as f32);
    }
  };
  let mut idx = 0usize;
  while idx + 7 <= bits.len() {
    for &b in &bits[idx..idx + 7] {
      push(
        &mut signal,
        if b { MARK_HZ } else { MARK_HZ + SHIFT_HZ },
        spb,
      );
    }
    push(&mut signal, MARK_HZ, half_symbol); // 1.5 停止位的余下 0.5
    idx += 7;
  }
  if noise > 0.0 {
    ham_web_dsp::add_noise(&mut signal, noise);
  }
  let wav = ham_web_wav::encode(&signal, RATE);
  std::fs::write(&out_path, &wav).expect("写入 WAV 失败");
  println!(
    "已生成 {out_path}：{text}，{} 样本 ≈ {:.1}s，噪声幅度 {noise:.2}",
    signal.len(),
    signal.len() as f32 / RATE as f32
  );
}
