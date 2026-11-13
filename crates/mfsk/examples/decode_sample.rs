//! 命令行解码 MFSK 录音（WAV）。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-mfsk --release --example decode_sample -- [MODE] [输入.wav] [中心频率Hz|auto] [reverse]
//! ```
//! MODE ∈ 4/8/11/16/22/31/32/64/128/64L/128L（默认 16）。

use ham_web_mfsk::{decode_audio, parse_mode};

fn main() {
  let mut args = std::env::args().skip(1);
  let mode_name = args.next().unwrap_or_else(|| "16".to_owned());
  let path = args
    .next()
    .unwrap_or_else(|| "tmp/mfsk-sample.wav".to_owned());
  let center = args.next().and_then(|c| c.parse::<f64>().ok());
  let reverse = args.next().is_some_and(|r| r == "reverse");

  let mode = parse_mode(&mode_name).expect("子模式（4/8/11/16/22/31/32/64/128/64L/128L）");
  let bytes = std::fs::read(&path).expect("读取 WAV 失败");
  let (samples, rate) = ham_web_wav::parse(&bytes).expect("解析 WAV 失败");
  match decode_audio(&samples, rate, mode, center, reverse) {
    Some(d) => {
      println!(
        "MFSK{mode_name} · 中心 {:.1} Hz · {} 个符号 · 交织深度 {}",
        d.center_hz, d.symbols, d.depth
      );
      println!("{}", d.text);
    }
    None => {
      eprintln!("没有可解调的信号");
      std::process::exit(1);
    }
  }
}
