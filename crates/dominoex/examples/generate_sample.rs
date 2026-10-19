//! 生成合成的 DominoEX 音频样本（WAV）—— 用**本 crate 的编码器**（与纯 Python
//! 参考实现互锁验证过），供手动测试与 `dominoex-decode` 用。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-dominoex --release --example generate_sample -- [MODE] [输出.wav] [文本]
//! ```
//! MODE ∈ 4/5/8/11/16/22（默认 8）；文本默认 `CQ CQ DE BG1AAA 599`。

use ham_web_dominoex::{encode_audio, parse_mode};

fn main() {
  let mut args = std::env::args().skip(1);
  let mode = args.next().unwrap_or_else(|| "8".to_owned());
  let out_path = args
    .next()
    .unwrap_or_else(|| "dominoex-sample.wav".to_owned());
  let text = args
    .next()
    .unwrap_or_else(|| "CQ CQ DE BG1AAA 599".to_owned());

  let (baud, spacing) = parse_mode(&mode).expect("子模式（4/5/8/11/16/22）");
  let audio = encode_audio(&text, baud, spacing, 8000, 0.5).expect("编码");
  let wav = ham_web_wav::encode(&audio, 8000);
  std::fs::write(&out_path, &wav).expect("写入 WAV 失败");
  println!(
    "已生成 {out_path}：{text}（DominoEX {mode}，{:.0} 波特）",
    baud
  );
}
