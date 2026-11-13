//! Faddeeva 函数 `w(z) = exp(−z²)·erfc(−jz)`。
//!
//! 移植自 NTIA 的 `wofz.cpp`（原件注明转写自 ACM Algorithm 680 / GPM Poppe, CMJ Wijers,
//! *More efficient computation of the complex error function*, ACM TOMS 16(1):38–46, 1990）。
//! 只用于平地 + 曲率修正那一支（`q` 的模大于 0.1 时）。算法本身没有改动。

use crate::cx::Cx;

/// `2/√π`（原件写的就是这个值，用标准常量避免「近似值」告警）。
const FACTOR: f64 = std::f64::consts::FRAC_2_SQRT_PI;
const RMAX_REAL: f64 = 0.5e154;
const RMAX_EXP: f64 = 708.503_061_461_606;
const RMAX_GONI: f64 = 3.537_118_876_014_22e15;

/// `w(z)`。原件在 `|Re z|` 或 `|Im z|` 超过 `RMAX_REAL` 时返回 0（判为溢出）。
pub(super) fn wofz(z: Cx) -> Cx {
  let (xi, yi) = (z.re, z.im);
  let xabs = xi.abs();
  let yabs = yi.abs();
  let x = xabs / 6.3;
  let y = yabs / 4.4;

  if xabs > RMAX_REAL || yabs > RMAX_REAL {
    return Cx::ZERO;
  }

  let mut qrho = x * x + y * y;
  let xabsq = xabs * xabs;
  let mut xquad = xabsq - yabs * yabs;
  let yquad = 2.0 * xabs * yabs;
  let small = qrho < 0.085_264;

  // 小 |z|：幂级数（Abramowitz & Stegun 7.1.5，p.297）。
  let (mut u2, mut v2);
  let (mut u, mut v);
  if small {
    qrho = (1.0 - 0.85 * y) * qrho.sqrt();
    let n = (6.0 + 72.0 * qrho) as i32;
    let mut j = 2 * n + 1;
    let mut xsum = 1.0 / f64::from(j);
    let mut ysum = 0.0;
    for i in (1..=n).rev() {
      j -= 2;
      let xaux = (xsum * xquad - ysum * yquad) / f64::from(i);
      ysum = (xsum * yquad + ysum * xquad) / f64::from(i);
      xsum = xaux + 1.0 / f64::from(j);
    }
    let u1 = -FACTOR * (xsum * yabs + ysum * xabs) + 1.0;
    let v1 = FACTOR * (xsum * xabs - ysum * yabs);
    let daux = (-xquad).exp();
    u2 = daux * yquad.cos();
    v2 = -daux * yquad.sin();
    u = u1 * u2 - v1 * v2;
    v = u1 * v2 + v1 * u2;
  } else {
    // 连分式（|z|² > 1）或截断泰勒展开（介于两者之间）。
    let mut h = 0.0f64;
    let kapn: i32;
    let nu: i32;
    if qrho > 1.0 {
      kapn = 0;
      qrho = qrho.sqrt();
      nu = (3.0 + 1442.0 / (26.0 * qrho + 77.0)) as i32;
    } else {
      qrho = (1.0 - y) * (1.0 - qrho).sqrt();
      h = 1.88 * qrho;
      kapn = (7.0 + 34.0 * qrho) as i32;
      nu = (16.0 + 26.0 * qrho) as i32;
    }
    let h2 = 2.0 * h;
    let b = h > 0.0;
    let mut qlambda = if b { h2.powi(kapn) } else { 0.0 };
    let (mut rx, mut ry, mut sx, mut sy) = (0.0, 0.0, 0.0, 0.0);
    for n in (0..=nu).rev() {
      let np1 = f64::from(n + 1);
      let tx = yabs + h + np1 * rx;
      let ty = xabs - np1 * ry;
      let cc = 0.5 / (tx * tx + ty * ty);
      rx = cc * tx;
      ry = cc * ty;
      if b && n <= kapn {
        let tx = qlambda + sx;
        sx = rx * tx - ry * sy;
        sy = ry * tx + rx * sy;
        qlambda /= h2;
      }
    }
    if h == 0.0 {
      u = FACTOR * rx;
      v = FACTOR * ry;
    } else {
      u = FACTOR * sx;
      v = FACTOR * sy;
    }
    if yabs == 0.0 {
      u = (-xabs * xabs).exp();
    }
    u2 = 0.0;
    v2 = 0.0;
  }

  // 其它象限：按原件的镜像关系拼回去。
  if yi < 0.0 {
    if small {
      u2 *= 2.0;
      v2 *= 2.0;
    } else {
      xquad = -xquad;
      if yquad > RMAX_GONI || xquad > RMAX_EXP {
        return Cx::ZERO;
      }
      let w1 = 2.0 * xquad.exp();
      u2 = w1 * yquad.cos();
      v2 = -w1 * yquad.sin();
    }
    u = u2 - u;
    v = v2 - v;
    if xi > 0.0 {
      v = -v;
    }
  } else if xi < 0.0 {
    v = -v;
  }

  Cx::new(u, v)
}

#[cfg(test)]
mod tests {
  use super::*;

  /// `w(0) = 1`；`w(z)` 满足 `w(−z) = w(z) 的共轭镜像关系`（`w(-z)=2e^{-z²}-w(z)`）。
  #[test]
  fn known_values_and_symmetry() {
    let w0 = wofz(Cx::ZERO);
    assert!((w0.re - 1.0).abs() < 1e-12 && w0.im.abs() < 1e-12, "{w0:?}");

    for z in [
      Cx::new(0.5, 0.5),
      Cx::new(1.0, -2.0),
      Cx::new(-3.0, 4.0),
      Cx::new(8.0, 8.0),
      Cx::new(0.01, -0.02),
    ] {
      let a = wofz(z);
      let b = wofz(-z);
      // w(-z) = 2·exp(−z²) − w(z)
      let two = (z * z * -1.0).exp() * 2.0;
      let want = two - a;
      assert!(
        (b.re - want.re).abs() < 1e-10 && (b.im - want.im).abs() < 1e-10,
        "z={z:?}: {b:?} vs {want:?}"
      );
    }
  }

  /// 纯虚轴上有闭式解 `w(jy) = e^{y²}·erfc(y)`，用 CPython 的 `math.exp(y*y)*math.erfc(y)` 作参照。
  #[test]
  fn imaginary_axis_matches_the_real_erfc() {
    // 参照值由 `math.exp(y*y)*math.erfc(y)` 算出（CPython 3.12 的双精度 erfc）。
    let cases = [
      (0.1f64, 0.896_456_979_969_126_5),
      (0.5, 0.615_690_344_192_925_9),
      (1.0, 0.427_583_576_155_807_05),
      (3.0, 0.179_001_151_181_389_93),
    ];
    for (y, want) in cases {
      let got = wofz(Cx::new(0.0, y));
      assert!((got.re - want).abs() < 1e-13, "y={y}: {} vs {want}", got.re);
      assert!(got.im.abs() < 1e-13, "y={y}: Im = {}", got.im);
    }
  }
}
