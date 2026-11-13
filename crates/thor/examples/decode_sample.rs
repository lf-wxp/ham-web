//! 命令行解码 THOR 录音（WAV）。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-thor --release --example decode_sample -- [MODE] [输入.wav] [中心频率Hz|auto] [reverse]
//! ```
//! MODE ∈ micro/4/5/8/11/16/22/25/32/44/56/25x4/50x1/50x2/100（默认 16）。

use ham_web_thor::{decode_audio, parse_mode};

fn main() {
  let mut args = std::env::args().skip(1);
  let mode_name = args.next().unwrap_or_else(|| "16".to_owned());
  let path = args
    .next()
    .unwrap_or_else(|| "tmp/thor-sample.wav".to_owned());
  let center = args.next().and_then(|c| c.parse::<f64>().ok());
  let reverse = args.next().is_some_and(|r| r == "reverse");

  let mode =
    parse_mode(&mode_name).expect("子模式（micro/4/5/8/11/16/22/25/32/44/56/25x4/50x1/50x2/100）");
  let bytes = std::fs::read(&path).expect("读取 WAV 失败");
  let (samples, rate) = ham_web_wav::parse(&bytes).expect("解析 WAV 失败");
  match decode_audio(&samples, rate, mode, center, reverse) {
    Some(d) => {
      println!(
        "THOR {mode_name} · 中心 {:.1} Hz · {} 个符号",
        d.center_hz, d.symbols
      );
      println!("{}", d.text);
      if !d.id.is_empty() {
        println!("二次信道（ID 信标）：{}", d.id);
      }
    }
    None => {
      eprintln!("没有可解调的信号");
      std::process::exit(1);
    }
  }
}
