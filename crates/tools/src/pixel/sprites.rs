//! 生成像素图标与精灵的 Rust 数据文件（`crates/app/src/icons/pixel/{icon_paths,sprite_data}.rs`）。
//!
//! 为什么用生成器而不是手写 path：
//! - pixelarticons（MIT）是 24×24 网格、整数坐标的「矢量化像素」，直接内嵌即可；
//! - 它没有的图标与全部游戏精灵，用「字符画 + 调色板」来画（数据见 [`super::sprite_art`]），
//!   这里把同色相邻像素按行合并成矩形，输出紧凑的 SVG `path`，运行时零计算。
//!
//! 产物提交入库，日常开发不需要运行；改了字符画或换了图标集才重跑：
//!
//! ```text
//! cargo make pixel-sprites          # 需要 PIXELARTICONS_DIR 指向 pixelarticons 的 svg 目录
//! ```

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail, ensure};
use regex::Regex;

use super::sprite_art::{HAND_ICONS, ICON_MAP, PALETTE, SPRITES};

/// 矩形：`(x, y, 宽, 高)`，单位是像素格。
type Rect = (u32, u32, u32, u32);
/// 一行里的同色连续段：`(y, x, 宽)`。
type Run = (u32, u32, u32);

/// 生成物头部：来源与许可证声明。
const HEADER: &str =
  "//! 由 `cargo make pixel-sprites`（`crates/tools/src/pixel/sprites.rs`）生成，请勿手改。
//!
//! 图标来源：pixelarticons（MIT License，Copyright (c) 2019 Gerrit Halfmann，
//! https://github.com/halfmage/pixelarticons）；其余图标与全部精灵为本项目手绘。
";

/// 把字符画按「同色连续横条」合并成矩形，再把上下对齐的相同横条合并成更高的矩形。
///
/// 返回值按「颜色字符首次出现的顺序」排列（行优先扫描）：图层顺序就是它，生成物要稳定。
fn rects_from_grid(rows: &[&str], scale: u32) -> Vec<(char, Vec<Rect>)> {
  // 第一步：每行里的同色连续段。
  let mut runs: Vec<(char, Vec<Run>)> = Vec::new();
  for (y, row) in (0u32..).zip(rows) {
    let cells: Vec<char> = row.chars().collect();
    let mut x = 0usize;
    while x < cells.len() {
      let ch = cells[x];
      if ch == '.' || ch == ' ' {
        x += 1;
        continue;
      }
      let start = x;
      while x < cells.len() && cells[x] == ch {
        x += 1;
      }
      let run = (y, start as u32, (x - start) as u32);
      match runs.iter_mut().find(|(c, _)| *c == ch) {
        Some((_, v)) => v.push(run),
        None => runs.push((ch, vec![run])),
      }
    }
  }

  // 第二步：上下对齐（同 x、同宽、连续行）的段并成一个更高的矩形。
  runs
    .into_iter()
    .map(|(ch, mut rs)| {
      rs.sort_unstable();
      // `(x, 宽)` → `(起始行, 已累计高度)`
      let mut open: BTreeMap<(u32, u32), (u32, u32)> = BTreeMap::new();
      let mut done: Vec<Rect> = Vec::new();
      let mut last_y = None;
      for (y, x, w) in rs {
        if last_y != Some(y) {
          // 换行：上一行没被延续的矩形收尾。
          let stale: Vec<_> = open
            .iter()
            .filter(|(_, (y0, h))| y0 + h != y)
            .map(|(k, _)| *k)
            .collect();
          for key in stale {
            if let Some((y0, h)) = open.remove(&key) {
              done.push((key.0, y0, key.1, h));
            }
          }
          last_y = Some(y);
        }
        match open.get(&(x, w)).copied() {
          Some((y0, h)) if y0 + h == y => {
            open.insert((x, w), (y0, h + 1));
          }
          Some((y0, h)) => {
            done.push((x, y0, w, h));
            open.insert((x, w), (y, 1));
          }
          None => {
            open.insert((x, w), (y, 1));
          }
        }
      }
      done.extend(open.into_iter().map(|((x, w), (y0, h))| (x, y0, w, h)));
      let scaled = done
        .into_iter()
        .map(|(x, y, w, h)| (x * scale, y * scale, w * scale, h * scale))
        .collect();
      (ch, scaled)
    })
    .collect()
}

/// 矩形 → SVG path：`M{x} {y}h{w}v{h}h-{w}z`，按（行，列）排序保证输出稳定。
fn rects_to_d(rects: &[Rect]) -> String {
  let mut sorted = rects.to_vec();
  sorted.sort_by_key(|&(x, y, _, _)| (y, x));
  sorted.iter().fold(String::new(), |mut out, &(x, y, w, h)| {
    let _ = write!(out, "M{x} {y}h{w}v{h}h-{w}z");
    out
  })
}

