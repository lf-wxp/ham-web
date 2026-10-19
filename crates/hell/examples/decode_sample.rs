//! 命令行解码 Feld Hell 样本 WAV。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-hell --release --example decode_sample -- [输入.wav]
//! ```

use ham_web_hell::decode_audio;

fn main() {
  let mut args = std::env::args().skip(1);
  let path = args.next().unwrap_or_else(|| "hell-sample.wav".to_owned());

  let bytes = std::fs::read(&path).expect("读取 WAV 失败");
  let (samples, rate) = ham_web_apt::parse_wav(&bytes).expect("解析 WAV 失败");
  match decode_audio(&samples, rate) {
    Some(text) => println!("{text}"),
    None => eprintln!("解码失败：未识别出有效的 Feld Hell 信号"),
  }
}
