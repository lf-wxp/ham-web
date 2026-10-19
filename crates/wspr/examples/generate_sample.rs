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
    ham_web_dsp::add_noise(&mut signal, noise);
  }
  let wav = ham_web_wav::encode(&signal, RATE);
  std::fs::write(&out_path, &wav).expect("写入 WAV 失败");
  println!(
    "已生成 {out_path}：{call} {locator} {power}，{} 样本 ≈ {:.1}s，噪声幅度 {noise:.2}",
    signal.len(),
    signal.len() as f32 / RATE as f32
  );
}
