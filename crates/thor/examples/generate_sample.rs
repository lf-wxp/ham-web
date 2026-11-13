//! 生成合成的 THOR 音频样本（WAV）—— 用**本 crate 的发端**（音调序列与 fldigi 上游逐位对拍过，
//! 音频还被 fldigi 自己的接收机解出过），供手动测试与 `thor-decode` 用。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-thor --release --example generate_sample -- [MODE] [输出.wav] [文本] [中心频率Hz] [采样率]
//! ```
//! MODE ∈ micro/4/5/8/11/16/22/25/32/44/56/25x4/50x1/50x2/100（默认 16）；文本默认 `CQ CQ DE BG1AAA 599`；
//! 中心默认 1500 Hz；采样率默认取该模式的标称采样率。

use ham_web_thor::{encode_audio, parse_mode};

fn main() {
  let mut args = std::env::args().skip(1);
  let mode_name = args.next().unwrap_or_else(|| "16".to_owned());
  let out_path = args.next().unwrap_or_else(|| "thor-sample.wav".to_owned());
  let text = args
    .next()
    .unwrap_or_else(|| "CQ CQ DE BG1AAA 599".to_owned());
  let center = args
    .next()
    .and_then(|c| c.parse::<f64>().ok())
    .unwrap_or(1500.0);

  let mode =
    parse_mode(&mode_name).expect("子模式（micro/4/5/8/11/16/22/25/32/44/56/25x4/50x1/50x2/100）");
  let rate = args
    .next()
    .and_then(|r| r.parse::<u32>().ok())
    .unwrap_or(mode.rate);
  let audio =
    encode_audio(&text, mode, rate, center).expect("编码（中心频率须让信号带落在 0–Nyquist 内）");
  let wav = ham_web_wav::encode(&audio, rate);
  std::fs::write(&out_path, &wav).expect("写入 WAV 失败");
  println!(
    "已生成 {out_path}：{text}（THOR {mode_name}，{:.2} 波特，中心 {center} Hz，{rate} Hz 采样）",
    mode.baud()
  );
}
