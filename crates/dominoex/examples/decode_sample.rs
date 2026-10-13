//! 命令行解码 DominoEX 样本 WAV。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-dominoex --release --example decode_sample -- [MODE] [输入.wav]
//! ```

use ham_web_dominoex::{decode_audio, parse_mode};

fn main() {
  let mut args = std::env::args().skip(1);
  let mode = args.next().unwrap_or_else(|| "8".to_owned());
  let path = args
    .next()
    .unwrap_or_else(|| "dominoex-sample.wav".to_owned());

  let (baud, spacing) = parse_mode(&mode).expect("子模式（4/5/8/11/16/22）");
  let bytes = std::fs::read(&path).expect("读取 WAV 失败");
  let (samples, rate) = ham_web_apt::parse_wav(&bytes).expect("解析 WAV 失败");
  match decode_audio(&samples, rate, baud, spacing) {
    Some((text, id)) if !text.is_empty() => {
      println!("{text}");
      if !id.is_empty() {
        println!("[ID] {id}");
      }
    }
    _ => eprintln!("解码失败：没有解出文本"),
  }
}