/// Rust 原始字符串字面量。
fn rust_str(s: &str) -> String {
  format!("r#\"{s}\"#")
}

/// 取 pixelarticons 单个 SVG 里全部 `<path>` 的 `d`，按出现顺序拼接。
///
/// 少数图标（headphone / scale / reload 等）把图形拆成多个 `<path>`，只取第一个会丢掉
/// 大部分形状；每个 `d` 都以 `z` 收尾、下一个以 `M` 开头，直接拼是合法的。
fn parse_path_d(svg: &str) -> Result<String> {
  let re = Regex::new(r#"<path[^>]*\sd="([^"]+)""#)?;
  let d: Vec<&str> = re
    .captures_iter(svg)
    .map(|c| c.get(1).map(|m| m.as_str()).unwrap_or_default())
    .filter(|s| !s.is_empty())
    .collect();
  ensure!(!d.is_empty(), "svg 里没有带 d 的 <path>");
  Ok(d.concat())
}

/// 图标数据文件的全文（未经 rustfmt）。
fn icon_paths_source(icons_dir: &Path) -> Result<String> {
  let mut out = format!(
    "{HEADER}\nuse crate::icons::IconKind;\n\n/// 24×24 网格里的像素图标路径（`fill=\"currentColor\"`）。\npub(in crate::icons) const fn icon_path(kind: IconKind) -> &'static str {{\n  match kind {{\n"
  );
  for &(variant, file) in ICON_MAP {
    let d = match file {
      Some(name) => {
        let path = icons_dir.join(format!("{name}.svg"));
        let svg = fs::read_to_string(&path)
          .with_context(|| format!("读取 {} 失败（IconKind::{variant}）", path.display()))?;
        parse_path_d(&svg).with_context(|| path.display().to_string())?
      }
      None => {
        let rows = HAND_ICONS
          .iter()
          .find(|(name, _)| *name == variant)
          .map(|(_, rows)| rows)
          .ok_or_else(|| anyhow!("IconKind::{variant} 既没有 pixelarticons 文件也没有手绘稿"))?;
        // 手绘图标只有一种颜色（`#`）；12×12 放大 2 倍恰好铺满 24×24。
        let by_color = rects_from_grid(rows, 2);
        let rects = by_color
          .iter()
          .find(|(c, _)| *c == '#')
          .map_or(&[][..], |(_, r)| r.as_slice());
        // 手绘稿一个 `#` 都没有时会静默产出一个空图标：宁可报错，也不要「图标不见了」。
        ensure!(
          !rects.is_empty(),
          "IconKind::{variant} 的手绘稿里没有 '#' 像素"
        );
        rects_to_d(rects)
      }
    };
    let _ = writeln!(out, "    IconKind::{variant} => {},", rust_str(&d));
  }
  out.push_str("  }\n}\n");
  Ok(out)
}

/// 精灵数据文件的全文（未经 rustfmt）。
fn sprite_data_source() -> Result<String> {
  let mut out = format!(
    "{HEADER}\n/// 一个调色板色块：颜色与它覆盖的像素路径。\npub struct SpriteLayer {{\n  pub fill: &'static str,\n  pub d: &'static str,\n}}\n\n/// 一张精灵：边长（像素格数）与分色图层。\npub struct SpriteData {{\n  pub size: u8,\n  pub layers: &'static [SpriteLayer],\n}}\n\n"
  );
  for &(name, rows) in SPRITES {
    let size = rows.len();
    ensure!(
      rows.iter().all(|r| r.chars().count() == size),
      "{name}: 精灵必须是正方形（{size} 行，每行 {:?} 个字符）",
      rows.iter().map(|r| r.chars().count()).collect::<Vec<_>>()
    );
    let ident = name.to_uppercase();
    let mut layers = String::new();
    for (ch, rects) in rects_from_grid(rows, 1) {
      let Some((_, fill)) = PALETTE.iter().find(|(c, _)| *c == ch) else {
        bail!("{name}: 未知调色板字符 {ch:?}");
      };
      let _ = writeln!(
        layers,
        "  SpriteLayer {{ fill: \"{fill}\", d: {} }},",
        rust_str(&rects_to_d(&rects))
      );
    }
    let _ = writeln!(out, "const {ident}_LAYERS: &[SpriteLayer] = &[\n{layers}];");
    let _ = writeln!(
      out,
      "pub const {ident}: SpriteData = SpriteData {{ size: {size}, layers: {ident}_LAYERS }};\n"
    );
  }
  out.push_str(
    "/// 按名称查找精灵（页面里用字符串选精灵，如成就 id → 徽章）。\npub fn sprite_by_name(name: &str) -> Option<&'static SpriteData> {\n  match name {\n",
  );
  for &(name, _) in SPRITES {
    let _ = writeln!(out, "    \"{name}\" => Some(&{}),", name.to_uppercase());
  }
  out.push_str("    _ => None,\n  }\n}\n");
  Ok(out)
}

/// 用项目的 `rustfmt.toml` 格式化生成物：否则 `cargo make fmt-check` 会因它们而红。
pub(super) fn rustfmt(root: &Path, files: &[&Path]) -> Result<()> {
  let status = Command::new("rustfmt")
    .args(["--edition", "2024", "--config-path"])
    .arg(root.join("rustfmt.toml"))
    .args(files)
    .status()
    .context("运行 rustfmt 失败（需要 Rust 工具链里的 rustfmt）")?;
  ensure!(status.success(), "rustfmt 返回 {status}");
  Ok(())
}

/// 生成 `icon_paths.rs` 与 `sprite_data.rs`。
pub fn generate(root: &Path, pixelarticons: &Path) -> Result<()> {
  let out_dir = root.join("crates/app/src/icons/pixel");
  fs::create_dir_all(&out_dir)?;
  let icon_file = out_dir.join("icon_paths.rs");
  let sprite_file = out_dir.join("sprite_data.rs");
  fs::write(&icon_file, icon_paths_source(pixelarticons)?)?;
  fs::write(&sprite_file, sprite_data_source()?)?;
  rustfmt(root, &[&icon_file, &sprite_file])?;
  println!(
    "图标 {} 个（手绘 {}），精灵 {} 张 → {}",
    ICON_MAP.len(),
    HAND_ICONS.len(),
    SPRITES.len(),
    out_dir.strip_prefix(root).unwrap_or(&out_dir).display()
  );
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn adjacent_same_color_cells_merge_into_one_rect() {
    let g = rects_from_grid(&["##", "##"], 1);
    assert_eq!(g, vec![('#', vec![(0, 0, 2, 2)])]);
  }

  #[test]
  fn different_widths_do_not_merge_vertically() {
    // 第二行比第一行窄：不能并成一个矩形，否则会多画一格。
    let g = rects_from_grid(&["###", "##."], 1);
    let rects = &g[0].1;
    assert_eq!(rects.len(), 2);
    let area: u32 = rects.iter().map(|&(_, _, w, h)| w * h).sum();
    assert_eq!(area, 5);
  }

  #[test]
  fn a_gap_row_closes_the_rect() {
    let g = rects_from_grid(&["#", ".", "#"], 1);
    assert_eq!(g[0].1.len(), 2, "中间隔了空行，上下两格是两个矩形");
  }

  #[test]
  fn scale_multiplies_every_coordinate() {
    let g = rects_from_grid(&[".#"], 2);
    assert_eq!(g, vec![('#', vec![(2, 0, 2, 2)])]);
  }

  #[test]
  fn layers_keep_first_appearance_order() {
    let g = rects_from_grid(&["BK", "KB"], 1);
    assert_eq!(g.iter().map(|(c, _)| *c).collect::<String>(), "BK");
  }

  #[test]
  fn path_is_sorted_by_row_then_column() {
    let d = rects_to_d(&[(4, 2, 1, 1), (0, 0, 2, 1), (3, 0, 1, 1)]);
    assert_eq!(d, "M0 0h2v1h-2zM3 0h1v1h-1zM4 2h1v1h-1z");
  }

  #[test]
  fn every_sprite_is_square_and_only_uses_known_palette_chars() {
    // 直接跑一遍生成：任何一张精灵画歪了或用了未登记的字符都会在这里报错。
    sprite_data_source().unwrap();
  }

  #[test]
  fn every_icon_kind_without_a_file_has_a_hand_drawing() {
    for &(variant, file) in ICON_MAP {
      if file.is_none() {
        assert!(
          HAND_ICONS.iter().any(|(n, _)| *n == variant),
          "{variant} 没有手绘稿"
        );
      }
    }
  }

  #[test]
  fn parses_all_path_d_in_order() {
    let svg = r#"<svg><path fill="none" d="M1 2h3z"/><path d="M9 9z"/></svg>"#;
    assert_eq!(parse_path_d(svg).unwrap(), "M1 2h3zM9 9z");
    assert!(parse_path_d("<svg/>").is_err());
  }
}
