//! 命令行解码 LRPT 基带样本（288 kHz 立体声 WAV：左 I / 右 Q），走与页面 Worker 相同的接收链。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-lrpt --release --example decode_sample -- [输入.wav]
//! ```
//! 输出帧层结果（解出多少帧、其中 RS 校验通过多少）与图像层结果。

use ham_web_lrpt::{decode_image, decode_stream288, rs};

fn main() {
  let mut args = std::env::args().skip(1);
  let path = args.next().unwrap_or_else(|| "lrpt-sample.wav".to_owned());

  let bytes = match std::fs::read(&path) {
    Ok(b) => b,
    Err(e) => {
      eprintln!("读取 {path} 失败：{e}");
      std::process::exit(1);
    }
  };
  // 带校验的解析器：chunk 越界 / 奇数字节 / 非 PCM 返回错误而不是索引 panic。
  let (iq, rate, channels) = match ham_web_apt::parse_wav_interleaved(&bytes) {
    Ok(v) => v,
    Err(e) => {
      eprintln!("WAV 解析失败：{}", e.code());
      std::process::exit(1);
    }
  };
  if rate != 288_000 || channels != 2 {
    eprintln!("需要 288 kHz 立体声 I/Q WAV（左 I 右 Q），当前是 {rate} Hz / {channels} 声道");
    std::process::exit(1);
  }
  let frames = decode_stream288(&iq);
  if frames.is_empty() {
    eprintln!("解码失败：没有解出任何帧");
    return;
  }
  let rs_ok = frames
    .iter()
    .filter(|f| rs::decode_frame(f).is_some())
    .count();
  println!("解出 {} 帧，其中 RS 校验通过 {rs_ok} 帧", frames.len());
  match decode_image(&frames) {
    Some(img) => println!("图像层：{} × {}", img.width, img.height),
    None => println!("图像层：没有可拼出的图像（伪数据样本属正常；真实图像见 samples/bands.wav）"),
  }
}
