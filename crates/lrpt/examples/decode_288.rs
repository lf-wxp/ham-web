//! 解码 288 kHz 立体声 WAV（左 I / 右 Q）—— 供真实样本（lrpt-encoder 输出
//! 与将来 SatNOGS 过境录音）使用。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-lrpt --release --example decode_288 -- [输入.wav]
//! ```

use ham_web_lrpt::decode_stream288;

fn main() {
  let mut args = std::env::args().skip(1);
  let path = args
    .next()
    .unwrap_or_else(|| "samples/lrpt_encoder_head.wav".to_owned());
  let bytes = match std::fs::read(&path) {
    Ok(b) => b,
    Err(e) => {
      eprintln!("读取 {path} 失败：{e}");
      std::process::exit(1);
    }
  };
  let iq = wav_to_iq(&bytes);
  let frames = decode_stream288(&iq);
  if frames.is_empty() {
    eprintln!("解码失败：没有解出任何 CADU 帧");
    return;
  }
  println!("解出 {} 帧（每帧 1020 字节 RS 码字）", frames.len());
  println!("首帧前 16 字节：{}", hex(&frames[0][..16]));
}

fn hex(bytes: &[u8]) -> String {
  bytes
    .iter()
    .map(|b| format!("{b:02X}"))
    .collect::<Vec<_>>()
    .join(" ")
}

fn wav_to_iq(bytes: &[u8]) -> Vec<f32> {
  // 用带校验的解析器（chunk 越界、奇数字节、非 PCM 都返回错误，而不是索引 panic）。
  match ham_web_apt::parse_wav_interleaved(bytes) {
    Ok((samples, _rate, _channels)) => samples,
    Err(e) => {
      eprintln!("WAV 解析失败：{}", e.code());
      std::process::exit(1);
    }
  }
}
