//! 由 `public/` 下的设计源渲染站点图标（纯 Rust，基于 resvg）。
//!
//! 三份设计源，各管一类产物：
//!
//! - **squircle 母版** `pwa-icon.svg` → `pwa-icon-*`（manifest 的 `any`、`og:image`）
//!   与 `favicon.ico`。母版画的是超椭圆轮廓（`|x|^5 + |y|^5 = 1`，不是圆，也不是普通
//!   圆角矩形）的调谐器玻璃面板：占画布约 80%、下方有一圈边缘厚度与柔和投影，
//!   画布其余部分透明，供不会替我们裁切的场景使用；`favicon.ico` 是个容器，把
//!   [`FAVICON`] 里的几档尺寸打包成单个文件，其中 16px 一档用的是对齐到像素格的
//!   简化图形（母版缩到 16px 会糊，见该文件里的注释）。
//! - **满幅版** `pwa-maskable.svg` → `apple-touch-icon-*`、`pwa-maskable-*`：
//!   整块画布都是背景、四角不透明、图形缩在安全圆内，圆角交给各平台的遮罩去加。
//! - **单色版** `pwa-badge.svg` → `pwa-badge-96.png`：通知栏 badge，白图形透明底。
//!
//! 为什么必须有满幅版：iOS 的 `apple-touch-icon` 是**按原图直接使用**的（不会像
//! Android 的 `maskable` 那样套一层启动器形状），而它自己的 squircle 遮罩弧度与
//! SVG 里的圆弧圆角并不一致。拿一张自带圆角、四角透明的 PNG 过去，系统遮罩盖不
//! 住的四个角会把透明像素合成到白底上 —— 也就是「4 个角露出白色背景」。
//!
//! 满幅版**不是**「squircle 母版 + 在角上补一块背景」拼出来的（早期是那么做的）：那样
//! 玻璃高光与毛玻璃盘仍按轮廓裁切，遮罩一盖就露出一个「圆角方框」的接缝，母版里
//! 的边缘高光线也会变成一个画在方形里的描边。所以满幅版有自己的设计源。

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use resvg::{tiny_skia, usvg};

use crate::fsutil::Paths;

/// squircle 母版（manifest `any`、`og:image`、favicon 的 48 档与浏览器的 SVG 图标）。
const ROUNDED_SRC: &str = "pwa-icon.svg";

/// squircle 版图标。
const ROUNDED: &[(&str, u32)] = &[("pwa-icon-192.png", 192), ("pwa-icon-512.png", 512)];

/// 满幅版设计源。
const BLEED_SRC: &str = "pwa-maskable.svg";

