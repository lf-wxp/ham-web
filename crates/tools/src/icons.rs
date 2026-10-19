//! 由 `public/` 下的设计源渲染站点图标（纯 Rust，基于 resvg）。
//!
//! 三份设计源，各管一类产物：
//!
//! - **圆角母版** `pwa-icon.svg` → `pwa-icon-*`（manifest 的 `any`、`og:image`）
//!   与 `favicon.ico`。母版保留设计好的品牌圆角（iOS 图标口径，22.37% 边长），
//!   供不会替我们裁切的场景使用；`favicon.ico` 是个容器，把 [`FAVICON`] 里的几档
//!   尺寸打包成单个文件，其中 16px 一档用的是对齐到像素格的简化图形
//!   （母版缩到 16px 会糊，见该文件里的注释）。
//! - **满幅版** `pwa-maskable.svg` → `apple-touch-icon-*`、`pwa-maskable-*`：
//!   整块画布都是背景、四角不透明、图形缩在安全圆内，圆角交给各平台的遮罩去加。
//! - **单色版** `pwa-badge.svg` → `pwa-badge-96.png`：通知栏 badge，白图形透明底。
//!
//! 为什么必须有满幅版：iOS 的 `apple-touch-icon` 是**按原图直接使用**的（不会像
//! Android 的 `maskable` 那样套一层启动器形状），而它自己的 squircle 遮罩弧度与
//! SVG 里的圆弧圆角并不一致。拿一张自带圆角、四角透明的 PNG 过去，系统遮罩盖不
//! 住的四个角会把透明像素合成到白底上 —— 也就是「4 个角露出白色背景」。
//!
//! 满幅版**不是**「圆角母版 + 在角上补一块背景」拼出来的（早期是那么做的）：那样
//! 玻璃高光与斜向光带仍按圆角裁切，遮罩一盖就露出一个「圆角方框」的接缝，母版里
//! 的圆角边缘高光线也会变成一个画在方形里的描边。所以满幅版有自己的设计源。

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use resvg::{tiny_skia, usvg};

use crate::fsutil::Paths;

/// 圆角母版（manifest `any`、`og:image`、favicon 的 32 / 48 档）。
const ROUNDED_SRC: &str = "pwa-icon.svg";

/// 圆角版图标。
const ROUNDED: &[(&str, u32)] = &[("pwa-icon-192.png", 192), ("pwa-icon-512.png", 512)];

/// 满幅版设计源。
const BLEED_SRC: &str = "pwa-maskable.svg";

/// 满幅（无圆角、不透明）图标：圆角由系统遮罩添加。
///
/// `apple-touch-icon-precomposed.png` 只被 iOS 6 及更早版本使用，页面里也没有任何 `link`
/// 引用它；生成一份没人下载的文件只会白占 SW 预缓存清单，故不再产出。
const BLEED: &[(&str, u32)] = &[
  ("apple-touch-icon.png", 180),
  ("pwa-maskable-192.png", 192),
  ("pwa-maskable-512.png", 512),
];

/// 单色设计源（通知栏 badge）。
const BADGE_SRC: &str = "pwa-badge.svg";

/// 通知栏 badge：Android 只取 alpha 通道并按系统主题着色，96px 是它建议的尺寸。
const BADGE: &[(&str, u32)] = &[("pwa-badge-96.png", 96)];

/// `favicon.ico` 里打包的 `(图形文件, 尺寸)`：浏览器标签页、书签栏与桌面快捷方式取用的几档。
///
/// 16 / 32 两档都用 `favicon-16.svg`（32 档是它的 2 倍整数放大，仍是整像素）：母版的
/// 塔腿在这个尺度上只有 1–2px，插值出来是一团灰。48 档仍用母版，细节撑得住。
const FAVICON: &[(&str, u32)] = &[
  ("favicon-16.svg", 16),
  ("favicon-16.svg", 32),
  ("pwa-icon.svg", 48),
];

pub fn generate(paths: &Paths) -> Result<()> {
  let rounded = read_svg(&paths.public, ROUNDED_SRC)?;
  let bleed = read_svg(&paths.public, BLEED_SRC)?;
  let badge = read_svg(&paths.public, BADGE_SRC)?;

  for &(name, px) in ROUNDED {
    save(&paths.public, name, px, &rounded)?;
  }
  for &(name, px) in BLEED {
    save(&paths.public, name, px, &bleed)?;
  }
  for &(name, px) in BADGE {
    save(&paths.public, name, px, &badge)?;
  }
  write_ico(&paths.public, FAVICON)?;
  println!("All PWA icons generated successfully!");
  Ok(())
}

