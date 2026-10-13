//! 命令行解码 Olivia 样本 WAV。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-olivia --release --example decode_sample -- [MODE] [输入.wav]
//! ```

use ham_web_olivia::{decode_audio, parse_submode};

fn main() {
  let mut args = std::env::args().skip(1);
  let mode = args.next().unwrap_or_else(|| "32/1000@1500".to_owned());
  let path = args
    .next()
    .unwrap_or_else(|| "olivia-sample.wav".to_owned());

  let (symbols, bandwidth, center) = parse_submode(&mode).expect("子模式格式（如 32/1000@1500）");
  let bytes = std::fs::read(&path).expect("读取 WAV 失败");
  let (samples, rate) = ham_web_apt::parse_wav(&bytes).expect("解析 WAV 失败");
  match decode_audio(&samples, rate, symbols, bandwidth, center) {
    Some(text) if !text.is_empty() => println!("{text}"),
    _ => eprintln!("解码失败：没有解出文本"),
  }
}
