//! 生成合成的 MFSK 音频样本（WAV）—— 用**本 crate 的发端**（音调序列与 fldigi 上游逐位对拍过），
//! 供手动测试与 `mfsk-decode` 用。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-mfsk --release --example generate_sample -- [MODE] [输出.wav] [文本] [中心频率Hz]
//! ```
//! MODE ∈ 4/8/11/16/22/31/32/64/128（默认 16）；文本默认 `CQ CQ DE BG1AAA 599`；中心默认 1000 Hz。

use ham_web_mfsk::{encode_audio, parse_mode};

fn main() {
  let mut args = std::env::args().skip(1);
  let mode_name = args.next().unwrap_or_else(|| "16".to_owned());
  let out_path = args.next().unwrap_or_else(|| "mfsk-sample.wav".to_owned());
  let text = args
    .next()
    .unwrap_or_else(|| "CQ CQ DE BG1AAA 599".to_owned());
  let center = args
    .next()
    .and_then(|c| c.parse::<f64>().ok())
    .unwrap_or(1000.0);

  let mode = parse_mode(&mode_name).expect("子模式（4/8/11/16/22/31/32/64/128）");
  let audio =
    encode_audio(&text, mode, 8000, center).expect("编码（中心频率须让信号带落在 0–4000 Hz 内）");
  let wav = ham_web_wav::encode(&audio, 8000);
  std::fs::write(&out_path, &wav).expect("写入 WAV 失败");
  println!(
    "已生成 {out_path}：{text}（MFSK{mode_name}，{:.2} 波特，中心 {center} Hz）",
    mode.baud()
  );
}
