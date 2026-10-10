//! 配色方案：由 [`ham_web_core::color_scheme`] 的方案表生成 `style/pixel/schemes.css`，并审计对比度。
//!
//! 方案表是唯一的数据源：种子色与派生规则在 core（浏览器里设置面板的色块预览也读它），
//! 这里只做两件事：
//! 1. 展开成 CSS（`html[data-scheme="x"]` 亮色块 + `html[data-scheme="x"].dark` 暗色块）；
//! 2. 审计「方案 × 明暗」里每一对字 / 底的对比度 —— 包括不随方案变的状态色文字
//!    （`text-gold-text` 等）落在新表面上是否还够，以及 Tailwind 色阶（见 [`super::palette`]）。
//!
//! 经典方案手写在 `tokens.css`，不在生成物里。

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use ham_web_core::color::Rgb;
use ham_web_core::color_scheme::{DEFAULT_SCHEME, SCHEMES, Scheme};

use super::palette::{self, SurfaceSet, Surfaces};

/// 生成物的头部说明。
const HEADER: &str =
  "/* 由 `cargo make pixel-schemes`（crates/tools/src/pixel/schemes.rs）生成，请勿手改。
   配色方案：每个方案一个亮色块 + 一个 `.dark` 暗色块，只覆盖随方案变化的语义变量；
   状态色（血条 / 经验条 / 金币 / 通关绿 / 危险红）与字体、圆角、阴影、动效不随方案变。
   经典方案是 tokens.css 里的基准，不在这里。
   方案表与派生规则：crates/core/src/color_scheme.rs。

   选择器优先级：亮色块 `html[data-scheme]` 高于 tokens.css 的 `:root` 与 `.dark`，
   所以暗色下由同样列全了变量的 `html[data-scheme].dark` 再盖回去。 */
";

/// 不随方案变化、但必须在新表面上仍然可读的「文字色」变量。
const STATUS_TEXT: [&str; 4] = [
  "--pxl-hp-text",
  "--pxl-xp-text",
  "--pxl-gold-text",
  "--pxl-win-text",
];

/// 危险红只当文字出现在页面的主表面上（错误提示、`text-destructive`）。
///
/// 不查 `secondary` / `muted` / `accent`：经典方案自己的危险红落在它的 `secondary` 上也只有 4.14，
/// 这条线若画得比基线还严，等于要求新方案比经典更好，而那些表面上本来就不放错误文字。
const DESTRUCTIVE_SURFACES: [&str; 3] = ["--background", "--card", "--popover"];

/// 状态色文字可能落到的表面变量（暗色 / 亮色同一份，强调面也算：`hover:bg-accent` 上会有字）。
const SURFACE_VARS: [&str; 6] = [
  "--background",
  "--card",
  "--popover",
  "--secondary",
  "--muted",
  "--accent",
];

/// 要生成的方案：除默认方案之外的全部。
fn generated() -> impl Iterator<Item = &'static Scheme> {
  SCHEMES.iter().filter(|s| s.id != DEFAULT_SCHEME)
}

fn mode_name(dark: bool) -> &'static str {
  if dark { "暗" } else { "亮" }
}

/// 一个方案在某个明暗下的变量：先取经典的整套，再用方案的覆盖。
///
/// 对比度审计看的是「用户实际看到的」，而不是只看方案覆盖的那几个。
fn effective_vars(
  scheme: &Scheme,
  dark: bool,
  classic: &BTreeMap<String, Rgb>,
) -> Result<BTreeMap<String, Rgb>> {
  let mut vars = classic.clone();
  let tokens = scheme
    .tokens(dark)
    .map_err(|e| anyhow!("方案 {}（{}）派生失败：{e}", scheme.id, mode_name(dark)))?;
  for (name, value) in tokens {
    if let Some(c) = value.color() {
      vars.insert(name.to_owned(), c);
    }
  }
  Ok(vars)
}

/// 展开成 CSS 全文。
pub fn generate_css() -> Result<String> {
  let mut out = String::from(HEADER);
  for scheme in generated() {
    let _ = writeln!(out, "\n/* ---------- {} ---------- */", scheme.id);
    for dark in [false, true] {
      let selector = if dark {
        format!("html[data-scheme=\"{}\"].dark", scheme.id)
      } else {
        format!("html[data-scheme=\"{}\"]", scheme.id)
      };
      let _ = writeln!(out, "{selector} {{");
      let tokens = scheme
        .tokens(dark)
        .map_err(|e| anyhow!("方案 {}（{}）派生失败：{e}", scheme.id, mode_name(dark)))?;
      for (name, value) in tokens {
        let _ = writeln!(out, "  {name}: {};", value.css());
      }
      out.push_str("}\n");
    }
  }
  Ok(out)
}

/// 把各方案的表面并入调色板审计（`pixel-palette` 用）。
pub fn extend_surfaces(surfaces: &mut Surfaces, root: &Path) -> Result<()> {
  let (light, dark) = classic_vars(root)?;
  for scheme in generated() {
    for (is_dark, classic, list) in [
      (false, &light, &mut surfaces.light),
      (true, &dark, &mut surfaces.dark),
    ] {
      let vars = effective_vars(scheme, is_dark, classic)?;
      list.push(SurfaceSet::from_tokens(scheme.id, &vars, is_dark)?);
    }
  }
  Ok(())
}

