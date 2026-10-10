//! 色彩数学：十六进制解析、线性混合、WCAG 相对亮度与对比度。
//!
//! 放在 core 而不是 tools 里，是因为两边都要用同一套算法：
//! - 构建工具（`pixel-palette` / `pixel-schemes`）用它审计与校正调色板；
//! - 配色方案的派生规则（[`crate::color_scheme`]）靠它保证「文字色在每个表面上都够对比度」。
//!
//! 对比度公式必须与浏览器端的 axe 一致，所以这里照 WCAG 2.x 原文实现，不做任何「近似」。

/// sRGB 颜色（8 位通道）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
  pub const WHITE: Self = Self(255, 255, 255);
  pub const BLACK: Self = Self(0, 0, 0);

  /// 由 `0xRRGGBB` 构造（`const`：方案表里的种子色直接写十六进制字面量）。
  #[must_use]
  pub const fn from_u32(v: u32) -> Self {
    // 高位被 `as u8` 静默吃掉，颜色只是「看起来不太对」，比 panic 难查得多。
    debug_assert!(v <= 0x00ff_ffff, "from_u32 只接受 0xRRGGBB");
    Self((v >> 16) as u8, (v >> 8) as u8, v as u8)
  }

  /// 解析 `#rrggbb`（`#` 可省略，大小写均可）。不接受 3 位简写：调色板里没有，写错了应当报错。
  #[must_use]
  pub fn parse(hex: &str) -> Option<Self> {
    let h = hex.strip_prefix('#').unwrap_or(hex);
    if h.len() != 6 || !h.is_ascii() {
      return None;
    }
    let ch = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
    Some(Self(ch(0)?, ch(2)?, ch(4)?))
  }

  /// 小写 `#rrggbb`。
  #[must_use]
  pub fn hex(self) -> String {
    format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
  }

  /// WCAG 相对亮度（0 = 黑，1 = 白）。
  #[must_use]
  pub fn luminance(self) -> f64 {
    fn lin(v: u8) -> f64 {
      let v = f64::from(v) / 255.0;
      if v <= 0.03928 {
        v / 12.92
      } else {
        ((v + 0.055) / 1.055).powf(2.4)
      }
    }
    0.2126 * lin(self.0) + 0.7152 * lin(self.1) + 0.0722 * lin(self.2)
  }

  /// 与 `other` 的 WCAG 对比度（1.0 ..= 21.0），与前后景顺序无关。
  #[must_use]
  pub fn contrast(self, other: Self) -> f64 {
    let (a, b) = (self.luminance(), other.luminance());
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
  }

  /// 向 `other` 混合 `t`（0 = 自己，1 = `other`），逐通道四舍五入。
  ///
  /// 取整用「银行家舍入」（遇 .5 取偶）：与 Python 的 `round()` 一致，当年调色板就是这么生成的，
  /// 换成「远离零」会在恰好 .5 的通道上差 1，色值就对不上存量的 `palette.css` 了。
  #[must_use]
  pub fn mix(self, other: Self, t: f64) -> Self {
    let ch = |a: u8, b: u8| {
      let v = f64::from(a) * (1.0 - t) + f64::from(b) * t;
      // clamp 之后必在 0..=255，转 u8 不会截断。
      v.round_ties_even().clamp(0.0, 255.0) as u8
    };
    Self(
      ch(self.0, other.0),
      ch(self.1, other.1),
      ch(self.2, other.2),
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_and_prints_lowercase_hex() {
    assert_eq!(Rgb::parse("#FFCD75"), Some(Rgb(255, 205, 117)));
    assert_eq!(Rgb::parse("ffcd75"), Some(Rgb(255, 205, 117)));
    assert_eq!(Rgb(26, 28, 44).hex(), "#1a1c2c");
  }

  #[test]
  fn from_u32_matches_the_hex_form() {
    assert_eq!(Rgb::from_u32(0xFFCD75), Rgb::parse("#ffcd75").unwrap());
    assert_eq!(Rgb::from_u32(0), Rgb::BLACK);
  }

  #[test]
  fn rejects_malformed_hex() {
    for bad in ["", "#fff", "#12345", "#1234567", "#gggggg", "#12345é"] {
      assert_eq!(Rgb::parse(bad), None, "{bad:?}");
    }
  }

  #[test]
  fn black_on_white_is_the_maximum_contrast() {
    assert!((Rgb::BLACK.contrast(Rgb::WHITE) - 21.0).abs() < 1e-9);
    assert!((Rgb::WHITE.contrast(Rgb::BLACK) - 21.0).abs() < 1e-9);
    assert!((Rgb::WHITE.contrast(Rgb::WHITE) - 1.0).abs() < 1e-9);
  }

  #[test]
  fn contrast_matches_known_pairs() {
    // 站内已用 axe 实测过的两组：深蓝灰字落在金色底、墨色字落在纸色底。
    let ink = Rgb::parse("#1a1c2c").unwrap();
    let paper = Rgb::parse("#fff8e1").unwrap();
    assert!(ink.contrast(paper) > 15.0);
    let muted = Rgb::parse("#42566c").unwrap();
    let gold = Rgb::parse("#ffcd75").unwrap();
    assert!((5.0..5.3).contains(&muted.contrast(gold)));
  }

  #[test]
  fn mix_endpoints_are_exact() {
    let (a, b) = (Rgb(10, 20, 30), Rgb(200, 100, 0));
    assert_eq!(a.mix(b, 0.0), a);
    assert_eq!(a.mix(b, 1.0), b);
  }

  #[test]
  fn mix_rounds_halves_to_even() {
    // 0.5 → 0（偶），1.5 → 2（偶）；「远离零」会得到 1 与 2 / 1 与 2 的不同结果。
    assert_eq!(Rgb(1, 0, 0).mix(Rgb(0, 0, 0), 0.5), Rgb(0, 0, 0));
    assert_eq!(Rgb(3, 0, 0).mix(Rgb(0, 0, 0), 0.5), Rgb(2, 0, 0));
    assert_eq!(
      Rgb(0, 0, 0).mix(Rgb(255, 255, 255), 0.5),
      Rgb(128, 128, 128)
    );
  }
}
