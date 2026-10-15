//! 互调、谐波与镜频：给定若干发射频率，列出会落到哪里，并标出落入业余波段的，排查 RFI 时用。
//!
//! 产物的定义（`f1 < f2 < f3 …`，频率按升序编号，MHz）：
//!
//! ```text
//! 谐波        n·f                       n = 2 … 5
//! 2 阶互调    f_i + f_j,  f_j − f_i
//! 3 阶互调    2·f_i ± f_j,  2·f_j ± f_i              （两个信号）
//!             f_i + f_j − f_k,  f_i + f_j + f_k      （三个信号）
//! 镜频        f_rx ± 2·f_IF                          （高端 / 低端本振）
//! ```
//!
//! 边界：
//! - 只做**频率**的代数，不估算幅度：产物落在哪是确定的，强不强取决于非线性程度与功率，
//!   这里不下结论；
//! - 5 次以上的谐波、4 阶及以上的互调不列（实际中越高阶越弱，列出来只会淹没真正要看的）；
//! - 「落入业余波段」按全站唯一的 [`AMATEUR_BAND_EDGES`]（中国 ITU 三区口径，半开区间）判断，
//!   该表只到 70 cm，**1.2 GHz 及以上的业余波段不会被标出**；
//! - 非正的产物（如 `2·f1 − f2` 在 `f2 > 2·f1` 时）丢弃。

use crate::bands::AMATEUR_BAND_EDGES;

/// 一次最多接受的频率个数：三信号组合随个数的立方增长，再多表格也没法读。
pub const MAX_FREQS: usize = 8;
/// 列出的最高谐波次数。
pub const MAX_HARMONIC: u32 = 5;

/// 产物的种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
  Harmonic,
  Im2,
  Im3,
}

/// 一个产物。
#[derive(Debug, Clone, PartialEq)]
pub struct Product {
  pub kind: Kind,
  /// 阶数：谐波为次数 n，互调为 2 或 3。
  pub order: u32,
  pub freq_mhz: f64,
  /// 数学表达式，如 `2·f1 − f2`；`f1 …` 指升序编号后的输入频率（见 [`Analysis::inputs`]）。
  pub expr: String,
  /// 落入的业余波段（如 `20m`），不在任何已知波段内为 `None`。
  pub band: Option<&'static str>,
}

/// 输入不合法的原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreqError {
  /// 一个频率都没有。
  Empty,
  /// 超过 [`MAX_FREQS`]（去重之后）。
  TooMany,
  /// 第 `n`（1 起算）个值不是正的有限数。
  Invalid(usize),
}

/// 分析结果。
#[derive(Debug, Clone, PartialEq)]
pub struct Analysis {
  /// 升序、去重后的输入频率（MHz）；产物表达式里的 `f1 …` 按它编号。
  pub inputs: Vec<f64>,
  /// 按频率升序（同频按阶数）排列的全部产物。
  pub products: Vec<Product>,
}

/// 该频率（MHz）落入的业余波段；半开区间 `[下, 上)`。
#[must_use]
pub fn amateur_band(freq_mhz: f64) -> Option<&'static str> {
  AMATEUR_BAND_EDGES
    .iter()
    .find(|&&(_, lo, hi)| (lo..hi).contains(&freq_mhz))
    .map(|&(name, _, _)| name)
}

/// 把文本解析成频率列表（MHz）：逗号、分号、空白或换行分隔，空行与 `#` 开头的行忽略。
///
/// 错误里的序号是第几个**值**（不是第几行）。
pub fn parse_freqs(text: &str) -> Result<Vec<f64>, FreqError> {
  let mut out = Vec::new();
  for line in text.lines() {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
      continue;
    }
    for field in line
      .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
      .filter(|f| !f.is_empty())
    {
      let v = field
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v > 0.0)
        .ok_or(FreqError::Invalid(out.len() + 1))?;
      out.push(v);
    }
  }
  Ok(out)
}

fn product(kind: Kind, order: u32, freq: f64, expr: String) -> Option<Product> {
  (freq.is_finite() && freq > 0.0).then(|| Product {
    kind,
    order,
    freq_mhz: freq,
    expr,
    band: amateur_band(freq),
  })
}

