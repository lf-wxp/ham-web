//! 复数类别（CLDR）：决定「`{} 天`」在英文下该是 `day` 还是 `days`。
//!
//! 界面文案里有大量「数量 + 单位」的短串（`common.days` = `{} 天`），英文 / 西班牙文
//! 要按数量换词形；一个 key 只存一条译文就会输出 `1 days` 这类语法错误。
//!
//! 这里只做**选类别**这一件事：给定语言与数量，返回 CLDR 的复数类别。词条变体的存储与
//! 查表在前端（`crates/app/src/i18n/`），规则与文案因此互不牵动。
//!
//! 类别集合取自 CLDR 的六类（`zero` / `one` / `two` / `few` / `many` / `other`）。
//! 当前接入的 zh / en / es 只用到 [`Category::One`] 与 [`Category::Other`]，其余类别在
//! 存储与查表通路里已预留 —— 将来加俄语 / 阿拉伯语只改 [`category`] 里的规则表，
//! 不必动数据结构。

/// CLDR 复数类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
  Zero,
  One,
  Two,
  Few,
  Many,
  /// 兜底类别：任何语言都必须提供，也是查表链的最后一级回退。
  Other,
}

impl Category {
  /// 词条变体表里用的名字（`data/i18n/{lang}/*.json` 里复数对象的 key）。
  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Zero => "zero",
      Self::One => "one",
      Self::Two => "two",
      Self::Few => "few",
      Self::Many => "many",
      Self::Other => "other",
    }
  }

  /// 全部合法变体名，供 `check-i18n` 校验词典用。
  pub const ALL: &'static [&'static str] = &["zero", "one", "two", "few", "many", "other"];

  /// 解析变体名；非法名字返回 `None`。
  #[must_use]
  pub fn from_variant(s: &str) -> Option<Self> {
    match s {
      "zero" => Some(Self::Zero),
      "one" => Some(Self::One),
      "two" => Some(Self::Two),
      "few" => Some(Self::Few),
      "many" => Some(Self::Many),
      "other" => Some(Self::Other),
      _ => None,
    }
  }
}

/// 参与复数判断的数量。
///
/// CLDR 的规则操作数是 `n`（值）、`i`（整数部分）与 `v`（小数位数）。当前三种语言只
/// 区分「有没有小数部分」（en / es 的 `one` 都是 `i = 1 and v = 0`），因此 `v` 存成
/// 布尔 —— 接入需要精确小数位数的语言（如 `ar`）时再改成 `u32`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Count {
  /// 整数部分（按绝对值取，因此 `-1` 与 `1` 同类）。
  i: u64,
  /// 是否没有小数部分（CLDR 的 `v == 0`）。
  integral: bool,
}

impl Count {
  /// 整数数量：界面里的计数（题数、天数、条数）基本都是整数。
  #[must_use]
  pub const fn int(i: u64) -> Self {
    Self { i, integral: true }
  }

  /// 带小数的数量：`1.5` 个百分点是「非整数」，与 `1` 不同类。
  ///
  /// 极大值（如 `1e30`）的 `i` 按饱和转换取，`integral` 仍按小数部分判定。
  #[must_use]
  pub fn float(n: f64) -> Self {
    let abs = n.abs();
    Self {
      i: if abs.is_finite() {
        abs.trunc() as u64
      } else {
        0
      },
      integral: abs.fract() == 0.0,
    }
  }
}

impl From<u64> for Count {
  fn from(v: u64) -> Self {
    Self::int(v)
  }
}

impl From<u32> for Count {
  fn from(v: u32) -> Self {
    Self::int(u64::from(v))
  }
}

impl From<usize> for Count {
  fn from(v: usize) -> Self {
    Self::int(v as u64)
  }
}

impl From<i64> for Count {
  fn from(v: i64) -> Self {
    Self::int(v.unsigned_abs())
  }
}

impl From<i32> for Count {
  fn from(v: i32) -> Self {
    Self::int(u64::from(v.unsigned_abs()))
  }
}

