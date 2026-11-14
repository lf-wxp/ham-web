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

/// WCAG AA 正文下限。
const AA: f64 = 4.5;

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

/// 审计经典方案手写在 `tokens.css` 里的屏幕层：门槛与 core 里派生方案的单测同一套。
///
/// 派生方案由 `color_scheme` 的单测守着；经典方案的屏幕色是手写的，只有这里看得到。亮色（`:root`）是反射式液晶，
/// 暗色（`.dark`）是背光屏，两份各审一遍；状态色还要与 core 里对应的常量同值（各方案共用这一份）。
fn audit_classic_display(
  light: &BTreeMap<String, Rgb>,
  dark: &BTreeMap<String, Rgb>,
) -> Result<Vec<String>> {
  use ham_web_core::color_scheme::{
    DISPLAY_FG_MIN, DISPLAY_GHOST_RANGE, DISPLAY_STATUS, DISPLAY_STATUS_LCD,
  };
  let mut bad = Vec::new();
  for (mode, block, vars) in [("亮色", ":root", light), ("暗色", ".dark", dark)] {
    let var = |name: &str| -> Result<Rgb> {
      vars
        .get(name)
        .copied()
        .ok_or_else(|| anyhow!("tokens.css 的 {block} 里缺少 {name}"))
    };
    let bg = var("--display-bg")?;
    let short = |name: &str, fg: Rgb, min: f64| -> Option<String> {
      let r = fg.contrast(bg);
      (r < min).then(|| {
        format!(
          "[经典·{mode}] {name} {} 落在屏幕底 {} 上只有 {r:.2}（要 >= {min}）",
          fg.hex(),
          bg.hex()
        )
      })
    };
    bad.extend(short("--display-fg", var("--display-fg")?, DISPLAY_FG_MIN));
    bad.extend(short("--display-dim", var("--display-dim")?, AA));
    let status = if block == ".dark" {
      DISPLAY_STATUS
    } else {
      DISPLAY_STATUS_LCD
    };
    for (name, c) in status {
      let v = var(name)?;
      if v != c {
        bad.push(format!(
          "[经典·{mode}] {name} 与 color_scheme 里的状态色常量不一致"
        ));
      }
      bad.extend(short(name, v, AA));
    }
    let (lo, hi) = DISPLAY_GHOST_RANGE;
    let r = var("--display-ghost")?.contrast(bg);
    if !(lo..=hi).contains(&r) {
      bad.push(format!(
        "[经典·{mode}] --display-ghost 对屏幕底 {r:.2}，应在 {lo}–{hi}"
      ));
    }
  }
  Ok(bad)
}

/// 热力格子里相邻两档的最小对比度（WCAG 比值）。
///
/// 1.0 = 看不出差别。热力格子靠"同一色相、不同浓淡"传达多少，相邻档若分不开，
/// 四档就退化成两三档。当前表（10 / 18 / 24 / 30%）实测最小是 1.052（亮色「森林」的 `--cat-5`
/// 第 2→3 档），门槛 1.04 留一点余量：它拦的是"档位被调得挤成一团"，**不**保证肉眼一定好分辨，
/// 改表时仍要在浅 / 深色各挑一两套配色目测。
const HEAT_MIN_STEP: f64 = 1.04;

/// 热力审计的结果：违规清单，以及全局最小的相邻档对比度（用来标定 [`HEAT_MIN_STEP`]）。
struct HeatAudit {
  bad: Vec<String>,
  worst_step: f64,
  worst_step_at: String,
}