/// 列出所有谐波与互调产物。输入需为正的有限数；重复的频率只算一个。
pub fn analyze(freqs: &[f64]) -> Result<Analysis, FreqError> {
  if freqs.is_empty() {
    return Err(FreqError::Empty);
  }
  if let Some(i) = freqs.iter().position(|f| !(f.is_finite() && *f > 0.0)) {
    return Err(FreqError::Invalid(i + 1));
  }
  let mut f = freqs.to_vec();
  f.sort_by(f64::total_cmp);
  f.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
  if f.len() > MAX_FREQS {
    return Err(FreqError::TooMany);
  }

  let n = f.len();
  let mut all: Vec<Product> = Vec::new();
  let mut push = |p: Option<Product>| all.extend(p);

  for (i, &a) in f.iter().enumerate() {
    for h in 2..=MAX_HARMONIC {
      push(product(
        Kind::Harmonic,
        h,
        f64::from(h) * a,
        format!("{h}·f{}", i + 1),
      ));
    }
  }
  for i in 0..n {
    for j in (i + 1)..n {
      let (a, b) = (f[i], f[j]);
      let (ni, nj) = (i + 1, j + 1);
      push(product(Kind::Im2, 2, a + b, format!("f{ni} + f{nj}")));
      push(product(Kind::Im2, 2, b - a, format!("f{nj} − f{ni}")));
      push(product(
        Kind::Im3,
        3,
        2.0 * a - b,
        format!("2·f{ni} − f{nj}"),
      ));
      push(product(
        Kind::Im3,
        3,
        2.0 * b - a,
        format!("2·f{nj} − f{ni}"),
      ));
      push(product(
        Kind::Im3,
        3,
        2.0 * a + b,
        format!("2·f{ni} + f{nj}"),
      ));
      push(product(
        Kind::Im3,
        3,
        2.0 * b + a,
        format!("2·f{nj} + f{ni}"),
      ));
    }
  }
  for i in 0..n {
    for j in (i + 1)..n {
      for k in (j + 1)..n {
        let (a, b, c) = (f[i], f[j], f[k]);
        let (ni, nj, nk) = (i + 1, j + 1, k + 1);
        push(product(
          Kind::Im3,
          3,
          a + b - c,
          format!("f{ni} + f{nj} − f{nk}"),
        ));
        push(product(
          Kind::Im3,
          3,
          a + c - b,
          format!("f{ni} + f{nk} − f{nj}"),
        ));
        push(product(
          Kind::Im3,
          3,
          b + c - a,
          format!("f{nj} + f{nk} − f{ni}"),
        ));
        push(product(
          Kind::Im3,
          3,
          a + b + c,
          format!("f{ni} + f{nj} + f{nk}"),
        ));
      }
    }
  }

  all.sort_by(|x, y| {
    x.freq_mhz
      .total_cmp(&y.freq_mhz)
      .then(x.order.cmp(&y.order))
      .then_with(|| x.expr.cmp(&y.expr))
  });
  Ok(Analysis {
    inputs: f,
    products: all,
  })
}

/// 镜频：接收频率两侧各一个，取决于本振在信号的哪一侧。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Image {
  /// 高端本振（`LO = f_rx + f_IF`）时的镜频：`f_rx + 2·f_IF`。
  pub high_side_mhz: f64,
  /// 低端本振（`LO = f_rx − f_IF`）时的镜频：`f_rx − 2·f_IF`；不是正数时为 `None`。
  pub low_side_mhz: Option<f64>,
  pub high_side_band: Option<&'static str>,
  pub low_side_band: Option<&'static str>,
}

