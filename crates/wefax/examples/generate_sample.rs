//! 生成一段合成的 WEFAX 视频样本（WAV），用于手动测试解码器。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-wefax --release --example generate_sample -- [输出.wav] [--fm | --fax]
//! ```
//! - 输出：默认 `wefax-sample.wav`；8000 Hz。
//! - 默认是简化模型的调幅视频（16 行 × 380 像素的渐变图案）；`--fm` 是简化模型的调频副载波；
//!   `--fax` 是**真实格式**（IOC 576、120 LPM、起始音 + 20 条相位线 + 40 行 × 1810 像素的图案）。

use ham_web_wefax::fax::{self, Ioc, ModulateOptions, Params};
use ham_web_wefax::{PIXELS_PER_LINE, WefaxImage, modulate, modulate_fm};

const RATE: u32 = 8000;
const HEIGHT: usize = 16;

fn main() {
  let args: Vec<String> = std::env::args().skip(1).collect();
  let fm = args.iter().any(|a| a == "--fm");
  if args.iter().any(|a| a == "--fax") {
    return generate_fax(args.iter().find(|a| !a.starts_with("--")));
  }
  let out_path = args
    .iter()
    .find(|a| !a.starts_with("--"))
    .cloned()
    .unwrap_or_else(|| "wefax-sample.wav".to_owned());

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
  let samples = if fm {
    modulate_fm(&img)
  } else {
    modulate(&img)
  }
  .expect("图案尺寸与 PIXELS_PER_LINE 一致");
  let wav = ham_web_wav::encode(&samples, RATE);
  std::fs::write(&out_path, &wav).expect("写入 WAV 失败");
  println!(
    "已生成 {out_path}：{PIXELS_PER_LINE}×{HEIGHT}，{} 样本 ≈ {:.1}s",
    samples.len(),
    samples.len() as f32 / RATE as f32
  );
}

/// 真实格式：渐变底、一块黑、一块白、每 150 列一条细竖线，加 20 条相位线。
fn generate_fax(out_path: Option<&String>) {
  let out_path = out_path.map_or("wefax-fax-sample.wav", String::as_str);
  let ioc = Ioc::I576;
  let (width, height) = (ioc.width(), 40);
  let mut gray = Vec::with_capacity(width * height);
  for row in 0..height {
    for col in 0..width {
      let x = col as f64 / width as f64;
      gray.push(
        if ((8..16).contains(&row) && (0.3..0.5).contains(&x)) || col % 150 < 3 {
          255
        } else if (22..28).contains(&row) && (0.6..0.8).contains(&x) {
          0
        } else {
          (x * 170.0) as u8 + 20
        },
      );
    }
  }
  let image = WefaxImage {
    width,
    height,
    gray,
  };
  let options = ModulateOptions::new(Params::default(), RATE);
  let samples = fax::modulate(&image, &options).expect("图案尺寸与 IOC 一致");
  std::fs::write(out_path, ham_web_wav::encode(&samples, RATE)).expect("写入 WAV 失败");
  println!(
    "已生成 {out_path}：真实格式 {width}×{height}，{} 样本 ≈ {:.1}s",
    samples.len(),
    samples.len() as f32 / RATE as f32
  );
}
