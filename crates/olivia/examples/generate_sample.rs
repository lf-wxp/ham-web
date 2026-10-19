//! 生成合成的 Olivia 音频样本（WAV）—— 用**本 crate 的编码器**（与 Python 参考
//! 实现互锁验证过），供手动测试与 `olivia-decode` 用。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-olivia --release --example generate_sample -- [MODE] [输出.wav] [文本]
//! ```
//! MODE 默认 `32/1000@1500`；文本默认 `CQ CQ DE BG1AAA 599`。

use ham_web_olivia::{encode_audio, parse_submode};

fn main() {
  let mut args = std::env::args().skip(1);
  let mode = args.next().unwrap_or_else(|| "32/1000@1500".to_owned());
  let out_path = args
    .next()
    .unwrap_or_else(|| "olivia-sample.wav".to_owned());
  let text = args
    .next()
    .unwrap_or_else(|| "CQ CQ DE BG1AAA 599".to_owned());

  let (symbols, bandwidth, center) = parse_submode(&mode).expect("子模式格式（如 32/1000@1500）");
  let audio = encode_audio(&text, symbols, bandwidth, center, 8000).expect("不支持的子模式");
  let wav = ham_web_wav::encode(&audio, 8000);
  std::fs::write(&out_path, &wav).expect("写入 WAV 失败");
  println!("已生成 {out_path}：{text}（{symbols} 音调 / {bandwidth} Hz @ {center} Hz）");
}
