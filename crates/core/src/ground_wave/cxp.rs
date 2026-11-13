//! 复数的极值运算：`powi` 与整数次幂。只在 `ground_wave` 里用到，不污染 `cx.rs` 的公共面。

use crate::cx::Cx;

/// 复数的整数次幂（`n ≥ 0`；`n < 0` 时先取倒数）。`0⁰` 返回 1。
///
/// 用平方求幂而不是 `exp(n·ln z)`：原 C++ 用的是 `pow(complex, double)`，
/// 在 `|z|` 跨好几个数量级时会在中间步骤丢精度；这里次数都很小（3 / 6 / 9）。
#[must_use]
pub(super) fn powi(z: Cx, n: i32) -> Cx {
  if n < 0 {
    return Cx::ONE / powi(z, -n);
  }
  let mut result = Cx::ONE;
  let mut base = z;
  let mut k = n;
  while k > 0 {
    if k & 1 == 1 {
      result *= base;
    }
    base *= base;
    k >>= 1;
  }
  result
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn integer_powers_match_repeated_multiplication() {
    let z = Cx::new(0.7, -1.3);
    for n in 0..=6 {
      let mut want = Cx::ONE;
      for _ in 0..n {
        want *= z;
      }
      let got = powi(z, n);
      assert!(
        (got.re - want.re).abs() < 1e-12 && (got.im - want.im).abs() < 1e-12,
        "n={n}"
      );
    }
    // 负次幂是倒数。
    let inv = powi(z, -2);
    let want = Cx::ONE / (z * z);
    assert!((inv.re - want.re).abs() < 1e-12 && (inv.im - want.im).abs() < 1e-12);
  }
}