/// 经典方案的亮 / 暗变量（读 `tokens.css`）。
fn classic_vars(root: &Path) -> Result<(BTreeMap<String, Rgb>, BTreeMap<String, Rgb>)> {
  let css = fs::read_to_string(root.join(super::TOKENS_CSS))?;
  palette::classic_tokens(&css)
}

/// 审计：每个方案 × 明暗里，不随方案变的文字色在所有表面上都要 >= AA。
///
/// 方案自己派生的字色（正文 / 静音 / 主色上的字）在 core 里按表面校正过，且有单测；
/// 这里补的是那些「沿用经典」的文字色 —— 它们是按经典的表面校准的，换了表面不一定还够。
fn audit_inherited_text(
  light: &BTreeMap<String, Rgb>,
  dark: &BTreeMap<String, Rgb>,
) -> Result<Vec<String>> {
  let mut bad = Vec::new();
  for scheme in generated() {
    for (is_dark, classic) in [(false, light), (true, dark)] {
      let vars = effective_vars(scheme, is_dark, classic)?;
      let checks = STATUS_TEXT
        .iter()
        .map(|n| (*n, SURFACE_VARS.as_slice()))
        .chain([("--destructive", DESTRUCTIVE_SURFACES.as_slice())]);
      for (fg_name, surfaces) in checks {
        let Some(&fg) = vars.get(fg_name) else {
          bail!("缺少 {fg_name}");
        };
        for &surf in surfaces {
          let Some(&bg) = vars.get(surf) else {
            bail!("缺少 {surf}");
          };
          let ratio = fg.contrast(bg);
          if ratio < 4.5 {
            bad.push(format!(
              "[{}] {}：{fg_name} {} 落在 {surf} {} 上只有 {ratio:.2}",
              mode_name(is_dark),
              scheme.id,
              fg.hex(),
              bg.hex()
            ));
          }
        }
      }
    }
  }
  Ok(bad)
}

/// `pixel-schemes`：生成 `schemes.css`；`check` 为真时只校验（过期或对比度不达标则失败）。
pub fn run(root: &Path, check: bool) -> Result<()> {
  let path = root.join(super::SCHEMES_CSS);
  let css = generate_css()?;
  let (light, dark) = classic_vars(root)?;
  let bad = audit_inherited_text(&light, &dark)?;
  if !bad.is_empty() {
    bail!(
      "配色方案有 {} 处文字对比度不达标（调整 core/color_scheme.rs 里的种子色）：\n{}",
      bad.len(),
      bad.join("\n")
    );
  }

  if check {
    let on_disk = fs::read_to_string(&path).unwrap_or_default();
    if on_disk != css {
      bail!(
        "{} 与方案表不一致（过期或被手改过），请运行 `cargo make pixel-schemes`",
        super::SCHEMES_CSS
      );
    }
    println!(
      "配色方案：{} 个方案（含默认）全部达标，schemes.css 是最新的",
      SCHEMES.len()
    );
    return Ok(());
  }

  fs::write(&path, &css).with_context(|| path.display().to_string())?;
  println!(
    "配色方案：写入 {} 个方案（另有默认方案在 tokens.css）→ {}",
    generated().count(),
    super::SCHEMES_CSS
  );
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn css_has_one_light_and_one_dark_block_per_generated_scheme() {
    let css = generate_css().unwrap();
    for s in generated() {
      assert_eq!(
        css
          .matches(&format!("html[data-scheme=\"{}\"] {{", s.id))
          .count(),
        1,
        "{}",
        s.id
      );
      assert_eq!(
        css
          .matches(&format!("html[data-scheme=\"{}\"].dark {{", s.id))
          .count(),
        1,
        "{}",
        s.id
      );
    }
    assert!(
      !css.contains(&format!("data-scheme=\"{DEFAULT_SCHEME}\"")),
      "默认方案在 tokens.css 里，不应重复生成"
    );
  }

  #[test]
  fn generation_is_stable() {
    assert_eq!(generate_css().unwrap(), generate_css().unwrap());
  }

  #[test]
  fn dark_blocks_follow_their_light_blocks() {
    // 同优先级里后者胜出之前，亮色块（更低优先级的 `.dark` 之上）必须先于暗色块出现。
    let css = generate_css().unwrap();
    for s in generated() {
      let l = css
        .find(&format!("html[data-scheme=\"{}\"] {{", s.id))
        .unwrap();
      let d = css
        .find(&format!("html[data-scheme=\"{}\"].dark {{", s.id))
        .unwrap();
      assert!(l < d, "{}", s.id);
    }
  }

  #[test]
  fn every_declared_value_is_valid_css_color_syntax() {
    let css = generate_css().unwrap();
    let re = regex::Regex::new(r"^  (--[a-z0-9-]+): (#[0-9a-f]{6}|rgb\(\d+ \d+ \d+ / [0-9.]+\));$")
      .unwrap();
    for line in css.lines().filter(|l| l.starts_with("  --")) {
      assert!(re.is_match(line), "{line:?}");
    }
  }
}
