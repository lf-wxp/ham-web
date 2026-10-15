//! 全站共用的数值判定小工具。

/// 正的有限数：`v.is_finite() && v > 0.0`。
///
/// 各计算模块共用的输入口径：NaN、无穷、0 与负数一律不是合法输入。
pub(crate) fn positive(v: f64) -> bool {
  v.is_finite() && v > 0.0
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn positive_accepts_only_finite_numbers_above_zero() {
    assert!(positive(1e-9));
    assert!(positive(1.0));
    assert!(!positive(0.0));
    assert!(!positive(-1.0));
    assert!(!positive(f64::NAN));
    assert!(!positive(f64::INFINITY));
    assert!(!positive(f64::NEG_INFINITY));
  }
}