/// 读 `public/` 下的设计源文件。
fn read_svg(public: &Path, name: &str) -> Result<String> {
  let path = public.join(name);
  fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))
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
  /// 满幅版设计源（Apple Touch / maskable）。
  const MASKABLE: &str = include_str!("../../../public/pwa-maskable.svg");
  /// 16px 档专用的简化图形。
  const SMALL: &str = include_str!("../../../public/favicon-16.svg");
  /// 通知栏 badge 设计源。
  const BADGE_ART: &str = include_str!("../../../public/pwa-badge.svg");

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

  /// 把 `svg` 渲染成 `px×px` 位图。
  fn render(svg: &str, px: u32) -> tiny_skia::Pixmap {
    let tree = usvg::Tree::from_data(svg.as_bytes(), &usvg::Options::default()).unwrap();
    let size = tree.size();
    let mut pixmap = tiny_skia::Pixmap::new(px, px).unwrap();
    let t = tiny_skia::Transform::from_scale(px as f32 / size.width(), px as f32 / size.height());
    resvg::render(&tree, t, &mut pixmap.as_mut());
    pixmap
  }

  /// 渲染 `px×px` 后取 `(x, y)` 处像素（用于断言四角是否透明）。
  fn pixel_at(svg: &str, px: u32, x: u32, y: u32) -> tiny_skia::PremultipliedColorU8 {
    render(svg, px).pixel(x, y).unwrap()
  }

  /// 圆角母版的四角是透明的（写死这份预期，才能说明满幅版在解决什么问题）。
  #[test]
  fn rounded_master_has_transparent_corners() {
    for (x, y) in [(0, 0), (63, 0), (0, 63), (63, 63)] {
      assert_eq!(
        pixel_at(SRC, 64, x, y).alpha(),
        0,
        "圆角版 ({x},{y}) 应当是透明白角"
      );
    }
  }

  /// 满幅版要交给系统遮罩：四角与四边中点都必须不透明，否则遮罩盖不到的角
  /// 会把透明像素合成到白底上（iOS 主屏、「启动器圆角方框」两个老问题的根因）。
  #[test]
  fn maskable_is_opaque_to_the_edge() {
    for (x, y) in [
      (0, 0),
      (511, 0),
      (0, 511),
      (511, 511),
      (0, 256),
      (511, 256),
      (256, 0),
      (256, 511),
    ] {
      assert_eq!(
        pixel_at(MASKABLE, 512, x, y).alpha(),
        255,
        "满幅版 ({x},{y}) 必须不透明"
      );
    }
  }

  /// 图形必须落在 maskable 的安全圆内：启动器的圆形遮罩只保证中心直径 80%
  /// （半径 0.4 × 512 ≈ 205）里的内容，越界就是被切掉的塔基与电波。
  ///
  /// 图形是纯白的，其余部分最亮处（顶部玻璃反光叠上光带）红通道也到不了 200，
  /// 所以「红通道 > 200」就是「这里是图形」。
  #[test]
  fn maskable_art_stays_inside_safe_circle() {
    const R: f32 = 204.8;
    let pixmap = render(MASKABLE, 512);
    let mut farthest: f32 = 0.0;
    for y in 0..512u32 {
      for x in 0..512u32 {
        if pixmap.pixel(x, y).unwrap().red() <= 200 {
          continue;
        }
        let dx = x as f32 + 0.5 - 256.0;
        let dy = y as f32 + 0.5 - 256.0;
        farthest = farthest.max(dx.hypot(dy));
      }
    }
    assert!(
      farthest < R,
      "最外的图形像素离圆心 {farthest:.1}px，已超出 {R}px 的安全圆"
    );
    // 顺带确认图形没被缩成一团：正常缩放后塔基角离圆心约 196。
    assert!(
      farthest > 180.0,
      "图形只铺到离圆心 {farthest:.1}px：缩得太小，启动器上会显得空"
    );
  }

  /// 满幅版里不能有圆角裁切，也不能有圆角边缘高光：遮罩之外露出的
  /// 「圆角方框」接缝与内嵌描边就是这么来的。
  #[test]
  fn maskable_has_no_rounded_tile() {
    assert!(
      !MASKABLE.contains(r#"clip-path=""#),
      "满幅版不该再按圆角裁切背景"
    );
    assert!(
      !MASKABLE.contains(r#"rx="115""#),
      "满幅版不该再画圆角边缘的高光细线"
    );
  }

  /// 背景整体仍是品牌蓝：贴边采样不透明、蓝通道明显压过红绿。
  ///
  /// 不断言纯色 —— 图标按 iOS 口径叠了玻璃渐变（顶部反光、斜向光带），
  /// 但底色不许被换成白底 / 黑框这类别的颜色：采样点贴边取，高光最亮的顶部
  /// （256,4）也在其中。两版都查：满幅版的四角是最容易被改成白底的地方。
  #[test]
  fn background_stays_brand_blue() {
    for (name, svg) in [("pwa-icon.svg", SRC), ("pwa-maskable.svg", MASKABLE)] {
      for (x, y) in [(256, 4), (4, 256), (508, 256), (256, 508), (256, 484)] {
        let p = pixel_at(svg, 512, x, y);
        let (r, g, b) = (p.red() as i32, p.green() as i32, p.blue() as i32);
        assert_eq!(p.alpha(), 255, "{name} ({x},{y}) 背景应当不透明");
        assert!(
          b > r + 0x30 && b > g + 0x30,
          "{name} ({x},{y}) 是 #{r:02x}{g:02x}{b:02x}：不再是品牌蓝底"
        );
      }
    }
  }

  /// 几份设计源共用一套品牌色：改了母版却漏改满幅版 / favicon 版会在这里报出来。
  #[test]
  fn art_sources_share_brand_palette() {
    for (name, svg) in [("pwa-icon.svg", SRC), ("pwa-maskable.svg", MASKABLE)] {
      for stop in ["#5f80ea", "#3b5dc9", "#2d489f"] {
        assert!(svg.contains(stop), "{name} 缺少品牌色 {stop}");
      }
    }
    assert!(
      SMALL.contains("#3b5dc9"),
      "favicon-16.svg 的底色要是品牌蓝的中段"
    );
  }

  /// badge 只能是「白色 / 透明」两色：Android 只取 alpha 通道再按系统主题着色，
  /// 彩色图标会被画成一个实心方块。坐标为整像素 ∕ 6 的倍数，本该没有中间色。
  #[test]
  fn badge_is_white_on_transparent() {
    let pixmap = render(BADGE_ART, 96);
    for y in 0..96u32 {
      for x in 0..96u32 {
        let p = pixmap.pixel(x, y).unwrap();
        let rgba = (p.red(), p.green(), p.blue(), p.alpha());
        assert!(
          rgba == (255, 255, 255, 255) || rgba == (0, 0, 0, 0),
          "badge ({x},{y}) 是 {rgba:?}：只允许纯白或全透明"
        );
      }
    }
    assert_eq!(
      pixel_at(BADGE_ART, 96, 0, 0).alpha(),
      0,
      "badge 四角必须透明"
    );
    assert_eq!(
      pixel_at(BADGE_ART, 96, 24, 66).alpha(),
      255,
      "塔腿中段应当被填上"
    );
    assert_eq!(
      pixel_at(BADGE_ART, 96, 48, 14).alpha(),
      255,
      "塔顶上方的那道波也应当在"
    );
    // 留白别把图形挤到边上：96 里图形占 12 × 6 = 72，两侧各 12px。
    assert_eq!(
      pixel_at(BADGE_ART, 96, 6, 48).alpha(),
      0,
      "badge 左右应当留白"
    );
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
    // 顺带确认图形没被删空：底座左端是前景、两条塔腿之间是底色（桁架要镂空）、
    // 塔顶上方的电波也在。
    assert_eq!(at(2, 12), FG, "底座左端");
    assert_eq!(at(7, 7), BG, "两条塔腿之间的缝");
    assert_eq!(at(7, 1), FG, "塔顶上方的波");
    // 32 档是这份图形的 2 倍整数放大：16 格的每一格都变成 2×2，同样不许有灰边。
    let p = pixel_at(SMALL, 32, 4, 24);
    assert_eq!(
      (p.red(), p.green(), p.blue(), p.alpha()),
      FG,
      "32 档放大后底座左端仍应是纯白"
    );
  }

  /// 设计源写坏时在这里失败，而不是安静地产出一张空白 / 四角透明的图标。
  #[test]
  fn art_sources_render_at_every_target_size() {
    for &(name, px) in ROUNDED {
      assert_eq!(render(SRC, px).width(), px, "{name}");
    }
    for &(name, px) in BLEED {
      let pixmap = render(MASKABLE, px);
      assert_eq!(
        pixmap.pixel(0, 0).unwrap().alpha(),
        255,
        "{name} 的四角必须不透明"
      );
    }
    for &(name, px) in BADGE {
      assert_eq!(render(BADGE_ART, px).width(), px, "{name}");
    }
  }
}