/// 由接收频率与中频（均为 MHz，正的有限数）算镜频；输入不合法返回 `None`。
#[must_use]
pub fn image_frequencies(rx_mhz: f64, if_mhz: f64) -> Option<Image> {
  let ok = |v: f64| v.is_finite() && v > 0.0;
  if !(ok(rx_mhz) && ok(if_mhz)) {
    return None;
  }
  let high = rx_mhz + 2.0 * if_mhz;
  let low = rx_mhz - 2.0 * if_mhz;
  let low = (low > 0.0).then_some(low);
  Some(Image {
    high_side_mhz: high,
    low_side_mhz: low,
    high_side_band: amateur_band(high),
    low_side_band: low.and_then(amateur_band),
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
  }

  fn find<'a>(a: &'a Analysis, expr: &str) -> &'a Product {
    a.products
      .iter()
      .find(|p| p.expr == expr)
      .unwrap_or_else(|| panic!("没有产物 {expr}"))
  }

  #[test]
  fn band_lookup_is_half_open_and_uses_the_shared_table() {
    assert_eq!(amateur_band(14.0), Some("20m"));
    assert_eq!(amateur_band(14.35), None); // 上沿不含
    assert_eq!(amateur_band(7.1), Some("40m"));
    assert_eq!(amateur_band(7.3), None); // 三区 40 m 到 7.2
    assert_eq!(amateur_band(144.0), Some("2m"));
    assert_eq!(amateur_band(148.0), None);
    assert_eq!(amateur_band(18.25), None);
    assert_eq!(amateur_band(1296.0), None); // 表只到 70 cm
    // 与全站单一事实来源一致：表里每个波段的下沿都应命中自己。
    for &(name, lo, hi) in AMATEUR_BAND_EDGES {
      assert_eq!(amateur_band(lo), Some(name), "{name}");
      assert_ne!(amateur_band(hi), Some(name), "{name} 上沿");
    }
  }

  #[test]
  fn harmonics_of_a_40m_signal() {
    let a = analyze(&[7.1]).unwrap();
    let h: Vec<_> = a
      .products
      .iter()
      .map(|p| (p.expr.as_str(), p.freq_mhz, p.band))
      .collect();
    assert_eq!(a.inputs, vec![7.1]);
    assert_eq!(h.len(), 4); // 2 … 5 次
    assert!(near(h[0].1, 14.2) && h[0].0 == "2·f1" && h[0].2 == Some("20m"));
    assert!(near(h[1].1, 21.3) && h[1].2 == Some("15m"));
    assert!(near(h[2].1, 28.4) && h[2].2 == Some("10m"));
    assert!(near(h[3].1, 35.5) && h[3].2.is_none());
    assert!(a.products.iter().all(|p| p.kind == Kind::Harmonic));
  }

  #[test]
  fn harmonics_that_miss_every_band_are_not_flagged() {
    // 3.65 MHz 的谐波：7.3（超出三区 40 m 的 7.2）、10.95、14.6、18.25 —— 全部落空。
    let a = analyze(&[3.65]).unwrap();
    assert!(
      a.products.iter().all(|p| p.band.is_none()),
      "{:?}",
      a.products
    );
  }

  #[test]
  fn two_close_signals_make_the_classic_third_order_pair() {
    // 14.200 与 14.205：2f1−f2 = 14.195，2f2−f1 = 14.210，二者都在 20 m 内，
    // 差频 0.005（不在任何业余段），和频 28.405 在 10 m。
    let a = analyze(&[14.205, 14.200]).unwrap();
    assert_eq!(a.inputs, vec![14.2, 14.205]);
    let low = find(&a, "2·f1 − f2");
    let high = find(&a, "2·f2 − f1");
    assert!(near(low.freq_mhz, 14.195) && low.band == Some("20m") && low.order == 3);
    assert!(near(high.freq_mhz, 14.21) && high.band == Some("20m"));
    let diff = find(&a, "f2 − f1");
    assert!(near(diff.freq_mhz, 0.005) && diff.band.is_none() && diff.kind == Kind::Im2);
    let sum = find(&a, "f1 + f2");
    assert!(near(sum.freq_mhz, 28.405) && sum.band == Some("10m"));
    assert!(near(find(&a, "2·f1 + f2").freq_mhz, 42.605));
    assert!(near(find(&a, "2·f2 + f1").freq_mhz, 42.61));
  }

  #[test]
  fn three_signal_third_order_products() {
    let a = analyze(&[14.0, 14.1, 14.3]).unwrap();
    assert!(near(find(&a, "f1 + f2 − f3").freq_mhz, 13.8));
    assert!(near(find(&a, "f1 + f3 − f2").freq_mhz, 14.2));
    assert!(near(find(&a, "f2 + f3 − f1").freq_mhz, 14.4));
    assert!(near(find(&a, "f1 + f2 + f3").freq_mhz, 42.4));
    assert_eq!(find(&a, "f1 + f3 − f2").band, Some("20m"));
    assert_eq!(find(&a, "f2 + f3 − f1").band, None); // 14.4 > 14.35
  }

  #[test]
  fn product_counts_follow_the_combinatorics() {
    // n 个频率、全部产物为正：谐波 4n + 2 阶 2·C(n,2) + 3 阶两信号 4·C(n,2) + 三信号 4·C(n,3)。
    // 取 7.0、7.1、7.2：f2 − f1 等都为正，2f1 − f2 = 6.9 > 0，所有产物都保留。
    let a = analyze(&[7.0, 7.1, 7.2]).unwrap();
    assert_eq!(a.products.len(), 4 * 3 + 2 * 3 + 4 * 3 + 4);
    let kinds = |k| a.products.iter().filter(|p| p.kind == k).count();
    assert_eq!(
      (kinds(Kind::Harmonic), kinds(Kind::Im2), kinds(Kind::Im3)),
      (12, 6, 16)
    );
  }

  #[test]
  fn non_positive_products_are_dropped() {
    // f2 = 3·f1：2·f1 − f2 = −f1 < 0，丢弃；f2 − f1、2·f2 − f1 等仍然保留。
    let a = analyze(&[2.0, 6.0]).unwrap();
    assert!(a.products.iter().all(|p| p.freq_mhz > 0.0));
    assert!(a.products.iter().all(|p| p.expr != "2·f1 − f2"));
    assert!(a.products.iter().any(|p| p.expr == "2·f2 − f1"));
  }

  #[test]
  fn products_are_sorted_by_frequency_then_order() {
    let a = analyze(&[7.0, 7.1, 14.2]).unwrap();
    for w in a.products.windows(2) {
      assert!(
        w[0].freq_mhz < w[1].freq_mhz
          || (w[0].freq_mhz == w[1].freq_mhz && w[0].order <= w[1].order),
        "{:?} 排在 {:?} 前面",
        w[0],
        w[1]
      );
    }
  }

  #[test]
  fn duplicates_collapse_and_inputs_are_validated() {
    let a = analyze(&[7.1, 7.1, 7.1]).unwrap();
    assert_eq!(a.inputs, vec![7.1]);
    assert_eq!(analyze(&[]), Err(FreqError::Empty));
    assert_eq!(analyze(&[7.1, 0.0]), Err(FreqError::Invalid(2)));
    assert_eq!(analyze(&[-1.0]), Err(FreqError::Invalid(1)));
    assert_eq!(analyze(&[7.1, f64::NAN]), Err(FreqError::Invalid(2)));
    assert_eq!(analyze(&[f64::INFINITY]), Err(FreqError::Invalid(1)));
    let nine: Vec<f64> = (1..=9).map(|i| f64::from(i) * 3.3).collect();
    assert_eq!(analyze(&nine), Err(FreqError::TooMany));
    // 恰好 MAX_FREQS 个可以。
    assert!(analyze(&nine[..MAX_FREQS]).is_ok());
    // 重复的不占名额：9 个值里有重复，去重后 8 个。
    let mut dup = nine[..MAX_FREQS].to_vec();
    dup.push(nine[0]);
    assert!(analyze(&dup).is_ok());
  }

  #[test]
  fn parse_accepts_mixed_separators_and_comments() {
    assert_eq!(
      parse_freqs("7.1, 14.2;21.3  28.4\n# 备注\n\n 50.1 ").unwrap(),
      vec![7.1, 14.2, 21.3, 28.4, 50.1]
    );
    assert_eq!(parse_freqs("").unwrap(), Vec::<f64>::new());
    assert_eq!(parse_freqs("7.1, abc"), Err(FreqError::Invalid(2)));
    assert_eq!(parse_freqs("7.1\n0"), Err(FreqError::Invalid(2)));
    assert_eq!(parse_freqs("7.1\n-3"), Err(FreqError::Invalid(2)));
    assert_eq!(parse_freqs("inf"), Err(FreqError::Invalid(1)));
  }

  #[test]
  fn image_frequencies_both_sides() {
    // 14.1 MHz、IF 3.5 MHz：高端本振的镜频 21.1（15 m），低端本振的镜频 7.1（40 m）。
    let im = image_frequencies(14.1, 3.5).unwrap();
    assert!(near(im.high_side_mhz, 21.1) && im.high_side_band == Some("15m"));
    assert!(near(im.low_side_mhz.unwrap(), 7.1) && im.low_side_band == Some("40m"));

    // 145 MHz、10.7 MHz 中频：123.6 与 166.4 都不在业余段。
    let im = image_frequencies(145.0, 10.7).unwrap();
    assert!(near(im.high_side_mhz, 166.4) && near(im.low_side_mhz.unwrap(), 123.6));
    assert!(im.high_side_band.is_none() && im.low_side_band.is_none());

    // 中频太高：低端镜频为负，没有这一侧。
    let im = image_frequencies(7.1, 4.0).unwrap();
    assert!(near(im.high_side_mhz, 15.1));
    assert_eq!(im.low_side_mhz, None);
    assert_eq!(im.low_side_band, None);
    // 恰好为 0 也不算。
    assert_eq!(image_frequencies(8.0, 4.0).unwrap().low_side_mhz, None);
  }

  #[test]
  fn image_rejects_nonsense() {
    for (rx, i) in [
      (0.0, 10.7),
      (14.0, 0.0),
      (-1.0, 3.5),
      (14.0, -3.5),
      (f64::NAN, 3.5),
      (14.0, f64::INFINITY),
    ] {
      assert_eq!(image_frequencies(rx, i), None, "{rx} {i}");
    }
  }
}
