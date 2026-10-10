//! 配色方案：每个方案有亮 / 暗两套「种子色」，由固定的派生规则展开成全套 CSS 变量。
//!
//! 为什么只存种子色：一个方案要覆盖三十来个语义变量（背景、卡片、弹层、次级面、静音字、描边、
//! 侧栏……）。逐个手填既容易漏，也没法保证每一对「字 / 底」都够对比度。这里每个方案只写
//! 5 个种子色，其余按规则算出来，并且**文字色按它实际会落到的每个表面校正**
//! （见 [`Scheme::tokens`]）—— 新增一个方案不会因为某个变量忘了配而在 axe 里翻车。
//!
//! 「经典」方案的变量手写在 `tokens.css` 里（它是整套像素风的基准），不由本模块生成；
//! 这里仍登记它的种子色，用于设置面板里的色块预览。其余方案由
//! `cargo make pixel-schemes` 生成 `style/pixel/schemes.css`。
//!
//! 与基准保持一致的有三件事：
//! - 亮色表面不低于经典亮色的最暗表面、暗色表面不高于经典暗色的最亮表面 ——
//!   共享的 Tailwind 色阶（`text-green-600` / `dark:text-red-400`）是按经典的表面校正的，
//!   新方案的表面越过这条线，那些色阶就会不够对比度（由 `pixel-check` 审计兜底）；
//! - 状态色（血条 / 经验条 / 金币 / 通关绿 / 危险红）是语义色，不随方案变；
//! - 方案只改「配色」，不改字体、圆角、阴影形状与动效。

use crate::color::Rgb;

/// 默认方案的 id（`tokens.css` 里的那一套）。
pub const DEFAULT_SCHEME: &str = "classic";

/// 一个方案在某个明暗下的种子色。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seeds {
  /// 页面底色。
  pub bg: Rgb,
  /// 正文与描边的墨色（暗色下是浅字）。
  pub ink: Rgb,
  /// 主色：主按钮、链接、选中态。
  pub primary: Rgb,
  /// 强调面：悬停 / 选中的底色（亮色下是明亮的高光色，暗色下是深色调）。
  pub accent: Rgb,
  /// 键盘焦点环。
  pub ring: Rgb,
}

/// 一个配色方案：亮 / 暗两套种子色。
#[derive(Clone, Copy, Debug)]
pub struct Scheme {
  /// 稳定 id：落在 `<html data-scheme>` 与 `localStorage`，改名会让用户的选择失效。
  pub id: &'static str,
  pub light: Seeds,
  pub dark: Seeds,
}

/// 派生出的一个变量值。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
  /// 实色。
  Color(Rgb),
  /// 带透明度的实色：`rgb(r g b / a)`。
  Alpha(Rgb, f64),
}

impl Value {
  /// CSS 写法。
  #[must_use]
  pub fn css(self) -> String {
    match self {
      Self::Color(c) => c.hex(),
      Self::Alpha(c, a) => format!("rgb({} {} {} / {a})", c.0, c.1, c.2),
    }
  }

  /// 实色值；带透明度的返回 `None`（对比度审计只看实色）。
  #[must_use]
  pub const fn color(self) -> Option<Rgb> {
    match self {
      Self::Color(c) => Some(c),
      Self::Alpha(..) => None,
    }
  }
}

