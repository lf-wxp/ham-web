//! 由 `public/pwa-icon.svg` 渲染 PWA / Apple Touch 图标（纯 Rust，基于 resvg）。
//!
//! 同一份 SVG 产出两类图标：
//!
//! - **圆角版**（`pwa-icon-*`、`favicon.ico`）：保留设计好的品牌圆角（iOS 图标口径，
//!   22.37% 边长），供 manifest 的 `any`、`favicon` 与社交分享图（`og:image`）使用 ——
//!   这些场景不会替我们裁切；
//!   `favicon.ico` 是个容器，把 [`FAVICON`] 里的几档尺寸打包成单个文件，其中 16px
//!   一档用的是对齐到像素格的简化图形（母版缩到 16px 会糊，见该文件里的注释）；
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

/// `favicon.ico` 里打包的 `(图形文件, 尺寸)`：浏览器标签页、书签栏与桌面快捷方式取用的几档。
const FAVICON: &[(&str, u32)] = &[
  ("favicon-16.svg", 16),
  ("pwa-icon.svg", 32),
  ("pwa-icon.svg", 48),
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
  write_ico(&paths.public, FAVICON)?;
  println!("All PWA icons generated successfully!");
  Ok(())
}

/// 把圆角 SVG 转成满幅 SVG：在 `<defs>` 之后补一块铺满画布的背景。
///
/// 补上去的矩形与图面背景同色（同一个 `#bg`），只是多填了原圆角以外的区域，
/// 不会出现接缝。这里用 `100%` 而不是写死边长，避免 SVG 换尺寸后悄悄错位；
/// `<defs>` 或 `#bg` 不在时直接报错，免得改坏 SVG 后静默生成一张四角仍然透明的
/// 「满幅」图标。
fn bleed(svg: &str) -> Result<String> {
  const MARK: &str = "</defs>";
  let end = svg
    .find(MARK)
    .map(|i| i + MARK.len())
    .ok_or_else(|| anyhow!("pwa-icon.svg missing {MARK}"))?;
  ensure!(svg.contains(r#"id="bg""#), "pwa-icon.svg missing #bg fill");
  let layer = "\n  <!-- 满幅背景：由 `cargo make icons` 注入，仅供 Apple Touch / maskable 图标 -->\n  <rect width=\"100%\" height=\"100%\" fill=\"url(#bg)\"/>";
  Ok(format!("{}{layer}{}", &svg[..end], &svg[end..]))
}

/// 按 [`FAVICON`] 逐档渲染并打包成 `favicon.ico`。
///
/// 条目里嵌的是 PNG（而不是老式的 BMP + AND 掩码）：resvg 本来就产出 PNG，直接嵌省事，
/// 半透明与体积也更好。读不了 PNG 条目的只有 IE11 之前的浏览器，而这个站是 wasm 应用，
/// 本来就不支持它们。
fn write_ico(public: &Path, entries: &[(&str, u32)]) -> Result<()> {
  let mut sizes = Vec::with_capacity(entries.len());
  let mut pngs = Vec::with_capacity(entries.len());
  for &(file, px) in entries {
    let svg_path = public.join(file);
    let svg = fs::read_to_string(&svg_path)
      .with_context(|| format!("failed to read {}", svg_path.display()))?;
    sizes.push(px);
    pngs.push(render_png(&svg, px)?);
  }
  let path = public.join("favicon.ico");
  fs::write(&path, pack_ico(&sizes, &pngs))
    .with_context(|| format!("failed to write {}", path.display()))?;
  println!("Generated favicon.ico（{sizes:?}）");
  Ok(())
}

/// 按 ICO 容器格式拼字节：`ICONDIR` + 每档一条 `ICONDIRENTRY` + 依次排布的 PNG 负载。
fn pack_ico(sizes: &[u32], pngs: &[Vec<u8>]) -> Vec<u8> {
  assert_eq!(sizes.len(), pngs.len(), "ICO 尺寸与渲染结果数量不一致");
  let head = 6 + 16 * sizes.len();
  let mut out = Vec::with_capacity(head + pngs.iter().map(Vec::len).sum::<usize>());
  out.extend_from_slice(&0u16.to_le_bytes()); // 保留字段
  out.extend_from_slice(&1u16.to_le_bytes()); // 1 = 图标（2 = 光标）
  out.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
  let mut offset = head as u32;
  for (&px, png) in sizes.iter().zip(pngs) {
    // 宽高各占一个字节，256 按约定写成 0；这里没有这么大的一档，真加了也得照规矩写。
    let dim = if px == 256 { 0 } else { px as u8 };
    out.extend_from_slice(&[dim, dim, 0, 0]);
    out.extend_from_slice(&1u16.to_le_bytes()); // 色平面数
    out.extend_from_slice(&32u16.to_le_bytes()); // 每像素位数
    out.extend_from_slice(&(png.len() as u32).to_le_bytes());
    out.extend_from_slice(&offset.to_le_bytes());
    offset += png.len() as u32;
  }
  for png in pngs {
    out.extend_from_slice(png);
  }
  out
}

/// 渲染 `svg` 到 `px×px` 的 PNG 字节。
fn render_png(svg: &str, px: u32) -> Result<Vec<u8>> {
  let tree =
    usvg::Tree::from_data(svg.as_bytes(), &usvg::Options::default()).context("invalid svg")?;
  let size = tree.size();
  let mut pixmap = tiny_skia::Pixmap::new(px, px).ok_or_else(|| anyhow!("invalid size {px}"))?;
  let transform =
    tiny_skia::Transform::from_scale(px as f32 / size.width(), px as f32 / size.height());
  resvg::render(&tree, transform, &mut pixmap.as_mut());
  pixmap.encode_png().context("failed to encode png")
}

/// 按 `px×px` 渲染 `svg`，写入 `dir/name`。
fn save(dir: &Path, name: &str, px: u32, svg: &str) -> Result<()> {
  let out = dir.join(name);
  fs::write(&out, render_png(svg, px)?)
    .with_context(|| format!("failed to write {}", out.display()))?;
  println!("Generated {px}x{px} {name}");
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  const SRC: &str = include_str!("../../../public/pwa-icon.svg");
  /// 16px 档专用的简化图形。
  const SMALL: &str = include_str!("../../../public/favicon-16.svg");

  /// 取 [`FAVICON`] 某一档用的图形内容。
  ///
  /// 新增图形文件却忘了在测试里登记时会 panic —— 提醒补上对应的 const 与断言，
  /// 而不是让新图形悄悄溜过检查。
  fn artwork(file: &str) -> &'static str {
    match file {
      "favicon-16.svg" => SMALL,
      "pwa-icon.svg" => SRC,
      other => panic!("未知图形文件 {other}"),
    }
  }

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

  /// 背景整体仍是品牌蓝：贴边采样不透明、蓝通道明显压过红绿。
  ///
  /// 不再断言纯色 —— 图标按 iOS 口径叠了玻璃渐变（顶部反光、斜向光带），
  /// 但底色不许被换成白底 / 黑框这类别的颜色：采样点与 `bleed` 一样贴边取，
  /// 高光最亮的顶部（256,4）也在其中。
  #[test]
  fn background_stays_brand_blue() {
    for (x, y) in [(256, 4), (4, 256), (508, 256), (256, 508), (256, 484)] {
      let p = pixel_at(SRC, 512, x, y);
      let (r, g, b) = (p.red() as i32, p.green() as i32, p.blue() as i32);
      assert_eq!(p.alpha(), 255, "({x},{y}) 背景应当不透明");
      assert!(
        b > r + 0x30 && b > g + 0x30,
        "({x},{y}) 是 #{r:02x}{g:02x}{b:02x}：不再是品牌蓝底"
      );
    }
  }

  /// 圆角按 iOS 图标口径（≈22.4% 边长，rx=115）：它是这套图标的品牌外轮廓，
  /// 防止被改回早期的小圆角。
  #[test]
  fn corner_radius_matches_ios_ratio() {
    // 64px 渲染下圆角圆心在 (14.375, 14.375)：(3,3) 落在圆角之外、(8,8) 在图标之内。
    assert_eq!(pixel_at(SRC, 64, 3, 3).alpha(), 0, "(3,3) 应在圆角之外");
    assert_eq!(pixel_at(SRC, 64, 8, 8).alpha(), 255, "(8,8) 应在图标之内");
  }

  /// ICO 容器：条目表必须自洽（宽高、负载长度、偏移），否则浏览器会当坏文件整个忽略掉。
  #[test]
  fn ico_container_layout_is_consistent() {
    let sizes = FAVICON.iter().map(|&(_, px)| px).collect::<Vec<_>>();
    let pngs = FAVICON
      .iter()
      .map(|&(file, px)| render_png(artwork(file), px).unwrap())
      .collect::<Vec<_>>();
    let ico = pack_ico(&sizes, &pngs);

    assert_eq!(
      &ico[..6],
      &[0x00, 0x00, 0x01, 0x00, sizes.len() as u8, 0x00],
      "ICONDIR：保留字段 / 图标类型 / 条目数"
    );
    let mut offset = 6 + 16 * sizes.len();
    for (i, (&px, png)) in sizes.iter().zip(&pngs).enumerate() {
      let entry = &ico[6 + 16 * i..22 + 16 * i];
      let u16_at = |k: usize| u16::from_le_bytes([entry[k], entry[k + 1]]);
      let u32_at =
        |k: usize| u32::from_le_bytes([entry[k], entry[k + 1], entry[k + 2], entry[k + 3]]);
      assert_eq!((entry[0], entry[1]), (px as u8, px as u8), "第 {i} 项宽高");
      assert_eq!((entry[2], entry[3]), (0, 0), "第 {i} 项调色板数 / 保留字段");
      assert_eq!(u16_at(4), 1, "第 {i} 项色平面数");
      assert_eq!(u16_at(6), 32, "第 {i} 项每像素位数");
      assert_eq!(u32_at(8) as usize, png.len(), "第 {i} 项负载长度");
      assert_eq!(u32_at(12) as usize, offset, "第 {i} 项偏移应当紧接上一项");
      assert_eq!(
        &ico[offset..offset + 8],
        b"\x89PNG\r\n\x1a\n",
        "第 {i} 项负载应当是 PNG"
      );
      offset += png.len();
    }
    assert_eq!(ico.len(), offset, "末尾不应有多余字节");
  }

  /// 16px 档必须整像素对齐：渲染成 16×16 后只该出现「底色蓝 / 前景白」两种颜色，
  /// 冒出中间色就说明坐标又压到半像素上（母版直接缩到 16px 正是这个毛病）。
  #[test]
  fn favicon_16_stays_crisp() {
    const BG: (u8, u8, u8, u8) = (0x3b, 0x5d, 0xc9, 255);
    const FG: (u8, u8, u8, u8) = (0xff, 0xff, 0xff, 255);
    let at = |x: u32, y: u32| {
      let p = pixel_at(SMALL, 16, x, y);
      (p.red(), p.green(), p.blue(), p.alpha())
    };

    // 圆角裁切（rx=3）只在最外两圈像素上抗锯齿，取样避开它们。
    for y in 2..14 {
      for x in 2..14 {
        let rgba = at(x, y);
        assert!(
          rgba == BG || rgba == FG,
          "({x},{y}) 是 {rgba:?}：16px 图形只该有底色与前景两种颜色"
        );
      }
    }
    // 顺带确认图形没被删空：横梁左端是前景、肩部那条 1px 的缝是底色、
    // 塔顶上方的电波也在。
    assert_eq!(at(2, 9), FG, "横梁左端");
    assert_eq!(at(5, 7), BG, "肩部缝");
    assert_eq!(at(7, 1), FG, "塔顶上方的波");
  }

  /// SVG 改坏时宁可报错，也不要静默产出一张四角透明的「满幅」图标。
  #[test]
  fn bleed_rejects_malformed_svg() {
    assert!(bleed("<svg/>").is_err());
    assert!(bleed("<defs></defs>").is_err());
  }
}
