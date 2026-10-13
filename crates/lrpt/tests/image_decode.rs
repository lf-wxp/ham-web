//! 帧层之上：RS → VCDU/MPDU → MSU-MR 段 → JPEG → **图像**的互操作对拍。
//!
//! 夹具（`samples/`，均由 lrpt-encoder（MIT）生成，真值随附）：
//! - `bands_cadus.bin`：1568 宽、64 高的纯色带图（`truth_bands.png`）经 `--rgb` 编码的
//!   18 帧 CADU 流 —— 颜色带为纯色、编码器 resize 恒等（已是目标尺寸）、无
//!   luma 转换歧义 → **逐像素真值**（`truth_bands.bin` = R/G/B 三平面）；
//! - `lrpt_encoder_cadus.bin`：`a.png`（554×554）经 Lanczos3 放大到 1568×1568 +
//!   luma（编码器内部处理，无独立真值文件）→ 只做结构断言（尺寸、R=G=B、
//!   内容非退化），如实说明。

use ham_web_lrpt::prand::PRAND;
use ham_web_lrpt::{IMAGE_WIDTH, decode_image};

fn read_bytes(path: &str) -> Vec<u8> {
  std::fs::read(path).unwrap_or_else(|e| panic!("读取 {path} 失败：{e}"))
}

/// 原始 CADU 流（[ASM | RS+扰码的 1020 字节]）→ 去扰码后的 RS 帧。
fn cadu_frames(bytes: &[u8]) -> Vec<[u8; 1020]> {
  assert_eq!(bytes.len() % 1024, 0, "CADU 流长度应帧对齐");
  bytes
    .chunks(1024)
    .filter(|c| c.len() == 1024)
    .map(|c| {
      let mut d = [0u8; 1020];
      d.copy_from_slice(&c[4..]);
      for (i, b) in d.iter_mut().enumerate() {
        *b ^= PRAND[i % PRAND.len()];
      }
      d
    })
    .collect()
}

#[test]
fn decodes_the_bands_image_pixel_exact() {
  let frames = cadu_frames(&read_bytes(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/samples/bands_cadus.bin"
  )));
  assert_eq!(frames.len(), 18);
  let image = decode_image(&frames).expect("整链解码");
  assert_eq!(image.width, IMAGE_WIDTH);
  assert!(
    image.height >= 24,
    "8 MCU 行（64px）的首尾残缺行丢弃后至少 3 行（24px），得到 {}",
    image.height
  );

  // 首尾残缺行被丢弃，比对从 image.first_row_in_recording 对应的真值行开始
  // （夹具从图像第 0 行开始录，所以该相对行号就是真值行号）。
  let truth = read_bytes(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/samples/truth_bands.bin"
  ));
  assert_eq!(truth.len(), 3 * IMAGE_WIDTH * 64);
  for ch in 0..3 {
    let plane = &image.channels[ch];
    assert_eq!(plane.len(), image.height, "通道 {ch} 行数");
    for (y, row) in plane.iter().enumerate() {
      let ty = image.first_row_in_recording * 8 + y;
      let expect = &truth[ch * IMAGE_WIDTH * 64 + ty * IMAGE_WIDTH..][..IMAGE_WIDTH];
      assert_eq!(
        row,
        &expect.to_vec(),
        "通道 {ch} 第 {y} 行（真值第 {ty} 行）逐像素一致"
      );
    }
  }
}

#[test]
fn a_dropped_frame_does_not_lose_the_whole_recording() {
  // 真实过境录音丢帧是常态：RS 译不出的帧跳过、帧计数出现空洞时仍应解出图像，
  // 而不是整段返回「图像不完整」。
  let frames = cadu_frames(&read_bytes(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/samples/bands_cadus.bin"
  )));
  let mut with_hole = frames.clone();
  with_hole.remove(9);
  let image = decode_image(&with_hole).expect("丢一帧后仍应解出图像");
  assert!(
    image.height >= 8,
    "至少保留一行完整 MCU 行，得到 {}",
    image.height
  );
}

#[test]
fn a_corrupted_frame_is_skipped_instead_of_failing_the_chain() {
  // 交错把连续 4 字节分到 4 个码字里：破坏 68 字节 = 每个码字 17 个符号错，
  // 超出 RS(255,223) 的 16 符号纠正能力 → 该帧必然译不出，必须被跳过。
  let frames = cadu_frames(&read_bytes(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/samples/bands_cadus.bin"
  )));
  let mut corrupted = frames.clone();
  for b in &mut corrupted[8][..68] {
    *b ^= 0xFF;
  }
  let image = decode_image(&corrupted).expect("坏帧跳过而不是整段失败");
  assert!(image.height >= 8, "得到 {}", image.height);
}

#[test]
fn decodes_the_full_a_png_image_structurally() {
  let frames = cadu_frames(&read_bytes(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/samples/lrpt_encoder_cadus.bin"
  )));
  assert!(frames.len() >= 800, "全量 CADU 流应有 800+ 帧");
  let image = decode_image(&frames).expect("整链解码");
  assert_eq!(image.width, IMAGE_WIDTH);
  assert_eq!(image.height, 1568, "a.png 放大到 1568×1568");
  // 编码器默认模式：ch2 = ch3 = ch1（同一灰度平面）→ R = G = B。
  assert_eq!(image.channels[0], image.channels[1], "R = G");
  assert_eq!(image.channels[0], image.channels[2], "R = B");
  // 内容非退化：方差 > 0（不是纯色）。
  let mut mean = 0f64;
  let mut var = 0f64;
  let n = (image.height * image.width) as f64;
  for row in &image.channels[0] {
    for &p in row {
      mean += f64::from(p);
    }
  }
  mean /= n;
  for row in &image.channels[0] {
    for &p in row {
      let d = f64::from(p) - mean;
      var += d * d;
    }
  }
  var /= n;
  assert!(var > 100.0, "图像内容应有结构（方差 {var:.0}）");
}
