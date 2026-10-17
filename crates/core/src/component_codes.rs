//! 元件丝印代码 → 值：SMD 电阻与瓷片电容。
//!
//! 面向「手里拿个贴片 / 瓷片，上面印着代码，它到底多大」的识别场景。代码规则都是
//! 公开标准，不依赖外部数据：
//!
//! - **SMD 电阻**：三位（`472` = 47×10² = 4.7 kΩ）、四位（`1001` = 1 kΩ）、
//!   EIA-96（两位代码 + 乘数字母，`68X` = 49.9 Ω）、`R` 小数点（`4R7` = 4.7 Ω）、
//!   `0` / `000` = 0 Ω 跳线。
//! - **瓷片电容**：两位（pF）或三位（`104` = 10×10⁴ pF = 100 nF），末尾字母是容差。
//!
//! # 一个刻意的取舍：`两位数字 + R`
//!
//! EIA-96 的乘数表里 `R` 也是 ×0.01，与 `R` 当小数点的老写法（`47R` = 47 Ω）撞车：
//! 同一个 `47R` 按「小数点」是 47 Ω、按 EIA-96 是 3.01 Ω。这里**一律按小数点**解释 ——
//! 贴片电阻上两位数末尾的 `R` 绝大多数就是这个意思，而 EIA-96 的 ×0.01 实际多用 `Y` 表示
//!（本模块也支持 `Y`），代价是 `68R` 会被读成 68 Ω 而不是 4.99 Ω。`Y` / `Z` 不受影响。

/// SMD 电阻代码的种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmdCode {
  /// 三位代码（`472`）。
  ThreeDigit,
  /// 四位代码（`1001`）。
  FourDigit,
  /// EIA-96（`68X`）。
  Eia96,
  /// `R` 小数点（`4R7`）。
  DecimalR,
  /// 0 Ω 跳线（`0` / `00` / `000`）。
  Jumper,
}

/// 一个 SMD 电阻代码的解码结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmdResistor {
  /// 阻值（Ω）。
  pub ohms: f64,
  /// 代码种类。
  pub code: SmdCode,
}

/// EIA-96 两位代码 → 标称值（Ω，×100 的 E96 数列，`01` → 100 … `96` → 976）。
const EIA96_VALUES: [u32; 96] = [
  100, 102, 105, 107, 110, 113, 115, 118, 121, 124, 127, 130, 133, 137, 140, 143, 147, 150, 154,
  158, 162, 165, 169, 174, 178, 182, 187, 191, 196, 200, 205, 210, 215, 221, 226, 232, 237, 243,
  249, 255, 261, 267, 274, 280, 287, 294, 301, 309, 316, 324, 332, 340, 348, 357, 365, 374, 383,
  392, 402, 412, 422, 432, 442, 453, 464, 475, 487, 499, 511, 523, 536, 549, 562, 576, 590, 604,
  619, 634, 649, 665, 681, 698, 715, 732, 750, 768, 787, 806, 825, 845, 866, 887, 909, 931, 953,
  976,
];

/// EIA-96 乘数字母 → 乘数。
///
/// `R` 按 ×0.01 是标准表里的写法，但它在 [`smd_resistor`] 里已经被「小数点」分支先吃掉
///（见模块文档的取舍），所以这里不收 `R`。
fn eia96_multiplier(letter: &str) -> Option<f64> {
  match letter {
    "Z" => Some(0.001),
    "Y" => Some(0.01),
    "X" | "S" => Some(0.1),
    "A" => Some(1.0),
    "B" | "H" => Some(10.0),
    "C" => Some(100.0),
    "D" => Some(1000.0),
    "E" => Some(10_000.0),
    "F" => Some(100_000.0),
    _ => None,
  }
}

