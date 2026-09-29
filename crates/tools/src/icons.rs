//! 由 `public/pwa-icon.svg` 渲染 PWA / Apple Touch 图标（纯 Rust，基于 resvg）。

use std::fs;

use anyhow::{Context, Result, anyhow};
use resvg::{tiny_skia, usvg};

use crate::fsutil::Paths;

const OUTPUTS: &[(&str, u32)] = &[
  ("pwa-icon-192.png", 192),
  ("pwa-icon-512.png", 512),
  ("apple-touch-icon.png", 180),
  ("apple-touch-icon-precomposed.png", 180),
];

pub fn generate(paths: &Paths) -> Result<()> {
  let svg_path = paths.public.join("pwa-icon.svg");
  let svg =
    fs::read(&svg_path).with_context(|| format!("failed to read {}", svg_path.display()))?;
  let tree = usvg::Tree::from_data(&svg, &usvg::Options::default()).context("invalid svg")?;
  let size = tree.size();

  for &(name, px) in OUTPUTS {
    let mut pixmap = tiny_skia::Pixmap::new(px, px).ok_or_else(|| anyhow!("invalid size {px}"))?;
    let transform =
      tiny_skia::Transform::from_scale(px as f32 / size.width(), px as f32 / size.height());
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let out = paths.public.join(name);
    pixmap
      .save_png(&out)
      .with_context(|| format!("failed to write {}", out.display()))?;
    println!("Generated {px}x{px} {name}");
  }
  println!("All PWA icons generated successfully!");
  Ok(())
}
