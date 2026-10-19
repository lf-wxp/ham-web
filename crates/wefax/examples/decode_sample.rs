//! 命令行解码 WEFAX 样本 WAV。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-wefax --release --example decode_sample -- [输入.wav]
//! ```

use ham_web_wefax::decode;

fn main() {
  let mut args = std::env::args().skip(1);
  let path = args.next().unwrap_or_else(|| "wefax-sample.wav".to_owned());

  let bytes = std::fs::read(&path).expect("读取 WAV 失败");
  match decode(&bytes) {
    Ok(img) => println!("{}×{}", img.width, img.height),
    Err(e) => eprintln!("解码失败：{e}"),
  }
}