/// 审计「易错知识点」热力格子（`color-mix(分类色 N%, var(--card))` 上压 `--foreground` 文字）：
///
/// 1. **文字**：每个方案 × 明暗 × 8 个分类色 × 4 档，`--foreground` 落在混出来的底上要 >= AA；
/// 2. **浓淡**：相邻两档（含 0 档 = 纯卡片底）的底色对比度要 >= [`HEAT_MIN_STEP`]。
///
/// 两件事是一对矛盾 —— 浓度越高文字越难读，越低档位越难分 —— 所以放在一起审，
/// 改 `ham_web_core::mistake_book::heat_percent` 的表时由这里兜住两头。
/// 只覆盖像素风的 `--card` / `--foreground`（经典方案 + 生成的各方案）；现代风的取值不在这里。
fn audit_heat(light: &BTreeMap<String, Rgb>, dark: &BTreeMap<String, Rgb>) -> Result<HeatAudit> {
  let mut audit = HeatAudit {
    bad: Vec::new(),
    worst_step: f64::INFINITY,
    worst_step_at: String::new(),
  };
  // 经典方案不在生成物里，单独补上；其余方案在其基础上叠覆盖。
  let mut cases: Vec<(String, bool, BTreeMap<String, Rgb>)> = vec![
    (DEFAULT_SCHEME.to_owned(), false, light.clone()),
    (DEFAULT_SCHEME.to_owned(), true, dark.clone()),
  ];
  for scheme in generated() {
    for (is_dark, classic) in [(false, light), (true, dark)] {
      cases.push((
        scheme.id.to_owned(),
        is_dark,
        effective_vars(scheme, is_dark, classic)?,
      ));
    }
  }
  for (id, is_dark, vars) in &cases {
    let (Some(&card), Some(&fg)) = (vars.get("--card"), vars.get("--foreground")) else {
      bail!("{id}：缺少 --card 或 --foreground");
    };
    for n in 1..=8 {
      let name = format!("--cat-{n}");
      let Some(&cat) = vars.get(&name) else {
        bail!("{id}：缺少 {name}");
      };
      let mut prev = card;
      for level in 1..=4u8 {
        let pct = f64::from(ham_web_core::mistake_book::heat_percent(level)) / 100.0;
        let cell = card.mix(cat, pct);
        let text = fg.contrast(cell);
        if text < AA {
          audit.bad.push(format!(
            "[{}] {id}：{name} 第 {level} 档（{}%）底 {} 上 --foreground {} 只有 {text:.2}",
            mode_name(*is_dark),
            (pct * 100.0).round(),
            cell.hex(),
            fg.hex()
          ));
        }
        let step = prev.contrast(cell);
        if step < audit.worst_step {
          audit.worst_step = step;
          audit.worst_step_at = format!(
            "[{}] {id}：{name} 第 {} → {level} 档（{} → {}）",
            mode_name(*is_dark),
            level - 1,
            prev.hex(),
            cell.hex()
          );
        }
        if step < HEAT_MIN_STEP {
          audit.bad.push(format!(
            "[{}] {id}：{name} 第 {} → {level} 档（{} → {}）只差 {step:.3}，分不开",
            mode_name(*is_dark),
            level - 1,
            prev.hex(),
            cell.hex()
          ));
        }
        prev = cell;
      }
    }
  }
  Ok(audit)
}

/// `pixel-schemes`：生成 `schemes.css`；`check` 为真时只校验（过期或对比度不达标则失败）。
pub fn run(root: &Path, check: bool) -> Result<()> {
  let path = root.join(super::SCHEMES_CSS);
  let css = generate_css()?;
  let (light, dark) = classic_vars(root)?;
  let mut bad = audit_inherited_text(&light, &dark)?;
  bad.extend(audit_classic_display(&light, &dark)?);
  if !bad.is_empty() {
    bail!(
      "配色方案有 {} 处文字对比度不达标（调整 core/color_scheme.rs 里的种子色）：\n{}",
      bad.len(),
      bad.join("\n")
    );
  }
  let heat = audit_heat(&light, &dark)?;
  if !heat.bad.is_empty() {
    bail!(
      "热力格子有 {} 处不达标（调整 core/mistake_book.rs 的 heat_percent；文字对比度要 >= {AA}，相邻档要 >= {HEAT_MIN_STEP}）：\n{}",
      heat.bad.len(),
      heat.bad.join("\n")
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
    println!(
      "热力格子：文字对比度全部 >= {AA}；最紧的相邻档对比度 {:.3}（门槛 {HEAT_MIN_STEP}），在 {}",
      heat.worst_step, heat.worst_step_at
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
