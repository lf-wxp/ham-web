//! 由 `public/pwa-icon.svg` 渲染 PWA / Apple Touch 图标（纯 Rust，基于 resvg）。
//!
//! 同一份 SVG 产出两类图标：
//!
//! - **圆角版**（`pwa-icon-*`）：保留设计好的品牌圆角，供 manifest 的 `any`、
//!   `favicon` 与社交分享图（`og:image`）使用 —— 这些场景不会替我们裁切；
//! - **满幅版**（`apple-touch-icon-*`、`pwa-maskable-*`）：不带圆角的不透明方形，
//!   圆角交给各平台的遮罩去加。
//!
//! 为什么必须有满幅版：iOS 的 `apple-touch-icon` 是**按原图直接使用**的（不会像
//! Android 的 `maskable` 那样套一层启动器形状），而它自己的 squircle 遮罩弧度与
//! SVG 里的圆弧圆角并不一致。拿一张自带圆角、四角透明的 PNG 过去，系统遮罩盖不
//! 住的四个角会把透明像素合成到白底上 —— 也就是「4 个角露出白色背景」。

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow, ensure};
use resvg::{tiny_skia, usvg};

use crate::fsutil::Paths;

/// 圆角版图标。
const ROUNDED: &[(&str, u32)] = &[("pwa-icon-192.png", 192), ("pwa-icon-512.png", 512)];

/// 满幅（无圆角、不透明）图标：圆角由系统遮罩添加。
const BLEED: &[(&str, u32)] = &[
  ("apple-touch-icon.png", 180),
  ("apple-touch-icon-precomposed.png", 180),
  ("pwa-maskable-192.png", 192),
  ("pwa-maskable-512.png", 512),
];

pub fn generate(paths: &Paths) -> Result<()> {
  let svg_path = paths.public.join("pwa-icon.svg");
  let svg = fs::read_to_string(&svg_path)
    .with_context(|| format!("failed to read {}", svg_path.display()))?;
  let bleed_svg = bleed(&svg)?;

  for &(name, px) in ROUNDED {
    save(&paths.public, name, px, &svg)?;
  }
  for &(name, px) in BLEED {
    save(&paths.public, name, px, &bleed_svg)?;
  }
  println!("All PWA icons generated successfully!");
  Ok(())
}

/// 把圆角 SVG 转成满幅 SVG：在 `<defs>` 之后补两块铺满画布的背景。
///
/// 背景渐变用的是默认的 `objectBoundingBox` 单位，而原背景矩形本来也是铺满画布
/// 的大小（`512×512`），所以补上去的矩形与原图渐变坐标完全一致 —— 只是多填了原
/// 圆角以外的区域，不会出现接缝。这里用 `100%` 而不是写死边长，避免 SVG 换尺寸后
/// 悄悄错位；`<defs>` 或渐变 id 不在时直接报错，免得改坏 SVG 后静默生成一张四角
/// 仍然透明的「满幅」图标。
fn bleed(svg: &str) -> Result<String> {
  const MARK: &str = "</defs>";
  let end = svg
    .find(MARK)
    .map(|i| i + MARK.len())
    .ok_or_else(|| anyhow!("pwa-icon.svg missing {MARK}"))?;
  ensure!(
    svg.contains(r#"id="bg""#),
    "pwa-icon.svg missing #bg gradient"
  );
  ensure!(
    svg.contains(r#"id="sheen""#),
    "pwa-icon.svg missing #sheen gradient"
  );
  let layer = "\n  <!-- 满幅背景：由 `cargo make icons` 注入，仅供 Apple Touch / maskable 图标 -->\n  <rect width=\"100%\" height=\"100%\" fill=\"url(#bg)\"/>\n  <rect width=\"100%\" height=\"100%\" fill=\"url(#sheen)\"/>";
  Ok(format!("{}{layer}{}", &svg[..end], &svg[end..]))
}

/// 按 `px×px` 渲染 `svg`，写入 `dir/name`。
fn save(dir: &Path, name: &str, px: u32, svg: &str) -> Result<()> {
  let tree =
    usvg::Tree::from_data(svg.as_bytes(), &usvg::Options::default()).context("invalid svg")?;
  let size = tree.size();
  let mut pixmap = tiny_skia::Pixmap::new(px, px).ok_or_else(|| anyhow!("invalid size {px}"))?;
  let transform =
    tiny_skia::Transform::from_scale(px as f32 / size.width(), px as f32 / size.height());
  resvg::render(&tree, transform, &mut pixmap.as_mut());
  let out = dir.join(name);
  pixmap
    .save_png(&out)
    .with_context(|| format!("failed to write {}", out.display()))?;
  println!("Generated {px}x{px} {name}");
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  const SRC: &str = include_str!("../../../public/pwa-icon.svg");

  /// 渲染 `px×px` 后取 `(x, y)` 处像素（用于断言四角是否透明）。
  fn pixel_at(svg: &str, px: u32, x: u32, y: u32) -> tiny_skia::PremultipliedColorU8 {
    let tree = usvg::Tree::from_data(svg.as_bytes(), &usvg::Options::default()).unwrap();
    let size = tree.size();
    let mut pixmap = tiny_skia::Pixmap::new(px, px).unwrap();
    let t = tiny_skia::Transform::from_scale(px as f32 / size.width(), px as f32 / size.height());
    resvg::render(&tree, t, &mut pixmap.as_mut());
    pixmap.pixel(x, y).unwrap()
  }

  /// 满幅版存在的唯一理由：把圆角版四角的透明像素补成不透明。
  #[test]
  fn bleed_fills_rounded_corners() {
    let bleed_svg = bleed(SRC).unwrap();
    for (x, y) in [(0, 0), (63, 0), (0, 63), (63, 63)] {
      assert_eq!(
        pixel_at(SRC, 64, x, y).alpha(),
        0,
        "圆角版 ({x},{y}) 应当是透明白角（故障根因）"
      );
      assert_eq!(
        pixel_at(&bleed_svg, 64, x, y).alpha(),
        255,
        "满幅版 ({x},{y}) 必须不透明，否则 iOS 上仍会露白"
      );
    }
  }

  /// 中心区域两版必须一致：满幅版只补四角，不该动图面主体。
  #[test]
  fn bleed_keeps_center_untouched() {
    let bleed_svg = bleed(SRC).unwrap();
    for (x, y) in [(32, 32), (16, 48), (48, 20)] {
      let rounded = pixel_at(SRC, 64, x, y);
      let full = pixel_at(&bleed_svg, 64, x, y);
      assert_eq!(
        (
          rounded.red(),
          rounded.green(),
          rounded.blue(),
          rounded.alpha()
        ),
        (full.red(), full.green(), full.blue(), full.alpha()),
        "({x},{y}) 两版渲染结果应当一致"
      );
    }
  }

  /// SVG 改坏时宁可报错，也不要静默产出一张四角透明的「满幅」图标。
  #[test]
  fn bleed_rejects_malformed_svg() {
    assert!(bleed("<svg/>").is_err());
    assert!(bleed("<defs></defs>").is_err());
  }
}