/// 满幅（无轮廓、不透明）图标：圆角由系统遮罩添加。
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
/// 刻度线与电波细弧在这个尺度上只有 1–2px，插值出来是一团灰。48 档仍用母版，细节撑得住。
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

  /// 母版的画布四角是透明的（写死这份预期，才能说明满幅版在解决什么问题）。
  #[test]
  fn master_canvas_corners_are_transparent() {
    for (x, y) in [(0, 0), (63, 0), (0, 63), (63, 63)] {
      assert_eq!(
        pixel_at(SRC, 64, x, y).alpha(),
        0,
        "母版 ({x},{y}) 应当是透明角"
      );
    }
  }

  /// 母版里的 squircle 只占画布约 80%，四周与下方留给边缘厚度和柔和投影：
  /// 四边中点是透明的，面板正下方是半透明的投影而不是实色。
  #[test]
  fn master_tile_occupies_about_eighty_percent() {
    let pixmap = render(SRC, 512);
    for (x, y) in [(256, 2), (2, 256), (509, 256)] {
      assert_eq!(pixmap.pixel(x, y).unwrap().alpha(), 0, "({x},{y}) 应当留白");
    }
    assert_eq!(
      pixmap.pixel(256, 248).unwrap().alpha(),
      255,
      "面板中心应当不透明"
    );

    // 面板中线上连续不透明的宽度 / 画布宽度。
    let opaque = (0..512u32)
      .filter(|&x| pixmap.pixel(x, 248).unwrap().alpha() == 255)
      .count() as f32;
    let ratio = opaque / 512.0;
    assert!(
      (0.76..=0.84).contains(&ratio),
      "squircle 占画布 {:.1}%，应当在 80% 上下",
      ratio * 100.0
    );

    // 面板与边缘厚度之下是投影：有一点颜色、但不是实心。
    let shadow = pixmap.pixel(256, 468).unwrap().alpha();
    assert!(
      (1..255).contains(&shadow),
      "面板正下方 (256,468) 的 alpha 是 {shadow}：应当是半透明的柔和投影"
    );
  }

  /// 轮廓是超椭圆 `|x|^5 + |y|^5 = 1`，既不是圆也不是普通圆角矩形。
  ///
  /// 取右上象限 `x = 0.97a` 的竖线（`a` = 面板半宽 ≈ 203.4px，圆心 (256, 248)）：
  /// - 超椭圆的边界在 `y ≈ 0.676a`（离圆心 137.5px），
  /// - 同尺寸的 iOS 圆角矩形在 `y ≈ 0.714a`（145.2px），圆在 `y ≈ 0.243a`。
  ///
  /// 在两者相差的那 7.7px 里取点：该是内的得是实心、该是外的只许有极淡的投影余晕。
  #[test]
  fn master_outline_is_a_squircle() {
    let pixmap = render(SRC, 512);
    let x = 453;
    // 边界在 y = 248 - 137.5 = 110.5。
    let inside = pixmap.pixel(x, 114).unwrap().alpha();
    let outside = pixmap.pixel(x, 107).unwrap().alpha();
    assert_eq!(inside, 255, "超椭圆边界之内 3.5px 应当是实心");
    assert!(
      outside < 40,
      "超椭圆边界之外 3.5px 的 alpha 是 {outside}：像是画成了圆角矩形"
    );
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
  /// （半径 0.4 × 512 ≈ 205）里的内容，越界就是被切掉的旋钮与电波。
  ///
  /// 底色（钴蓝 → 青绿）叠上毛玻璃盘与反光后红通道最高也只到 100 出头，
  /// 而白色刻线 / 电波、琥珀指针、铝旋钮的红通道都在 160 以上，
  /// 所以「红通道 > 160」就是「这里是图形」。
  #[test]
  fn maskable_art_stays_inside_safe_circle() {
    const R: f32 = 204.8;
    let pixmap = render(MASKABLE, 512);
    let mut farthest: f32 = 0.0;
    let mut at = (0, 0);
    for y in 0..512u32 {
      for x in 0..512u32 {
        if pixmap.pixel(x, y).unwrap().red() <= 160 {
          continue;
        }
        let dx = x as f32 + 0.5 - 256.0;
        let dy = y as f32 + 0.5 - 256.0;
        if dx.hypot(dy) > farthest {
          farthest = dx.hypot(dy);
          at = (x, y);
        }
      }
    }
    assert!(
      farthest < R,
      "最外的图形像素在 {at:?}，离圆心 {farthest:.1}px，已超出 {R}px 的安全圆"
    );
    // 顺带确认图形没被缩成一团：正常缩放后旋钮外缘离圆心约 197。
    assert!(
      farthest > 170.0,
      "图形只铺到离圆心 {farthest:.1}px：缩得太小，启动器上会显得空"
    );
  }

  /// 满幅版里不能有轮廓裁切、边缘厚度与上下沿高光：遮罩之外露出的
  /// 「圆角方框」接缝与内嵌描边就是这么来的。
  #[test]
  fn maskable_has_no_squircle_tile() {
    for marker in [r#"id="sq""#, "url(#face)", "rimFace", "blurShadow"] {
      assert!(
        !MASKABLE.contains(marker),
        "满幅版不该再画轮廓 / 厚度 / 边缘高光：发现 {marker}"
      );
    }
  }

  /// 背景整体仍是「钴蓝 → 青绿」：左上偏蓝、右下偏绿。
  ///
  /// 不断言纯色 —— 底色上叠了毛玻璃与反光，但不许被换成白底 / 黑框这类别的颜色。
  /// 母版取面板内部的点（画布外圈是透明的），满幅版取四角：最容易被改成白底的地方。
  #[test]
  fn background_runs_cobalt_to_teal() {
    let cases = [
      ("pwa-icon.svg", SRC, (110, 100), (430, 400)),
      ("pwa-maskable.svg", MASKABLE, (4, 4), (507, 507)),
    ];
    for (name, svg, (bx, by), (gx, gy)) in cases {
      let blue = pixel_at(svg, 512, bx, by);
      let (r, g, b) = (blue.red() as i32, blue.green() as i32, blue.blue() as i32);
      assert_eq!(blue.alpha(), 255, "{name} ({bx},{by}) 背景应当不透明");
      assert!(
        b > r + 0x30 && b > g + 0x30,
        "{name} ({bx},{by}) 是 #{r:02x}{g:02x}{b:02x}：左上不再是钴蓝"
      );

      let teal = pixel_at(svg, 512, gx, gy);
      let (r, g, b) = (teal.red() as i32, teal.green() as i32, teal.blue() as i32);
      assert_eq!(teal.alpha(), 255, "{name} ({gx},{gy}) 背景应当不透明");
      assert!(
        g > r + 0x30 && b > r + 0x30,
        "{name} ({gx},{gy}) 是 #{r:02x}{g:02x}{b:02x}：右下不再是青绿"
      );
    }
  }

  /// 几份设计源共用一套配色：改了母版却漏改满幅版 / favicon 版会在这里报出来。
  #[test]
  fn art_sources_share_brand_palette() {
    for (name, svg) in [("pwa-icon.svg", SRC), ("pwa-maskable.svg", MASKABLE)] {
      for stop in ["#0b3d91", "#0f766e", "#f59e0b"] {
        assert!(svg.contains(stop), "{name} 缺少品牌色 {stop}");
      }
    }
    for stop in ["#0a5c8c", "#f59e0b"] {
      assert!(
        SMALL.contains(stop),
        "favicon-16.svg 缺少 {stop}（底色取母版渐变的中段，指针与母版同为琥珀色）"
      );
    }
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
    // 像素稿的 (6,6) 格是指针；整体向左平移一格，所以像素坐标 = 格 × 6 - 6。
    assert_eq!(
      pixel_at(BADGE_ART, 96, 33, 39).alpha(),
      255,
      "指针应当被填上"
    );
    assert_eq!(
      pixel_at(BADGE_ART, 96, 57, 9).alpha(),
      255,
      "右上角的电波应当在"
    );
    assert_eq!(
      pixel_at(BADGE_ART, 96, 9, 63).alpha(),
      255,
      "左侧的长刻线应当在"
    );
    assert_eq!(
      pixel_at(BADGE_ART, 96, 69, 75).alpha(),
      255,
      "右下角的旋钮应当在"
    );
    assert_eq!(
      pixel_at(BADGE_ART, 96, 15, 63).alpha(),
      0,
      "刻线之间应当留缝"
    );
    // 留白别把图形挤到边上：图形占 x 2…15、y 1…14，平移后四周各留 6px。
    assert_eq!(
      pixel_at(BADGE_ART, 96, 2, 48).alpha(),
      0,
      "badge 左右应当留白"
    );
    assert_eq!(
      pixel_at(BADGE_ART, 96, 57, 2).alpha(),
      0,
      "badge 上下应当留白"
    );
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

  /// 16px 档必须整像素对齐：渲染成 16×16 后只该出现设计稿里的几种实色
  /// （底色 / 刻度窗 / 白 / 琥珀 / 旋钮亮面 / 旋钮暗面），
  /// 冒出中间色就说明坐标又压到半像素上（母版直接缩到 16px 正是这个毛病）。
  #[test]
  fn favicon_16_stays_crisp() {
    type Rgba = (u8, u8, u8, u8);
    const BG: Rgba = (0x0a, 0x5c, 0x8c, 255);
    const PLATE: Rgba = (0x36, 0x79, 0xa1, 255);
    const WHITE: Rgba = (0xff, 0xff, 0xff, 255);
    const AMBER: Rgba = (0xf5, 0x9e, 0x0b, 255);
    const KNOB: Rgba = (0xe2, 0xe8, 0xf0, 255);
    const SHADE: Rgba = (0x94, 0xa3, 0xb8, 255);
    let at = |x: u32, y: u32| {
      let p = pixel_at(SMALL, 16, x, y);
      (p.red(), p.green(), p.blue(), p.alpha())
    };

    // 圆角裁切（rx=3.5）只在最外两圈像素上抗锯齿，取样避开它们。
    for y in 2..14 {
      for x in 2..14 {
        let rgba = at(x, y);
        assert!(
          [BG, PLATE, WHITE, AMBER, KNOB, SHADE].contains(&rgba),
          "({x},{y}) 是 {rgba:?}：16px 图形只该有设计稿里的实色"
        );
      }
    }
    // 顺带确认图形没被删空：指针、刻度窗、刻线、旋钮与右上角的三道电波都在，
    // 指针两侧要有缝（刻度窗的颜色露出来），电波之间也要有缝。
    assert_eq!(at(6, 6), AMBER, "琥珀指针（窗口之上的部分）");
    assert_eq!(at(7, 12), AMBER, "琥珀指针（横过刻度窗的部分）");
    assert_eq!(at(5, 11), PLATE, "指针左侧的缝");
    assert_eq!(at(2, 11), WHITE, "左侧的长刻线");
    assert_eq!(at(3, 11), PLATE, "刻线之间的缝");
    assert_eq!(at(12, 12), KNOB, "右下角的铝旋钮");
    assert_eq!(at(14, 12), SHADE, "旋钮的暗面");
    assert_eq!(at(13, 1), WHITE, "最内一道电波");
    assert_eq!(at(10, 1), WHITE, "中间一道电波");
    assert_eq!(at(8, 1), WHITE, "最外一道电波");
    assert_eq!(at(11, 1), BG, "两道电波之间的缝");
    assert_eq!(at(3, 3), BG, "左上是纯底色");
    // 32 档是这份图形的 2 倍整数放大：16 格的每一格都变成 2×2，同样不许有灰边。
    let p = pixel_at(SMALL, 32, 4, 22);
    assert_eq!(
      (p.red(), p.green(), p.blue(), p.alpha()),
      WHITE,
      "32 档放大后左侧长刻线仍应是纯白"
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