/// 解码 SMD 电阻代码；无法识别返回 `None`。
#[must_use]
pub fn smd_resistor(code: &str) -> Option<SmdResistor> {
  let c = code.trim().to_ascii_uppercase();
  if c.is_empty() {
    return None;
  }
  // 跳线。
  if c == "0" || c == "00" || c == "000" || c == "0000" {
    return Some(SmdResistor {
      ohms: 0.0,
      code: SmdCode::Jumper,
    });
  }
  // R 小数点：`4R7` = 4.7、`R47` = 0.47、`47R` = 47。
  if c.contains('R') {
    let (a, b) = c.split_once('R')?;
    // 两侧只能是数字（空的一侧表示 0），且不能两边都空：单个 `R` 不是任何零件的代码，
    // 按旧的兜底会算成 0 Ω —— 那会让人把不认识的一行当成跳线电阻。
    if a.is_empty() && b.is_empty() {
      return None;
    }
    if !a.bytes().all(|x| x.is_ascii_digit()) || !b.bytes().all(|x| x.is_ascii_digit()) {
      return None;
    }
    let left = if a.is_empty() { "0" } else { a };
    let right = if b.is_empty() { "0" } else { b };
    let ohms = format!("{left}.{right}").parse::<f64>().ok()?;
    return Some(SmdResistor {
      ohms,
      code: SmdCode::DecimalR,
    });
  }
  // EIA-96：两位数字 + 一个字母。
  if c.len() == 3 && c.as_bytes()[2].is_ascii_alphabetic() {
    let idx = c[..2].parse::<usize>().ok()?;
    let mult = eia96_multiplier(&c[2..])?;
    // 代码从 `01` 起，`00` 不是合法下标 —— 用 `checked_sub` 表达这一点，别靠越界兜。
    let base = *EIA96_VALUES.get(idx.checked_sub(1)?)? as f64;
    return Some(SmdResistor {
      ohms: base * mult,
      code: SmdCode::Eia96,
    });
  }
  // 三位 / 四位数字：前 n−1 位是有效数字，末位是乘数。
  if (c.len() == 3 || c.len() == 4) && c.bytes().all(|b| b.is_ascii_digit()) {
    let digits = c[..c.len() - 1].parse::<f64>().ok()?;
    let exp = c[c.len() - 1..].parse::<f64>().ok()?;
    return Some(SmdResistor {
      ohms: digits * 10f64.powf(exp),
      code: if c.len() == 3 {
        SmdCode::ThreeDigit
      } else {
        SmdCode::FourDigit
      },
    });
  }
  None
}

/// 一个瓷片电容代码的解码结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CeramicCapacitor {
  /// 容值（F）。
  pub farads: f64,
  /// 容差（如 `"±10%"`；无字母时为 `None`）。
  pub tolerance: Option<&'static str>,
}

/// 容差字母 → 容差文本（符号 + 数字，无需翻译）。
///
/// `B / C / D` 是**绝对容差**（pF），只有 `F / G / J / K / M` 才是百分比：这两组在 EIA 表里
/// 量纲不同，把 `B` 写成 `±0.1%` 会让「±0.1 pF 的小容值电容」被读成百分比公差，直接误导选型。
fn cap_tolerance(letter: &str) -> Option<&'static str> {
  match letter {
    "B" => Some("±0.1 pF"),
    "C" => Some("±0.25 pF"),
    "D" => Some("±0.5 pF"),
    "F" => Some("±1%"),
    "G" => Some("±2%"),
    "J" => Some("±5%"),
    "K" => Some("±10%"),
    "M" => Some("±20%"),
    "Z" => Some("+80/−20%"),
    _ => None,
  }
}

