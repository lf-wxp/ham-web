//! 命令行解码 NAVTEX 样本 WAV。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-navtex --release --example decode_sample -- [输入.wav]
//! ```

use ham_web_navtex::decode_audio;

fn main() {
  let mut args = std::env::args().skip(1);
  let path = args
    .next()
    .unwrap_or_else(|| "navtex-sample.wav".to_owned());

  let bytes = std::fs::read(&path).expect("读取 WAV 失败");
  let (samples, rate) = ham_web_apt::parse_wav(&bytes).expect("解析 WAV 失败");
  match decode_audio(&samples, rate) {
    Some(messages) if !messages.is_empty() => {
      for m in messages {
        println!(
          "{} {} {:02} [{}] {}{}",
          m.transmitter,
          m.category,
          m.serial,
          if m.accepted { "收" } else { "拒" },
          m.text,
          if m.corrected > 0 {
            format!("（修正 {} 处）", m.corrected)
          } else {
            String::new()
          }
        );
      }
    }
    _ => eprintln!("解码失败：没有解出完整的 NAVTEX 报文"),
  }
}