impl From<f64> for Count {
  fn from(v: f64) -> Self {
    Self::float(v)
  }
}

/// 按 CLDR 规则取 `lang` 下数量 `n` 的复数类别。
///
/// 未接入规则的语言（含中文）一律返回 [`Category::Other`] —— 中文没有复数变化，前端
/// 在中文下也不会走到这里。
#[must_use]
pub fn category(lang: &str, n: Count) -> Category {
  let one = n.i == 1 && n.integral;
  match lang {
    // en：`one` → i = 1 and v = 0；es：`one` → i = 1 and v = 0
    // （CLDR 42 起西班牙语不再有 `many`，大数形式并入 `other`）。
    "en" | "es" => {
      if one {
        Category::One
      } else {
        Category::Other
      }
    }
    _ => Category::Other,
  }
}

#[cfg(test)]
mod tests {
  use super::{Category, Count, category};

  fn cat(lang: &str, n: impl Into<Count>) -> Category {
    category(lang, n.into())
  }

  #[test]
  fn english_uses_one_only_for_exactly_one() {
    for n in [0u64, 2, 3, 11, 21, 101] {
      assert_eq!(cat("en", n), Category::Other, "n = {n}");
    }
    assert_eq!(cat("en", 1u64), Category::One);
    assert_eq!(cat("en", 0u64), Category::Other);
  }

  #[test]
  fn spanish_matches_english_for_the_cases_that_matter() {
    assert_eq!(cat("es", 1u64), Category::One);
    assert_eq!(cat("es", 0u64), Category::Other);
    assert_eq!(cat("es", 2u64), Category::Other);
    assert_eq!(cat("es", 1_000_000u64), Category::Other);
  }

  /// 中文没有复数变化；未接入规则的语言也不能崩，统一回落 `other`。
  #[test]
  fn unknown_and_chinese_languages_fall_back_to_other() {
    for lang in ["zh", "fr", "ar", ""] {
      assert_eq!(cat(lang, 1u64), Category::Other, "{lang}");
      assert_eq!(cat(lang, 5u64), Category::Other, "{lang}");
    }
  }

  /// 小数不是「一个」：`1.5 days` 而非 `1.5 day`。
  #[test]
  fn fractional_values_are_not_singular() {
    assert_eq!(cat("en", 1.0f64), Category::One);
    assert_eq!(cat("en", 1.5f64), Category::Other);
    assert_eq!(cat("en", 0.5f64), Category::Other);
    assert_eq!(cat("es", 1.0f64), Category::One);
    assert_eq!(cat("es", 1.5f64), Category::Other);
  }

  /// 负数按绝对值取：`-1 day` 与 `1 day` 同类（`-1` 不是「多个」）。
  #[test]
  fn negative_counts_use_absolute_value() {
    assert_eq!(cat("en", -1i64), Category::One);
    assert_eq!(cat("en", -2i64), Category::Other);
    assert_eq!(cat("en", -1i32), Category::One);
  }

  /// 界面里的计数多为 `usize`：转换不能溢出成 `1`。
  #[test]
  fn usize_and_extremes_do_not_overflow() {
    assert_eq!(cat("en", usize::MAX), Category::Other);
    assert_eq!(cat("en", u64::MAX), Category::Other);
    assert_eq!(cat("en", i64::MIN), Category::Other);
    assert_eq!(cat("en", 1usize), Category::One);
  }

  #[test]
  fn non_finite_floats_do_not_panic() {
    assert_eq!(cat("en", f64::NAN), Category::Other);
    assert_eq!(cat("en", f64::INFINITY), Category::Other);
  }

  #[test]
  fn variant_names_round_trip() {
    for c in [
      Category::Zero,
      Category::One,
      Category::Two,
      Category::Few,
      Category::Many,
      Category::Other,
    ] {
      assert_eq!(Category::from_variant(c.as_str()), Some(c));
      assert!(Category::ALL.contains(&c.as_str()));
    }
    assert_eq!(Category::from_variant("sometimes"), None);
  }
}
