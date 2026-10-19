//! 生成一段合成的 Feld Hell 音频样本（WAV），用于手动测试解码器。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-hell --release --example generate_sample -- [输出.wav] [文本] [噪声]
//! ```
//! - 文本：默认 `CQ CQ DE BG1AAA`；
//! - 噪声幅度：默认 0.02；0 表示无噪声；
//! - 采样率 8000 Hz，980 Hz 载波、122.5 像素/秒（AM 点阵）。

use ham_web_hell::modulate;

const RATE: u32 = 8000;

fn main() {
  let mut args = std::env::args().skip(1);
  let out_path = args.next().unwrap_or_else(|| "hell-sample.wav".to_owned());
  let text = args.next().unwrap_or_else(|| "CQ CQ DE BG1AAA".to_owned());
  let noise: f32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(0.02);

  let mut signal = modulate(&text, RATE);
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
