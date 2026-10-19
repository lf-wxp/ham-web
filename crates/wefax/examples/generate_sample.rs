//! 生成一段合成的 WEFAX 视频样本（WAV），用于手动测试解码器。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-wefax --release --example generate_sample -- [输出.wav]
//! ```
//! - 输出：默认 `wefax-sample.wav`；8000 Hz，16 行 × 380 像素的渐变图案。

use ham_web_wefax::{PIXELS_PER_LINE, WefaxImage, modulate};

const RATE: u32 = 8000;
const HEIGHT: usize = 16;

fn main() {
  let mut args = std::env::args().skip(1);
  let out_path = args.next().unwrap_or_else(|| "wefax-sample.wav".to_owned());

  let mut gray = Vec::with_capacity(PIXELS_PER_LINE * HEIGHT);
  for row in 0..HEIGHT {
    for col in 0..PIXELS_PER_LINE {
      // 横向渐变 + 按行递增的亮度带，肉眼可核对。
      let v =
        (((col as f32 / PIXELS_PER_LINE as f32) * 255.0) as u8).wrapping_add((row as u8) * 16);
      gray.push(v);
    }
  }
  let img = WefaxImage {
    width: PIXELS_PER_LINE,
    height: HEIGHT,
    gray,
  };
  let samples = modulate(&img).expect("图案尺寸与 PIXELS_PER_LINE 一致");
  let wav = ham_web_wav::encode(&samples, RATE);
  std::fs::write(&out_path, &wav).expect("写入 WAV 失败");
  println!(
    "已生成 {out_path}：{PIXELS_PER_LINE}×{HEIGHT}，{} 样本 ≈ {:.1}s",
    samples.len(),
    samples.len() as f32 / RATE as f32
  );
}
