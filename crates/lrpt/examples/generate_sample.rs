//! 生成合成的 LRPT 基带样本：**288 kHz 立体声 WAV（左 I、右 Q）**，页面能解出**图像**。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-lrpt --release --example generate_sample -- [输出.wav]
//! ```
//! 帧内容取自仓库内的真实图像帧夹具 `samples/bands_cadus.bin`（lrpt-encoder 对色带图编码的
//! 18 帧 CADU，含 MSU-MR 图像段），去扰码后经 [`ham_web_lrpt::encode_stream288`]
//! （NRZ-M → 连续卷积 → RRC 成形 OQPSK，72k 符号/s × 4 采样）调制。
//!
//! 为什么不用伪随机数据：伪数据能通过帧层（RS），但不是图像段，页面会一直报
//! `image-incomplete`（解码失败：帧层通过但图像层不完整）。

use ham_web_lrpt::prand::PRAND;
use ham_web_lrpt::{DATA_LEN, decode_image, decode_stream288, encode_stream288};

/// LRPT 基带标称采样率（72k 符号/s × 4 采样/符号）。
const SAMPLE_RATE: u32 = 288_000;

/// 色带图的 CADU 流（每帧 `[ASM 4 字节 | 扰码后的 1020 字节]`）。
const CADUS: &[u8] = include_bytes!("../samples/bands_cadus.bin");

fn main() {
  let out_path = std::env::args()
    .nth(1)
    .unwrap_or_else(|| "lrpt-sample.wav".to_owned());

  let (cadus, _) = CADUS.as_chunks::<1024>();
  let frames: Vec<[u8; DATA_LEN]> = cadus
    .iter()
    .map(|c| {
      let mut d = [0u8; DATA_LEN];
      d.copy_from_slice(&c[4..]);
      for (i, b) in d.iter_mut().enumerate() {
        *b ^= PRAND[i % PRAND.len()];
      }
      d
    })
    .collect();
  let interleaved = encode_stream288(&frames);

  // 自检：生成的信号必须能被同一条接收链还原出图像，否则不写文件（别产出一个页面解不了的样本）。
  let decoded = decode_stream288(&interleaved);
  match decode_image(&decoded) {
    Some(img) => println!(
      "自检通过：解出 {} 帧，图像 {} × {}",
      decoded.len(),
      img.width,
      img.height
    ),
    None => {
      eprintln!(
        "自检失败：解出 {} 帧但拼不出图像，未写入文件",
        decoded.len()
      );
      std::process::exit(1);
    }
  }

  let wav = make_stereo_wav(&interleaved);
  std::fs::write(&out_path, &wav).expect("写入 WAV 失败");
  println!(
    "已生成 {out_path}：{} 帧，{:.2}s @ {SAMPLE_RATE} Hz 立体声（左 I / 右 Q）",
    frames.len(),
    interleaved.len() as f64 / 2.0 / f64::from(SAMPLE_RATE),
  );
}

/// 16 位 PCM 立体声 WAV（左 I / 右 Q）。
fn make_stereo_wav(samples: &[f32]) -> Vec<u8> {
  let data_len = samples.len() * 2;
  let mut out = Vec::with_capacity(44 + data_len);
  out.extend_from_slice(b"RIFF");
  out.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
  out.extend_from_slice(b"WAVEfmt ");
  out.extend_from_slice(&16u32.to_le_bytes());
  out.extend_from_slice(&1u16.to_le_bytes());
  out.extend_from_slice(&2u16.to_le_bytes()); // 立体声
  out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
  out.extend_from_slice(&(SAMPLE_RATE * 4).to_le_bytes());
  out.extend_from_slice(&4u16.to_le_bytes());
  out.extend_from_slice(&16u16.to_le_bytes());
  out.extend_from_slice(b"data");
  out.extend_from_slice(&(data_len as u32).to_le_bytes());
  for &s in samples {
    out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
  }
  out
}