/// 全部方案，`classic` 必须排第一（它是回退目标，也是设置面板里的第一项）。
pub const SCHEMES: &[Scheme] = &[
  Scheme {
    id: "classic",
    light: Seeds {
      bg: Rgb::from_u32(0xf2e8cf),
      ink: Rgb::from_u32(0x1a1c2c),
      primary: Rgb::from_u32(0x3b5dc9),
      accent: Rgb::from_u32(0xffcd75),
      ring: Rgb::from_u32(0x3b5dc9),
    },
    dark: Seeds {
      bg: Rgb::from_u32(0x1a1c2c),
      ink: Rgb::from_u32(0xf4f4f4),
      primary: Rgb::from_u32(0x41a6f6),
      accent: Rgb::from_u32(0x29366f),
      ring: Rgb::from_u32(0xffcd75),
    },
  },
  Scheme {
    id: "forest",
    light: Seeds {
      bg: Rgb::from_u32(0xe3eed5),
      ink: Rgb::from_u32(0x14281c),
      primary: Rgb::from_u32(0x1f6f48),
      accent: Rgb::from_u32(0xc9e58a),
      ring: Rgb::from_u32(0x1f6f48),
    },
    dark: Seeds {
      bg: Rgb::from_u32(0x101c16),
      ink: Rgb::from_u32(0xe8f2e6),
      primary: Rgb::from_u32(0x5fd49a),
      accent: Rgb::from_u32(0x1a3d2e),
      ring: Rgb::from_u32(0xc9e58a),
    },
  },
  Scheme {
    id: "ocean",
    light: Seeds {
      bg: Rgb::from_u32(0xdcecf2),
      ink: Rgb::from_u32(0x0f2233),
      primary: Rgb::from_u32(0x1a6a9a),
      accent: Rgb::from_u32(0xa4e3ea),
      ring: Rgb::from_u32(0x1a6a9a),
    },
    dark: Seeds {
      bg: Rgb::from_u32(0x0c1a2a),
      ink: Rgb::from_u32(0xe6f1f7),
      primary: Rgb::from_u32(0x5cc8f0),
      accent: Rgb::from_u32(0x17395a),
      ring: Rgb::from_u32(0x7ee8e8),
    },
  },
  Scheme {
    id: "sunset",
    light: Seeds {
      bg: Rgb::from_u32(0xf6e1d6),
      ink: Rgb::from_u32(0x2b1722),
      primary: Rgb::from_u32(0x7a3b8c),
      accent: Rgb::from_u32(0xffc99b),
      ring: Rgb::from_u32(0x7a3b8c),
    },
    dark: Seeds {
      bg: Rgb::from_u32(0x1f1626),
      ink: Rgb::from_u32(0xf6e8ee),
      primary: Rgb::from_u32(0xff8fb0),
      accent: Rgb::from_u32(0x4a2a4f),
      ring: Rgb::from_u32(0xffb86b),
    },
  },
  Scheme {
    id: "graphite",
    light: Seeds {
      bg: Rgb::from_u32(0xe9e9e5),
      ink: Rgb::from_u32(0x151515),
      primary: Rgb::from_u32(0x3a4256),
      accent: Rgb::from_u32(0xd4d4cc),
      ring: Rgb::from_u32(0x3a4256),
    },
    dark: Seeds {
      bg: Rgb::from_u32(0x121212),
      ink: Rgb::from_u32(0xececec),
      primary: Rgb::from_u32(0xcfd4dc),
      accent: Rgb::from_u32(0x2e2e32),
      ring: Rgb::from_u32(0xf0c24b),
    },
  },
];

/// 按 id 找方案。
#[must_use]
pub fn find(id: &str) -> Option<&'static Scheme> {
  SCHEMES.iter().find(|s| s.id == id)
}

/// 按 id 找方案；不认识（旧版本写入的、被手改过的）一律回退默认。
#[must_use]
pub fn resolve(id: &str) -> &'static Scheme {
  find(id).unwrap_or(&SCHEMES[0])
}

/// 暗色下最亮表面的相对亮度上限：经典方案的 `--secondary`（#333c57）。
///
/// 共享的 Tailwind 暗色文字档（`dark:text-red-400` 等）是在经典的暗色表面上校准到 >= 4.6 的；
/// 新方案的任何暗色表面比它更亮，那些色阶落上去就不够了。
pub const DARK_SURFACE_MAX_LUMINANCE: f64 = 0.0463;
/// 亮色下卡片的相对亮度下限：经典方案的 `--card`（#fff8e1，约 0.938）。
///
/// 色调底（`bg-X-500/20` 叠在卡片上）的对比度由它决定，卡片越暗，浅色文字档越容易不够。
pub const LIGHT_CARD_MIN_LUMINANCE: f64 = 0.93;
/// 亮色下其余表面（页面底 / 静音面 / 次级面 / 弹层）的相对亮度下限：经典方案最暗的 `--secondary`（约 0.712）。
pub const LIGHT_SURFACE_MIN_LUMINANCE: f64 = 0.71;

/// 浅字 / 深字的候选：自动挑对比度更高的那个。
const LIGHT_TEXT: Rgb = Rgb::from_u32(0xf4f4f4);
const DARK_TEXT: Rgb = Rgb::from_u32(0x1a1c2c);

/// 字色的最低要求：比 AA 的 4.5 留一点余量。
const MIN_TEXT: f64 = 4.8;

