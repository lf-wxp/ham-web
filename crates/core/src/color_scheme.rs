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
  // 以下几套取自经典像素调色板（PICO-8 / Game Boy / NES / C64 / CGA / ZX Spectrum）。
  // 种子色不是原样照搬：原调色板里的亮色当「主色」时往往过不了正文对比度，
  // 这里按同一套派生规则把亮暗两侧都调到可通过审计的范围内，色相本身保留。
  // id 只用小写字母与连字符（见单测），所以 PICO-8 记作 `pico`、C64 记作 `commodore`。
  Scheme {
    id: "pico",
    light: Seeds {
      bg: Rgb::from_u32(0xfff1e8),
      ink: Rgb::from_u32(0x1d2b53),
      primary: Rgb::from_u32(0x7e2553),
      accent: Rgb::from_u32(0xffec27),
      ring: Rgb::from_u32(0x7e2553),
    },
    dark: Seeds {
      bg: Rgb::from_u32(0x131c38),
      ink: Rgb::from_u32(0xfff1e8),
      primary: Rgb::from_u32(0x29adff),
      accent: Rgb::from_u32(0x331d3f),
      ring: Rgb::from_u32(0xffec27),
    },
  },
  Scheme {
    id: "gameboy",
    light: Seeds {
      bg: Rgb::from_u32(0xe8f8d0),
      ink: Rgb::from_u32(0x0f2a12),
      primary: Rgb::from_u32(0x2f6b2f),
      accent: Rgb::from_u32(0xc4e58a),
      ring: Rgb::from_u32(0x2f6b2f),
    },
    dark: Seeds {
      bg: Rgb::from_u32(0x071409),
      ink: Rgb::from_u32(0xe8f2e6),
      primary: Rgb::from_u32(0x9bbc0f),
      accent: Rgb::from_u32(0x1b2e18),
      ring: Rgb::from_u32(0xc4e58a),
    },
  },
  Scheme {
    id: "nes",
    light: Seeds {
      bg: Rgb::from_u32(0xfcf8f0),
      ink: Rgb::from_u32(0x000000),
      primary: Rgb::from_u32(0xd82800),
      accent: Rgb::from_u32(0xffc81e),
      ring: Rgb::from_u32(0xd82800),
    },
    dark: Seeds {
      bg: Rgb::from_u32(0x101018),
      ink: Rgb::from_u32(0xf4f4f4),
      primary: Rgb::from_u32(0x3cbcfc),
      accent: Rgb::from_u32(0x241635),
      ring: Rgb::from_u32(0xf8b800),
    },
  },
  Scheme {
    id: "commodore",
    light: Seeds {
      bg: Rgb::from_u32(0xeae8e0),
      ink: Rgb::from_u32(0x14142b),
      primary: Rgb::from_u32(0x2e2c9b),
      accent: Rgb::from_u32(0x9fd7d0),
      ring: Rgb::from_u32(0x2e2c9b),
    },
    dark: Seeds {
      bg: Rgb::from_u32(0x14142b),
      ink: Rgb::from_u32(0xece9ff),
      primary: Rgb::from_u32(0xb8c4ff),
      accent: Rgb::from_u32(0x22223f),
      ring: Rgb::from_u32(0xedf171),
    },
  },
  Scheme {
    id: "cga",
    light: Seeds {
      bg: Rgb::from_u32(0xeefaf8),
      ink: Rgb::from_u32(0x101010),
      primary: Rgb::from_u32(0xc02060),
      accent: Rgb::from_u32(0x7ef9f9),
      ring: Rgb::from_u32(0xc02060),
    },
    dark: Seeds {
      bg: Rgb::from_u32(0x0a0a14),
      ink: Rgb::from_u32(0xf4f4f4),
      primary: Rgb::from_u32(0x55ffff),
      accent: Rgb::from_u32(0x281430),
      ring: Rgb::from_u32(0xff55ff),
    },
  },
  Scheme {
    id: "spectrum",
    light: Seeds {
      bg: Rgb::from_u32(0xf2f2ea),
      ink: Rgb::from_u32(0x0a0a12),
      primary: Rgb::from_u32(0x0000d8),
      accent: Rgb::from_u32(0xffe94a),
      ring: Rgb::from_u32(0x0000d8),
    },
    dark: Seeds {
      bg: Rgb::from_u32(0x0b0b16),
      ink: Rgb::from_u32(0xf4f4f4),
      primary: Rgb::from_u32(0xffe94a),
      accent: Rgb::from_u32(0x221033),
      ring: Rgb::from_u32(0xffe94a),
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

/// 控件 / 窗口描边（`--pxl-ink`）对它所在表面的最低要求：WCAG 1.4.11（非文本对比度）的 3:1。
///
/// 为什么单独列：像素风的核心是那圈 2px 墨框。亮色下墨框是近黑，对卡片 13–20:1；暗色下它曾是
/// 「底色往边色靠 55%」的一条灰，对卡片只有 2.6–2.9、对按钮底最低 2.0 —— 同一套设计，暗色的
/// 「框」薄得只剩内高光在撑。这里按暗色里最亮的几个表面把它推到够 3:1 为止。
const MIN_EDGE: f64 = 3.0;

/// 屏幕读数（`--display-fg`）对屏幕底的最低要求：读数是一屏里最该一眼读出的字，按 AAA 的 7:1 给。
pub const DISPLAY_FG_MIN: f64 = 7.0;

/// 暗色（背光）屏幕上的三个状态色：不随方案变，与 `tokens.css` 的 `.dark` 同值（`--display-hot` / `--display-alarm` /
/// `--display-ok`）。列在这里是为了让单测检查它们落在每个方案的暗色屏幕底上都够 4.5。
pub const DISPLAY_STATUS: [(&str, Rgb); 3] = [
  ("--display-hot", Rgb::from_u32(0xffcd75)),
  ("--display-alarm", Rgb::from_u32(0xef7d57)),
  ("--display-ok", Rgb::from_u32(0xa7f070)),
];

/// 幽灵段（未点亮的七段 / 电平格）对屏幕底的对比度区间：低于下限看不出「这里有一段」，
/// 高于上限会被读成「亮着」。
pub const DISPLAY_GHOST_RANGE: (f64, f64) = (1.15, 1.8);

/// 亮色屏幕（反射式液晶）：屏幕底从**本方案自己的纸色**压深一档得来，不再往主色里混。
///
/// 为什么不用主色：主色在不少方案里和纸色相差很远（石墨是暖白纸 + 冷蓝主色、CGA 是薄荷纸 +
/// 品红主色），混 16% 就足以把屏推成一块**异色补丁**（中性灰 / 粉 / 淡紫）—— 用户反馈
/// 「某些配色下屏幕背景还是比较突兀」正是这几套。液晶是一块**同族**的反射面板：
/// 纸色压深、色相不动，主色只用来给极轻的色偏（[`LCD_PRIMARY_TINT`]）。
const LCD_DEEPEN: f64 = 0.07;

/// 主色给屏幕底的色偏：足够让每套方案「看得出是这套」，又不会压过纸色的色相。
const LCD_PRIMARY_TINT: f64 = 0.05;

/// 屏幕底与卡片色的色相最多差多少度。纸色接近中性（饱和度 ~0.01）时，几个百分点的主色就能把
/// 色相整个拽走（Spectrum 的暖白卡片 + 纯蓝主色 → 一块淡紫屏）；超过这条线就把主色的色偏收掉，
/// 只留纸色的明度关系 —— 液晶是一块**同族**的反射面板，不是另一种颜色。
const LCD_MAX_HUE_DRIFT: f64 = 12.0;

/// 色相角（0–360）。中性色（最大与最小通道相等）返回 `None`。
fn hue(c: Rgb) -> Option<f64> {
  let max = c.0.max(c.1).max(c.2);
  let min = c.0.min(c.1).min(c.2);
  if max == min {
    return None;
  }
  let d = f64::from(max - min);
  let (r, g, b) = (f64::from(c.0), f64::from(c.1), f64::from(c.2));
  let h = if max == c.0 {
    60.0 * (((g - b) / d) % 6.0)
  } else if max == c.1 {
    60.0 * ((b - r) / d + 2.0)
  } else {
    60.0 * ((r - g) / d + 4.0)
  };
  Some(if h < 0.0 { h + 360.0 } else { h })
}

/// 两个色相的夹角（0–180）。只看**看得出来的颜色**：两边都几乎中性时返回 0。
///
/// 为什么不是直接比色相角：卡片的纸色常常淡到几乎中性（`#fafaf8` 的饱和度 ~0.01），这时色相角
/// 是噪声 —— 差 15° 看着刺眼，肉眼看却是同一个暖白。判据要问的是「屏和卡片看起来是不是两种颜色」，
/// 所以两边里**更饱和的那个**得先过一道可见门槛，否则不构成色相冲突。
fn hue_gap(a: Rgb, b: Rgb) -> f64 {
  let (Some(ha), Some(hb)) = (hue(a), hue(b)) else {
    return 0.0;
  };
  const VISIBLE_SAT: f64 = 0.06;
  if saturation(a).max(saturation(b)) < VISIBLE_SAT {
    return 0.0;
  }
  let d = (ha - hb).abs() % 360.0;
  d.min(360.0 - d)
}

/// HSV 饱和度（0 = 中性，1 = 纯色）。
fn saturation(c: Rgb) -> f64 {
  let max = c.0.max(c.1).max(c.2);
  let min = c.0.min(c.1).min(c.2);
  if max == 0 {
    return 0.0;
  }
  f64::from(max - min) / f64::from(max)
}

/// 亮色屏幕对它所在的亮色卡片的对比度区间：下限是「看得出这是一块屏」，上限是「别暗成补丁」。
/// 部件的边界另有 2px 墨框（墨框对屏幕底 >= 3:1）。
pub const LCD_CARD_RANGE: (f64, f64) = (1.08, 1.22);

/// 暗色屏幕（背光屏）：从**本方案的卡片色**压深一档得来 —— 卡片是「机身面板」，屏幕比它暗一层，
/// 像面板上嵌着的一块玻璃。不再和卡片同色（那样屏幕根本不存在，一片糊），也不再像经典方案那样
/// 比卡片暗一大截（1.55 的对比像挖了个洞）。
const BACKLIGHT_DEEPEN: f64 = 0.30;

/// 暗色屏幕对暗色卡片的对比度区间：下限「看得出是另一块材质」，上限「别成了黑洞」。
pub const BACKLIGHT_CARD_RANGE: (f64, f64) = (1.12, 1.34);

/// 亮色屏幕（液晶）上的三个状态色：深色、不随方案变，与 `tokens.css` 的 `:root` 同值（暗色屏幕用 [`DISPLAY_STATUS`]）。
/// 不随方案派生是为了不让每个方案块都再写一遍（样式表预算）；单测保证它们落在每个方案的液晶底上都够 4.5。
pub const DISPLAY_STATUS_LCD: [(&str, Rgb); 3] = [
  ("--display-hot", Rgb::from_u32(0x754500)),
  ("--display-alarm", Rgb::from_u32(0x932937)),
  ("--display-ok", Rgb::from_u32(0x1a5b31)),
];

/// 暗色墨框：从「墨色往底色靠 30%」的灰出发，往墨色推，直到在每个会被它描边的表面上都 >= [`MIN_EDGE`]。
/// 与 [`Scheme::tokens`] 里 `--pxl-ink` 的算法一致（那边还要照顾按钮底 / 输入底，这里只需照顾屏幕）。
fn pxl_ink(d: &Seeds) -> Rgb {
  let edge = d.ink.mix(d.bg, 0.30);
  let frame = d.bg.mix(edge, 0.55);
  readable(frame, d.ink, &[d.bg], MIN_EDGE)
}

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

  /// 屏幕层（`--display-*`）：两套都从自己的种子派生、都带本方案的主色色相 —— 暗色下是背光屏
  /// （夜色底混主色、荧光读数），亮色下是反射式液晶（卡片色混主色、墨色段码），亮度贴近纸面。
  /// 白天看液晶、夜里开背光（`docs/ui/decisions.md` §11.6）。
  /// 经典方案的两套屏幕色手写在 `tokens.css`（暗色是青色荧光），不走这里。扫描线与内边框（`--display-scan` /
  /// `--display-rim`）在 `tokens.css` 里用 `color-mix` 从屏幕底算，不按方案逐个写。
  #[must_use]
  pub fn display_tokens(&self, dark: bool) -> Vec<(&'static str, Value)> {
    if dark {
      // 读数取暗色主色、不够 7:1 就往白色推；幽灵段是读数 16% 混进屏幕底。
      let d = &self.dark;
      let card = d.bg.mix(d.primary, 0.10);
      // 屏幕比卡片暗一层：往黑压（保住色相），一档一档收到区间内。
      let (lo, hi) = BACKLIGHT_CARD_RANGE;
      let mut deepen = BACKLIGHT_DEEPEN;
      let mut bg = card.mix(Rgb::BLACK, deepen);
      while bg.contrast(card) < lo && deepen < 0.9 {
        deepen += 0.02;
        bg = card.mix(Rgb::BLACK, deepen);
      }
      while bg.contrast(card) > hi && deepen > 0.02 {
        deepen -= 0.02;
        bg = card.mix(Rgb::BLACK, deepen);
      }
      let fg = readable(
        d.primary.mix(Rgb::WHITE, 0.2),
        Rgb::WHITE,
        &[bg],
        DISPLAY_FG_MIN,
      );
      let dim = readable(d.ink.mix(bg, 0.4), d.ink, &[bg], MIN_TEXT);
      return vec![
        ("--display-bg", Value::Color(bg)),
        ("--display-fg", Value::Color(fg)),
        ("--display-ghost", Value::Color(bg.mix(fg, 0.16))),
        ("--display-dim", Value::Color(dim)),
        // 暗色背光屏的外框仍是墨框（`--pxl-ink`）：深底上那圈亮线就是机身轮廓，
        // 与 `tokens.css` 的 `.dark` 同值 —— 派生到这里只是为了亮 / 暗两侧变量名对齐。
        ("--display-edge", Value::Color(pxl_ink(d))),
      ];
    }
    // 段码是墨色、不够 7:1 往黑推；幽灵段从很淡开始一格一格加深，刚好够「看得出这里有一段」为止。
    let l = &self.light;
    let card = l.bg.mix(Rgb::WHITE, 0.78);
    // 屏幕底 = 纸色 + 主色极轻的色偏（先定色相），再压深一档（只动明度）：同族、不发紫、不发粉。
    // 压深量按区间反解：主色那 5% 本身就在压深，从固定值往回退会退不到（经典方案的墨色很深）。
    // 纸色越中性，主色越容易把色相整个拽走：先按色相上限收色偏（中性纸色按满收）。
    // 收到上限的 80% 再停：压深那一步还会再挪一点色相，留出余量才不会停在阈值外。
    let mut tint = LCD_PRIMARY_TINT;
    while hue_gap(l.bg.mix(l.primary, tint), l.bg) > LCD_MAX_HUE_DRIFT * 0.8 && tint > 0.0 {
      tint = (tint - 0.005).max(0.0);
    }
    let base = l.bg.mix(l.primary, tint);
    let (lo, hi) = LCD_CARD_RANGE;
    let mut deepen = LCD_DEEPEN;
    let mut bg = base.mix(l.ink, deepen);
    while bg.contrast(card) > hi && deepen > 0.0 {
      deepen = (deepen - 0.005).max(0.0);
      bg = base.mix(l.ink, deepen);
    }
    // 退到 0 还不够（主色自己就把屏压得比卡片暗太多）：把主色的色偏也收掉，只留纸色压深。
    while bg.contrast(card) > hi && tint > 0.0 {
      tint = (tint - 0.01).max(0.0);
      bg = l.bg.mix(l.primary, tint).mix(l.ink, deepen);
    }
    while bg.contrast(card) < lo && deepen < 0.9 {
      deepen += 0.005;
      bg = l.bg.mix(l.primary, tint).mix(l.ink, deepen);
    }
    let fg = readable(l.ink, Rgb::BLACK, &[bg], DISPLAY_FG_MIN);
    // 次要字是段码色往屏幕底靠一半：液晶上的「暗一点的字」，不是另一套灰。
    let dim = readable(fg.mix(bg, 0.5), fg, &[bg], MIN_TEXT);
    // 幽灵段要同时守住区间两端：太淡看不出「这里有一段」，太亮会被读成「亮着」。
    let (ghost_lo, ghost_hi) = DISPLAY_GHOST_RANGE;
    let mut t = 0.09;
    while bg.mix(fg, t).contrast(bg) < ghost_lo + 0.05 && t < 0.5 {
      t += 0.01;
    }
    // 上限也在这里守：墨色深的方案从 0.09 起就可能越线，往回收一档，而不是留给单测去发现。
    while bg.mix(fg, t).contrast(bg) > ghost_hi && t > 0.01 {
      t -= 0.01;
    }
    let edge = readable(bg.mix(fg, 0.55), fg, &[bg], MIN_EDGE);
    vec![
      ("--display-bg", Value::Color(bg)),
      ("--display-fg", Value::Color(fg)),
      ("--display-ghost", Value::Color(bg.mix(fg, t))),
      ("--display-dim", Value::Color(dim)),
      // 屏幕外框：亮色下取「屏幕底再深一档」，不用纸面的纯黑墨框 —— 浅灰屏配 2px 近黑框像贴上去的
      // 补丁（`docs/ui/decisions.md` §11.6）。仍要够 3:1（WCAG 1.4.11 的部件边界）。
      ("--display-edge", Value::Color(edge)),
    ]
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
      // 墨框：从原来的灰出发往墨色推，直到在每个会被它描边的表面上都 >= MIN_EDGE。
      let frame = readable(
        bg.mix(edge, 0.55),
        ink,
        &[bg, card, popover, secondary, muted, field],
        MIN_EDGE,
      );
      (edge, Value::Alpha(edge, 0.26), frame)
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
    // 屏幕层亮暗两块都写（亮色是液晶、暗色是背光屏）：守住「两块覆盖同一组变量」这条约定（见单测）。
    out.extend(self.display_tokens(dark));
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
  fn the_ink_frame_clears_non_text_contrast_on_every_surface_it_outlines() {
    // 亮色一直是近黑墨框；暗色曾只有 2.0–2.9:1（见 `MIN_EDGE`）。
    for s in SCHEMES {
      for dark in [false, true] {
        let t = s.tokens(dark).unwrap();
        let ink = get(&t, "--pxl-ink");
        for name in [
          "--background",
          "--card",
          "--popover",
          "--secondary",
          "--muted",
          "--pxl-field",
        ] {
          let r = ink.contrast(get(&t, name));
          assert!(
            r >= MIN_EDGE,
            "{} dark={dark}: 墨框落在 {name} 上只有 {r:.2}",
            s.id
          );
        }
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
  fn display_tokens_follow_the_mode() {
    // 亮色是反射式液晶、暗色是背光屏：两套各自从对应明暗的种子派生，展开的全套变量里也是这一份。
    for s in SCHEMES {
      for dark in [false, true] {
        let picked: Vec<(&'static str, Value)> = s
          .tokens(dark)
          .unwrap()
          .into_iter()
          .filter(|(n, _)| n.starts_with("--display-"))
          .collect();
        assert_eq!(picked, s.display_tokens(dark), "{} dark={dark}", s.id);
      }
    }
  }

  #[test]
  fn display_readouts_and_status_colors_read_on_every_screen() {
    for s in SCHEMES {
      for dark in [false, true] {
        let t = s.display_tokens(dark);
        let bg = get(&t, "--display-bg");
        let r = get(&t, "--display-fg").contrast(bg);
        assert!(
          r >= DISPLAY_FG_MIN,
          "{} dark={dark}: 屏幕读数只有 {r:.2}",
          s.id
        );
        let r = get(&t, "--display-dim").contrast(bg);
        assert!(r >= 4.5, "{} dark={dark}: 屏幕次要字只有 {r:.2}", s.id);
        let status = if dark {
          DISPLAY_STATUS
        } else {
          DISPLAY_STATUS_LCD
        };
        for (name, c) in status {
          let r = c.contrast(bg);
          assert!(
            r >= 4.5,
            "{} dark={dark}: {name} 落在屏幕底上只有 {r:.2}",
            s.id
          );
        }
      }
    }
  }

  #[test]
  fn display_ghost_segments_read_as_unlit_but_visible() {
    let (lo, hi) = DISPLAY_GHOST_RANGE;
    for s in SCHEMES {
      for dark in [false, true] {
        let t = s.display_tokens(dark);
        let r = get(&t, "--display-ghost").contrast(get(&t, "--display-bg"));
        assert!(
          (lo..=hi).contains(&r),
          "{} dark={dark}: 幽灵段对屏幕底 {r:.2}，应在 {lo}–{hi}",
          s.id
        );
      }
    }
  }

  #[test]
  fn screens_stay_in_the_same_color_family_as_their_panel() {
    // 用户反馈「某些配色下亮 / 暗主题里屏幕背景还是比较突兀」。根因是屏幕底曾经往主色里混 16%：
    // 主色与纸色色相远时（石墨的暖白纸 + 冷蓝主色、CGA 的薄荷纸 + 品红主色）屏幕就成了一块异色补丁。
    // 现在屏幕底只从**本方案自己的纸色 / 卡片色**派生，主色只给限幅的色偏。
    // 两条判据：① 与卡片色相差 <= `LCD_MAX_HUE_DRIFT`；② 对比度落在各自区间内（不是同色、也不是黑洞）。
    for s in SCHEMES {
      for dark in [false, true] {
        let t = s.display_tokens(dark);
        let base = s.tokens(dark).unwrap();
        let screen = get(&t, "--display-bg");
        let card = get(&base, "--card");
        let gap = hue_gap(screen, card);
        // 卡片本身接近中性时色相没有意义，`hue_gap` 会返回 0。
        assert!(
          gap <= LCD_MAX_HUE_DRIFT + 0.5,
          "{}（dark={dark}）：屏幕 {} 与卡片 {} 色相差 {gap:.0}°",
          s.id,
          screen.hex(),
          card.hex()
        );
        let (lo, hi) = if dark {
          BACKLIGHT_CARD_RANGE
        } else {
          LCD_CARD_RANGE
        };
        let r = screen.contrast(card);
        assert!(
          (lo..=hi).contains(&r),
          "{}（dark={dark}）：屏幕对卡片 {r:.2}，应在 {lo}–{hi}",
          s.id
        );
        // 读数仍要好读：屏幕再好看，字读不清也没用。
        let fg = get(&t, "--display-fg");
        assert!(
          fg.contrast(screen) >= DISPLAY_FG_MIN,
          "{}（dark={dark}）：读数对屏幕只有 {:.2}",
          s.id,
          fg.contrast(screen)
        );
        // 判据是色相（不是饱和度）：同族面可以比卡片更饱和 —— Game Boy 的纸色几乎中性、
        // 屏幕是明确的橄榄绿，但它和卡片同色相（86° vs 83°），看着就是「这块屏偏绿」，不突兀。
        // 突兀的是**另一种颜色**：淡紫 / 粉 / 冷蓝的屏配暖白卡片。
      }
    }
  }

  #[test]
  fn the_screen_frame_reads_against_the_screen() {
    // 屏幕外框是部件边界（WCAG 1.4.11）：亮色下它是屏幕底压深的一档（不是纸面纯黑），暗色下是墨框。
    // 无论哪套，框对屏幕底都得 >= 3:1 —— 否则「这一块是屏」这条边界就没了。
    for s in SCHEMES {
      for dark in [false, true] {
        let t = s.display_tokens(dark);
        let screen = get(&t, "--display-bg");
        let edge = get(&t, "--display-edge");
        let r = edge.contrast(screen);
        assert!(
          r >= MIN_EDGE,
          "{}（dark={dark}）：屏幕边框 {} 对屏幕 {} 只有 {r:.2}",
          s.id,
          edge.hex(),
          screen.hex()
        );
      }
    }
  }

  #[test]
  fn the_light_screen_is_a_soft_tinted_panel() {
    // 亮色液晶要比卡片暗一档、读得出「另一块材质」，又不能暗成补丁；部件边界靠墨框（>= 3:1）。
    let (lo, hi) = LCD_CARD_RANGE;
    for s in SCHEMES {
      let screen = get(&s.display_tokens(false), "--display-bg");
      let light = s.tokens(false).unwrap();
      let card = get(&light, "--card");
      assert!(
        screen.luminance() < card.luminance(),
        "{}: 液晶不比卡片暗",
        s.id
      );
      let r = screen.contrast(card);
      assert!(
        (lo..=hi).contains(&r),
        "{}: 液晶对卡片 {r:.2}，应在 {lo}–{hi}",
        s.id
      );
      assert!(screen.luminance() > 0.5, "{}: 液晶暗成了补丁", s.id);
      let r = get(&light, "--pxl-ink").contrast(screen);
      assert!(r >= MIN_EDGE, "{}: 墨框对液晶只有 {r:.2}", s.id);
    }
  }

  #[test]
  fn screens_carry_each_schemes_own_hue() {
    // 「屏幕要和配色有关联」：每套方案的屏幕底都从自己的卡片色 / 夜色底混主色派生，不能是一套共同的灰。
    for dark in [false, true] {
      let screens: BTreeSet<String> = SCHEMES
        .iter()
        .map(|s| get(&s.display_tokens(dark), "--display-bg").hex())
        .collect();
      assert!(
        screens.len() >= SCHEMES.len() - 2,
        "dark={dark}: {} 套方案只有 {} 种屏幕底，屏幕没跟配色走",
        SCHEMES.len(),
        screens.len()
      );
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
