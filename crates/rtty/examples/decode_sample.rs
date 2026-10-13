//! 命令行解码 RTTY 样本 WAV。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-rtty --release --example decode_sample -- [输入.wav] [传号频率Hz]
//! ```
//! - 传号频率默认 2125 Hz（空号 = 传号 + 170 Hz）。

use ham_web_rtty::decode_audio;

fn main() {
  let mut args = std::env::args().skip(1);
  let path = args.next().unwrap_or_else(|| "rtty-sample.wav".to_owned());
  let mark_hz: f64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(2125.0);

  let bytes = std::fs::read(&path).expect("读取 WAV 失败");
  let (samples, rate) = ham_web_apt::parse_wav(&bytes).expect("解析 WAV 失败");
  match decode_audio(&samples, rate, mark_hz) {
    Some(text) => println!("{text}"),
    None => eprintln!("解码失败：未识别出有效的 RTTY 信号"),
  }
}