/// 在 `bg` 上读得最清楚的字色（浅 / 深二选一）；两个都不够 4.5 时返回 `Err`。
fn best_text_on(bg: Rgb, what: &str) -> Result<Rgb, String> {
  let best = [LIGHT_TEXT, DARK_TEXT]
    .into_iter()
    .max_by(|a, b| a.contrast(bg).total_cmp(&b.contrast(bg)))
    .unwrap_or(LIGHT_TEXT);
  let ratio = best.contrast(bg);
  if ratio < 4.5 {
    return Err(format!(
      "{what}：浅字与深字落在 {} 上都不到 4.5（最好 {ratio:.2}）",
      bg.hex()
    ));
  }
  Ok(best)
}

/// 把 `from` 向 `toward` 一格（1%）一格混合，直到在 `surfaces` 的每一个上都 >= `min`。
fn readable(from: Rgb, toward: Rgb, surfaces: &[Rgb], min: f64) -> Rgb {
  let mut t = 0.0_f64;
  let mut cur = from;
  while surfaces.iter().any(|s| cur.contrast(*s) < min) && t < 1.0 {
    t += 0.01;
    cur = from.mix(toward, t);
  }
  cur
}

impl Scheme {
  /// 某个明暗下的种子色。
  #[must_use]
  pub const fn seeds(&self, dark: bool) -> &Seeds {
    if dark { &self.dark } else { &self.light }
  }

