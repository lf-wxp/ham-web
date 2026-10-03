//! 命令行解码一个 WSPR WAV 文件并打印消息，用于验证解码器。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-wspr --release --example decode_sample -- <wav 路径> [基准频率Hz]
//! ```

use ham_web_wspr::decode_audio;

fn main() {
  let mut args = std::env::args().skip(1);
  let Some(path) = args.next() else {
    eprintln!("用法：decode_sample <wav 路径> [基准频率Hz]");
    std::process::exit(1);
  };
  let base: f64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1500.0);

  let bytes = std::fs::read(&path).expect("读取文件失败");
  let (samples, rate) = ham_web_apt::parse_wav(&bytes).expect("解析 WAV 失败");

  match decode_audio(&samples, base, rate) {
    Some(msg) => println!("解码结果：{msg}"),
    None => eprintln!("解码失败：未识别出有效消息"),
  }
}
