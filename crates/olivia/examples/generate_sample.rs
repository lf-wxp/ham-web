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
  let wav = make_wav(&audio);
  std::fs::write(&out_path, &wav).expect("写入 WAV 失败");
  println!("已生成 {out_path}：{text}（{symbols} 音调 / {bandwidth} Hz @ {center} Hz）");
}

fn make_wav(signal: &[f32]) -> Vec<u8> {
  let data_len = signal.len() * 2;
  let mut out = Vec::with_capacity(44 + data_len);
  out.extend_from_slice(b"RIFF");
  out.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
  out.extend_from_slice(b"WAVEfmt ");
  out.extend_from_slice(&16u32.to_le_bytes());
  out.extend_from_slice(&1u16.to_le_bytes());
  out.extend_from_slice(&1u16.to_le_bytes());
  out.extend_from_slice(&8000u32.to_le_bytes());
  out.extend_from_slice(&16000u32.to_le_bytes());
  out.extend_from_slice(&2u16.to_le_bytes());
  out.extend_from_slice(&16u16.to_le_bytes());
  out.extend_from_slice(b"data");
  out.extend_from_slice(&(data_len as u32).to_le_bytes());
  for &s in signal {
    out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
  }
  out
}