/// 解码瓷片电容代码（两位 pF / 三位 pF，末尾字母是容差）；无法识别返回 `None`。
#[must_use]
pub fn capacitor_code(code: &str) -> Option<CeramicCapacitor> {
  let c = code.trim().to_ascii_uppercase();
  if c.is_empty() {
    return None;
  }
  let (num, tolerance) = match c.as_bytes().last() {
    // 末位是字母就必须是已知容差：认不出来的字母不能当成「没有容差」糊过去，
    // 否则 `104Q` 会显示成一个看着正常的 100 nF。
    Some(&b) if b.is_ascii_alphabetic() => {
      (&c[..c.len() - 1], Some(cap_tolerance(&c[c.len() - 1..])?))
    }
    _ => (c.as_str(), None),
  };
  let pf = if num.len() == 2 && num.bytes().all(|b| b.is_ascii_digit()) {
    num.parse::<f64>().ok()?
  } else if num.len() == 3 && num.bytes().all(|b| b.is_ascii_digit()) {
    let digits = num[..2].parse::<f64>().ok()?;
    let exp = num[2..].parse::<f64>().ok()?;
    digits * 10f64.powf(exp)
  } else {
    return None;
  };
  Some(CeramicCapacitor {
    farads: pf * 1e-12,
    tolerance,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn smd_three_and_four_digit_codes() {
    assert_eq!(smd_resistor("472").unwrap().ohms, 4700.0);
    assert_eq!(smd_resistor("103").unwrap().ohms, 10_000.0);
    assert_eq!(smd_resistor("1001").unwrap().ohms, 1000.0);
    assert_eq!(smd_resistor("1000").unwrap().ohms, 100.0);
    assert_eq!(smd_resistor("472").unwrap().code, SmdCode::ThreeDigit);
    assert_eq!(smd_resistor("1001").unwrap().code, SmdCode::FourDigit);
  }

  #[test]
  fn smd_eia96_codes() {
    // 68 = 499，X = ×0.1 → 49.9 Ω；01 = 100，A = ×1 → 100 Ω；96 = 976，C = ×100 → 97.6 kΩ。
    assert!((smd_resistor("68X").unwrap().ohms - 49.9).abs() < 1e-9);
    assert_eq!(smd_resistor("01A").unwrap().ohms, 100.0);
    assert_eq!(smd_resistor("96C").unwrap().ohms, 97_600.0);
    assert_eq!(smd_resistor("68X").unwrap().code, SmdCode::Eia96);
    // 小数乘数：Y = ×0.01、Z = ×0.001（标准表里的另外两档）。
    assert_eq!(smd_resistor("01Y").unwrap().ohms, 1.0);
    assert!((smd_resistor("01Z").unwrap().ohms - 0.1).abs() < 1e-9);
    assert_eq!(smd_resistor("01Y").unwrap().code, SmdCode::Eia96);
    // 非法下标 / 非法乘数字母。
    assert_eq!(smd_resistor("00X"), None);
    assert_eq!(smd_resistor("97X"), None);
    assert_eq!(smd_resistor("68Q"), None);
  }

  #[test]
  fn smd_r_as_multiplier_is_read_as_decimal_point() {
    // 模块文档写明的取舍：`两位数字 + R` 按小数点解释，因此 `68R` = 68 Ω（而非 EIA-96 的 4.99 Ω）。
    let r = smd_resistor("68R").unwrap();
    assert_eq!(r.ohms, 68.0);
    assert_eq!(r.code, SmdCode::DecimalR);
  }

  #[test]
  fn smd_r_decimal_and_jumper() {
    assert_eq!(smd_resistor("4R7").unwrap().ohms, 4.7);
    assert_eq!(smd_resistor("R47").unwrap().ohms, 0.47);
    assert_eq!(smd_resistor("47R").unwrap().ohms, 47.0);
    assert_eq!(smd_resistor("0").unwrap().ohms, 0.0);
    assert_eq!(smd_resistor("000").unwrap().code, SmdCode::Jumper);
  }

  #[test]
  fn smd_rejects_garbage() {
    assert_eq!(smd_resistor(""), None);
    assert_eq!(smd_resistor("abc"), None);
    assert_eq!(smd_resistor("47"), None); // 只有两位且无 R：不是合法代码
    assert_eq!(smd_resistor("1A2"), None); // 字母不在末尾
    // 单个 `R` 不是代码（旧逻辑会兜成 0 Ω）；R 两侧也不能有非数字。
    assert_eq!(smd_resistor("R"), None);
    assert_eq!(smd_resistor("1R2R"), None);
    assert_eq!(smd_resistor("R-1"), None);
  }

  #[test]
  fn smd_eia96_table_is_fully_reachable() {
    // `01`…`96` 整表走一遍（乘数固定 A = ×1）：表里任一格错位或取不到都会被这里抓住。
    for (i, expected) in EIA96_VALUES.iter().enumerate() {
      let code = format!("{:02}A", i + 1);
      let got = smd_resistor(&code).unwrap_or_else(|| panic!("{code} 应可解码"));
      assert_eq!(got.ohms, f64::from(*expected), "{code}");
      assert_eq!(got.code, SmdCode::Eia96);
    }
    // 下标 0 与 97 都不在表内。
    assert_eq!(smd_resistor("00A"), None);
    assert_eq!(smd_resistor("97A"), None);
  }

  #[test]
  fn capacitor_three_digit_pf_code() {
    assert_eq!(capacitor_code("104").unwrap().farads, 100e-9);
    assert_eq!(capacitor_code("103").unwrap().farads, 10e-9);
    assert_eq!(capacitor_code("104K").unwrap().farads, 100e-9);
    assert_eq!(capacitor_code("104K").unwrap().tolerance, Some("±10%"));
    assert_eq!(capacitor_code("104").unwrap().tolerance, None);
  }

  #[test]
  fn capacitor_two_digit_and_tolerance_letters() {
    assert_eq!(capacitor_code("47").unwrap().farads, 47e-12);
    assert_eq!(capacitor_code("47J").unwrap().tolerance, Some("±5%"));
    assert_eq!(capacitor_code("104Z").unwrap().tolerance, Some("+80/−20%"));
    // 未知容差字母不能当成「没有容差」：整条代码按无法识别处理。
    assert_eq!(capacitor_code("104Q"), None);
  }

  #[test]
  fn capacitor_tolerance_letters_keep_their_units() {
    // B / C / D 是绝对容差（pF），F / G / J / K / M 才是百分比 —— 两组量纲不同，
    // 逐字母钉住，免得再被写成同一种单位。
    for (letter, expected) in [
      ("B", "±0.1 pF"),
      ("C", "±0.25 pF"),
      ("D", "±0.5 pF"),
      ("F", "±1%"),
      ("G", "±2%"),
      ("J", "±5%"),
      ("K", "±10%"),
      ("M", "±20%"),
      ("Z", "+80/−20%"),
    ] {
      let code = format!("47{letter}");
      assert_eq!(
        capacitor_code(&code).unwrap().tolerance,
        Some(expected),
        "{code}"
      );
    }
  }

  #[test]
  fn capacitor_rejects_garbage() {
    assert_eq!(capacitor_code(""), None);
    assert_eq!(capacitor_code("1"), None); // 单位数字不是合法代码
    assert_eq!(capacitor_code("1045"), None); // 四位无字母：不是瓷片代码
    assert_eq!(capacitor_code("abc"), None);
  }
}