  /// 展开成全套要覆盖的 CSS 变量（变量名 → 值），顺序固定。
  ///
  /// 只列**随方案变化**的变量；状态色（`--pxl-hp` / `--destructive` 等）不在其中，沿用 `tokens.css`。
  /// 亮 / 暗两侧返回的变量名集合必须完全一致 —— `schemes.css` 里亮色块的选择器优先级
  /// 高于 `.dark`，暗色下靠同优先级的 `.dark` 块把每一个都再盖回去（见单测）。
  ///
  /// # Errors
  /// 种子色选得不好、派生不出够对比度的字色时返回说明（生成器会把它当错误报出来，而不是静默出一个不可读的方案）。
  pub fn tokens(&self, dark: bool) -> Result<Vec<(&'static str, Value)>, String> {
    let s = self.seeds(dark);
    let (bg, ink) = (s.bg, s.ink);
    let white = Rgb::WHITE;

    // ---- 表面：亮色在纸色上叠白 / 压墨，暗色在夜色上叠主色（让面带上方案的色相）----
    let (card, popover, secondary, muted, field) = if dark {
      (
        bg.mix(s.primary, 0.10),
        bg.mix(s.primary, 0.14),
        bg.mix(s.primary, 0.17),
        bg.mix(s.primary, 0.14),
        bg.mix(Rgb::BLACK, 0.22),
      )
    } else {
      (
        bg.mix(white, 0.78),
        bg.mix(white, 0.90),
        bg.mix(ink, 0.04),
        bg.mix(ink, 0.018),
        bg.mix(white, 0.92),
      )
    };
    let surfaces = [bg, card, popover, secondary, muted, s.accent];

    // ---- 字色 ----
    // 静音字：从「墨色往底色靠 35%」出发，不够再往墨色回拉，直到在每个表面上都 >= MIN_TEXT。
    let muted_fg = readable(ink.mix(bg, 0.35), ink, &surfaces, MIN_TEXT);
    let primary_fg = best_text_on(s.primary, "主色上的字")?;
    let accent_fg = best_text_on(s.accent, "强调面上的字")?;

    // ---- 描边 ----
    // 暗色下描边用「墨色往底色靠 30%」的灰；亮色下直接用墨色。
    let edge = ink.mix(bg, 0.30);
    let (input, border, pxl_ink) = if dark {
      (edge, Value::Alpha(edge, 0.26), bg.mix(edge, 0.55))
    } else {
      (ink, Value::Alpha(ink, 0.22), ink)
    };

    let c = Value::Color;
    let mut out = vec![
      ("--background", c(bg)),
      ("--foreground", c(ink)),
      ("--card", c(card)),
      ("--card-foreground", c(ink)),
      ("--popover", c(popover)),
      ("--popover-foreground", c(ink)),
      ("--primary", c(s.primary)),
      ("--primary-foreground", c(primary_fg)),
      ("--secondary", c(secondary)),
      ("--secondary-foreground", c(ink)),
      ("--muted", c(muted)),
      ("--muted-foreground", c(muted_fg)),
      ("--accent", c(s.accent)),
      ("--accent-foreground", c(accent_fg)),
      ("--border", border),
      ("--input", c(input)),
      ("--ring", c(s.ring)),
      ("--chart-1", c(s.primary)),
      (
        "--sidebar",
        c(if dark { bg.mix(white, 0.03) } else { muted }),
      ),
      ("--sidebar-foreground", c(ink)),
      ("--sidebar-primary", c(s.primary)),
      ("--sidebar-primary-foreground", c(primary_fg)),
      ("--sidebar-accent", c(s.accent)),
      ("--sidebar-accent-foreground", c(accent_fg)),
      ("--sidebar-border", border),
      ("--sidebar-ring", c(s.ring)),
      ("--pxl-ink", c(pxl_ink)),
      ("--pxl-field", c(field)),
    ];
    // 亮色下的抖动点阵与像素窗口内阴影是「墨色 + 透明度」，要跟着墨色走；
    // 暗色下它们是白 / 黑 + 透明度，与方案无关，沿用 `tokens.css`。
    if !dark {
      out.push(("--pxl-lo", Value::Alpha(ink, 0.2)));
      out.push(("--pxl-dither", Value::Alpha(ink, 0.055)));
    }
    Ok(out)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::BTreeSet;

  fn names(tokens: &[(&'static str, Value)]) -> BTreeSet<&'static str> {
    tokens.iter().map(|(n, _)| *n).collect()
  }

  fn get(tokens: &[(&'static str, Value)], name: &str) -> Rgb {
    tokens
      .iter()
      .find(|(n, _)| *n == name)
      .and_then(|(_, v)| v.color())
      .unwrap_or_else(|| panic!("{name} 不是实色或不存在"))
  }

  #[test]
  fn ids_are_unique_kebab_case_and_classic_comes_first() {
    assert_eq!(SCHEMES[0].id, DEFAULT_SCHEME);
    let ids: BTreeSet<_> = SCHEMES.iter().map(|s| s.id).collect();
    assert_eq!(ids.len(), SCHEMES.len(), "id 重复");
    for id in ids {
      assert!(
        !id.is_empty() && id.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
        "{id:?} 要是小写 kebab-case：它会直接写进 CSS 属性选择器"
      );
    }
  }

  #[test]
  fn unknown_ids_fall_back_to_the_default() {
    assert_eq!(resolve("forest").id, "forest");
    assert_eq!(resolve("no-such-scheme").id, DEFAULT_SCHEME);
    assert_eq!(resolve("").id, DEFAULT_SCHEME);
    assert!(find("no-such-scheme").is_none());
  }

  #[test]
  fn every_scheme_derives_without_error_in_both_modes() {
    for s in SCHEMES {
      for dark in [false, true] {
        s.tokens(dark)
          .unwrap_or_else(|e| panic!("{} dark={dark}: {e}", s.id));
      }
    }
  }

  #[test]
  fn light_and_dark_override_the_same_variables_except_the_ink_tinted_extras() {
    // 亮色块的选择器优先级高于 `.dark`：暗色下若有变量只在亮色块里出现，就会漏到暗色里。
    // 唯二的例外是 `--pxl-lo` / `--pxl-dither`，它们在 `.dark` 里本来就没有「墨色版」。
    for s in SCHEMES {
      let (l, d) = (
        names(&s.tokens(false).unwrap()),
        names(&s.tokens(true).unwrap()),
      );
      let only_light: BTreeSet<_> = l.difference(&d).copied().collect();
      assert_eq!(
        only_light,
        BTreeSet::from(["--pxl-dither", "--pxl-lo"]),
        "{}",
        s.id
      );
      assert!(
        d.is_subset(&l),
        "{}: 暗色多出变量 {:?}",
        s.id,
        d.difference(&l).collect::<Vec<_>>()
      );
    }
  }

  #[test]
  fn body_and_secondary_text_clear_aa_on_every_surface() {
    for s in SCHEMES {
      for dark in [false, true] {
        let t = s.tokens(dark).unwrap();
        let surfaces: Vec<(&str, Rgb)> = [
          "--background",
          "--card",
          "--popover",
          "--secondary",
          "--muted",
        ]
        .iter()
        .map(|n| (*n, get(&t, n)))
        .collect();
        for (name, bg) in &surfaces {
          for fg in ["--foreground", "--card-foreground", "--muted-foreground"] {
            let r = get(&t, fg).contrast(*bg);
            assert!(
              r >= 4.5,
              "{} dark={dark}: {fg} 落在 {name} 上只有 {r:.2}",
              s.id
            );
          }
        }
        let accent = get(&t, "--accent");
        let r = get(&t, "--muted-foreground").contrast(accent);
        assert!(
          r >= 4.5,
          "{} dark={dark}: 静音字落在强调面上只有 {r:.2}",
          s.id
        );
        let r = get(&t, "--accent-foreground").contrast(accent);
        assert!(r >= 4.5, "{} dark={dark}: 强调面上的字只有 {r:.2}", s.id);
        let r = get(&t, "--primary-foreground").contrast(get(&t, "--primary"));
        assert!(r >= 4.5, "{} dark={dark}: 主色上的字只有 {r:.2}", s.id);
      }
    }
  }

  #[test]
  fn the_primary_color_reads_as_text_on_the_page_surfaces() {
    // `text-primary` 被大量用作链接 / 强调字。
    for s in SCHEMES {
      for dark in [false, true] {
        let t = s.tokens(dark).unwrap();
        for n in ["--background", "--card", "--popover"] {
          let r = get(&t, "--primary").contrast(get(&t, n));
          assert!(r >= 4.5, "{} dark={dark}: 主色落在 {n} 上只有 {r:.2}", s.id);
        }
      }
    }
  }

  #[test]
  fn focus_ring_and_input_borders_are_visible_against_the_page() {
    // WCAG 1.4.11：界面部件的边界 / 焦点指示至少 3:1。
    for s in SCHEMES {
      for dark in [false, true] {
        let t = s.tokens(dark).unwrap();
        let bg = get(&t, "--background");
        for n in ["--ring", "--input"] {
          let r = get(&t, n).contrast(bg);
          assert!(r >= 3.0, "{} dark={dark}: {n} 对页面底色只有 {r:.2}", s.id);
        }
      }
    }
  }

  #[test]
  fn surfaces_stay_inside_the_envelope_the_shared_palette_was_tuned_for() {
    for s in SCHEMES {
      let t = s.tokens(true).unwrap();
      for n in [
        "--background",
        "--card",
        "--popover",
        "--secondary",
        "--muted",
        "--accent",
      ] {
        let l = get(&t, n).luminance();
        assert!(
          l <= DARK_SURFACE_MAX_LUMINANCE,
          "{} 暗色 {n} 太亮（{l:.4} > {DARK_SURFACE_MAX_LUMINANCE}）：共享色阶会不够对比度",
          s.id
        );
      }
      let t = s.tokens(false).unwrap();
      let card = get(&t, "--card").luminance();
      assert!(
        card >= LIGHT_CARD_MIN_LUMINANCE,
        "{} 亮色 --card 太暗（{card:.3} < {LIGHT_CARD_MIN_LUMINANCE}）",
        s.id
      );
      for n in ["--background", "--popover", "--secondary", "--muted"] {
        let l = get(&t, n).luminance();
        assert!(
          l >= LIGHT_SURFACE_MIN_LUMINANCE,
          "{} 亮色 {n} 太暗（{l:.3} < {LIGHT_SURFACE_MIN_LUMINANCE}）",
          s.id
        );
      }
    }
  }

  #[test]
  fn dark_schemes_are_dark_and_light_schemes_are_light() {
    for s in SCHEMES {
      assert!(s.light.bg.luminance() > 0.5, "{} 亮色底不够亮", s.id);
      assert!(s.dark.bg.luminance() < 0.05, "{} 暗色底不够暗", s.id);
    }
  }

  #[test]
  fn alpha_values_print_as_space_separated_rgb() {
    assert_eq!(
      Value::Alpha(Rgb(26, 28, 44), 0.22).css(),
      "rgb(26 28 44 / 0.22)"
    );
    assert_eq!(Value::Color(Rgb(255, 205, 117)).css(), "#ffcd75");
  }

  #[test]
  fn derivation_is_deterministic() {
    for s in SCHEMES {
      assert_eq!(s.tokens(true), s.tokens(true));
    }
  }
}
