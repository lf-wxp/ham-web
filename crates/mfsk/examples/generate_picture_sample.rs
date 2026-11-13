//! 生成合成的 MFSK 图片模式音频样本（WAV）—— 用**本 crate 的发端**（`Pic:宽x高[增补];` 报头 + 序言 + FM 图像），
//! 供手动测试与 `decode_picture` 用；旁边的 `<输出.wav>.rgb` 是想发的原图（`decode_picture` 会自动读它并报逐字节误差）。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-mfsk --release --example generate_picture_sample -- [MODE] [输出.wav] [宽] [高] [color|gray] [图案] [中心频率Hz] [每像素样本数]
//! ```
//! MODE ∈ 16/32/64/128（默认 16：只有这几个子模式有图片模式）；默认 32 × 24 彩色 `bands`、中心 1000 Hz、每像素 8 个样本。
//! 图案：`bands` —— 四块纯色；`ramp` —— R 随 x、G 随 y 线性变化，B 是 8 像素的棋盘格。两者与
//! `reference/fldigi_web/gen_pic.mjs` 的同名图案一致，本仓生成的样本可以和 fldigi 的样本直接对照。
//! 文本固定发 `CQ ` 前缀：报头紧随其后，解码结果里会原样带出来（`CQ Pic:32x24C;`）。

use ham_web_mfsk::picture::{Header, MAX_PIXELS};
use ham_web_mfsk::{encode_picture_audio, parse_mode};

fn main() {
  let mut args = std::env::args().skip(1);
  let mode_name = args.next().unwrap_or_else(|| "16".to_owned());
  let out_path = args
    .next()
    .unwrap_or_else(|| "mfsk-picture-sample.wav".to_owned());
  let width = next_or(&mut args, 32usize);
  let height = next_or(&mut args, 24usize);
  let kind = args.next().unwrap_or_else(|| "color".to_owned());
  let pattern = args.next().unwrap_or_else(|| "bands".to_owned());
  let center = next_or(&mut args, 1000.0f64);
  let spp = next_or(&mut args, 8usize);

  assert!(
    kind == "color" || kind == "gray",
    "色彩只有 color / gray：{kind}"
  );
  let gray = kind == "gray";
  // 先按宽 × 高拦一次：下面要按它分配 `宽 × 高 × 3` 字节，报头那边的同类检查在分配之后。
  let pixels = width
    .checked_mul(height)
    .filter(|&n| n > 0 && n <= MAX_PIXELS)
    .unwrap_or_else(|| panic!("尺寸按宽 × 高算像素总数，须在 1–{MAX_PIXELS} 之间"));
  let mut rgb = pattern_rgb(width, height, &pattern)
    .unwrap_or_else(|| panic!("图案只有 bands / ramp：{pattern}"));
  assert_eq!(rgb.len(), pixels * 3);

  if gray {
    // 与 fldigi 的灰度公式 31 / 61 / 8 一致（发端按 R 通道发，三个通道必须先相同）。
    for px in rgb.as_chunks_mut::<3>().0.iter_mut() {
      let g = ((31 * u32::from(px[0]) + 61 * u32::from(px[1]) + 8 * u32::from(px[2])) / 100) as u8;
      px.fill(g);
    }
  }

  let mode = parse_mode(&mode_name).expect("子模式（16/32/64/128）");
  let header = Header {
    width,
    height,
    color: !gray,
    spp,
  };
  let audio = encode_picture_audio("CQ ", &header, &rgb, mode, mode.rate, center).expect(
    "编码（子模式须有图片模式、每像素样本数须为 2 / 4 / 8，且中心频率须让图像带落在 0–Nyquist 内）",
  );
  std::fs::write(&out_path, ham_web_wav::encode(&audio, mode.rate)).expect("写入 WAV 失败");
  std::fs::write(format!("{out_path}.rgb"), &rgb).expect("写入原图失败");
  println!(
    "已生成 {out_path}（原图 {out_path}.rgb）：MFSK{mode_name} · {width} × {height} · {} · {pattern} · 每像素 {spp} 个样本 · 中心 {center} Hz · {:.1} 秒",
    if gray { "灰度" } else { "彩色" },
    audio.len() as f64 / f64::from(mode.rate)
  );
}

/// 取下一个参数并解析；缺省或解析失败就用默认值（几个尺寸类参数都是这样可省略）。
fn next_or<T: std::str::FromStr>(args: &mut impl Iterator<Item = String>, default: T) -> T {
  args.next().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// 测试图案（与 `reference/fldigi_web/gen_pic.mjs` 的同名图案一致）。
fn pattern_rgb(w: usize, h: usize, pattern: &str) -> Option<Vec<u8>> {
  let mut rgb = Vec::with_capacity(w * h * 3);
  match pattern {
    // 左右判半用浮点比较：奇数宽度下与 fldigi-web 的 `x < 宽 / 2` 一致（整数除法会多错一列）。
    "bands" => {
      for y in 0..h {
        for x in 0..w {
          let left = (x as f64) < w as f64 / 2.0;
          let r = if left { 220 } else { 40 };
          let g = if y < h / 2 { 180 } else { 70 };
          let b = if left { 30 } else { 210 };
          rgb.extend([r, g, b]);
        }
      }
    }
    "ramp" => {
      let dx = (w.max(2) - 1) as f64;
      let dy = (h.max(2) - 1) as f64;
      for y in 0..h {
        for x in 0..w {
          let b = if (x / 4 + y / 4) % 2 == 1 { 230 } else { 25 };
          rgb.extend([
            (x as f64 * 255.0 / dx).round() as u8,
            (y as f64 * 255.0 / dy).round() as u8,
            b,
          ]);
        }
      }
    }
    _ => return None,
  }
  Some(rgb)
}
