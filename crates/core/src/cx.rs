//! 极简复数：滤波器原型求值与矩量法求解共用，不引入任何外部依赖。
//!
//! 只实现这两个场景真正用到的运算（含 `exp`、`abs`、`arg`、`conj`），
//! 不做通用数学库。

use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub, SubAssign};

/// 双精度复数。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Cx {
  /// 实部。
  pub re: f64,
  /// 虚部。
  pub im: f64,
}

impl Cx {
  /// `0`。
  pub const ZERO: Self = Self { re: 0.0, im: 0.0 };
  /// `1`。
  pub const ONE: Self = Self { re: 1.0, im: 0.0 };
  /// `j`。
  pub const J: Self = Self { re: 0.0, im: 1.0 };

  /// 由实部与虚部构造。
  #[must_use]
  pub const fn new(re: f64, im: f64) -> Self {
    Self { re, im }
  }

  /// 实数量。
  #[must_use]
  pub const fn of(v: f64) -> Self {
    Self { re: v, im: 0.0 }
  }

  /// 纯虚数 `j·v`。
  #[must_use]
  pub const fn j(v: f64) -> Self {
    Self { re: 0.0, im: v }
  }

  /// 单位圆上的点 `exp(j·ang)`。
  #[must_use]
  pub fn new_polar(ang: f64) -> Self {
    Self {
      re: ang.cos(),
      im: ang.sin(),
    }
  }

  /// `e^z`。
  #[must_use]
  pub fn exp(self) -> Self {
    let m = self.re.exp();
    Self {
      re: m * self.im.cos(),
      im: m * self.im.sin(),
    }
  }

  /// 模。
  #[must_use]
  pub fn abs(self) -> f64 {
    self.re.hypot(self.im)
  }

  /// 模的平方（避免开方）。
  #[must_use]
  pub fn abs2(self) -> f64 {
    self.re * self.re + self.im * self.im
  }

  /// 辐角（弧度）。
  #[must_use]
  pub fn arg(self) -> f64 {
    self.im.atan2(self.re)
  }

  /// 共轭。
  #[must_use]
  pub fn conj(self) -> Self {
    Self {
      re: self.re,
      im: -self.im,
    }
  }

  /// 复数平方根（取主值）。
  ///
  /// 用「按实部符号分支」的写法而不是 `re^{jθ/2}`：有耗介质的相对介电常数
  /// 实部远大于虚部、且常常极为接近某个实数，极坐标形式会先在 `atan2` 上
  /// 丢有效位。
  #[must_use]
  pub fn sqrt(self) -> Self {
    let m = self.abs();
    if m == 0.0 {
      return Self::ZERO;
    }
    if self.re >= 0.0 {
      let u = ((m + self.re) / 2.0).sqrt();
      Self::new(u, self.im / (2.0 * u))
    } else {
      let v = ((m - self.re) / 2.0).sqrt() * if self.im < 0.0 { -1.0 } else { 1.0 };
      Self::new(self.im / (2.0 * v), v)
    }
  }
}

impl Add for Cx {
  type Output = Self;
  fn add(self, o: Self) -> Self {
    Self {
      re: self.re + o.re,
      im: self.im + o.im,
    }
  }
}

impl Sub for Cx {
  type Output = Self;
  fn sub(self, o: Self) -> Self {
    Self {
      re: self.re - o.re,
      im: self.im - o.im,
    }
  }
}

impl Neg for Cx {
  type Output = Self;
  fn neg(self) -> Self {
    Self {
      re: -self.re,
      im: -self.im,
    }
  }
}

impl Mul for Cx {
  type Output = Self;
  fn mul(self, o: Self) -> Self {
    Self {
      re: self.re * o.re - self.im * o.im,
      im: self.re * o.im + self.im * o.re,
    }
  }
}

impl Mul<f64> for Cx {
  type Output = Self;
  fn mul(self, k: f64) -> Self {
    Self {
      re: self.re * k,
      im: self.im * k,
    }
  }
}

/// 复数除法（`÷0` 会得到 `inf`/`NaN`，**调用方负责保证除数非零**）。
///
/// 不做零保护：这里在消元的热路径上，每个元素都判一次零是白掏开销，而所有调用点
/// （`solve_linear` 的主元、`Cx::of(volts) / i_feed`）在进入前都已判过。
impl Div for Cx {
  type Output = Self;
  fn div(self, o: Self) -> Self {
    let d = o.re * o.re + o.im * o.im;
    Self {
      re: (self.re * o.re + self.im * o.im) / d,
      im: (self.im * o.re - self.re * o.im) / d,
    }
  }
}

impl AddAssign for Cx {
  fn add_assign(&mut self, o: Self) {
    self.re += o.re;
    self.im += o.im;
  }
}

impl SubAssign for Cx {
  fn sub_assign(&mut self, o: Self) {
    self.re -= o.re;
    self.im -= o.im;
  }
}

impl MulAssign for Cx {
  fn mul_assign(&mut self, o: Self) {
    *self = *self * o;
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::f64::consts::PI;

  #[test]
  fn sqrt_squares_back_on_real_and_complex_arguments() {
    // 有耗介质的相对介电常数：实部远大于虚部，`sqrt` 不能在这类量上丢精度。
    for z in [
      Cx::new(13.0, -1.015),
      Cx::new(80.0, -300.0),
      Cx::new(1.0, 0.0),
      Cx::new(0.0, 4.0),
      Cx::new(-4.0, 0.0),
      Cx::new(-3.0, -4.0),
      Cx::new(1e-6, 0.0),
    ] {
      let r = z.sqrt();
      let back = r * r;
      assert!(
        (back.re - z.re).abs() < 1e-9 * z.abs().max(1.0)
          && (back.im - z.im).abs() < 1e-9 * z.abs().max(1.0),
        "{z:?} 的平方根 {r:?} 平方回去得到 {back:?}"
      );
      // 主值分支：实部非负。
      assert!(r.re >= 0.0);
    }
    assert_eq!(Cx::ZERO.sqrt(), Cx::ZERO);
  }

  #[test]
  fn multiplication_and_division_round_trip() {
    let a = Cx::new(3.0, -4.0);
    let b = Cx::new(-1.5, 2.5);
    let q = (a * b) / b;
    assert!((q.re - a.re).abs() < 1e-12 && (q.im - a.im).abs() < 1e-12);
  }

  #[test]
  fn exponential_matches_euler() {
    // e^{jπ} = −1。
    let v = Cx::j(PI).exp();
    assert!((v.re + 1.0).abs() < 1e-12 && v.im.abs() < 1e-12);
    // e^{jπ/2} = j。
    let v = Cx::j(PI / 2.0).exp();
    assert!(v.re.abs() < 1e-12 && (v.im - 1.0).abs() < 1e-12);
    // 实指数：e^{ln 2} = 2。
    let v = Cx::of(2.0f64.ln()).exp();
    assert!((v.re - 2.0).abs() < 1e-12 && v.im.abs() < 1e-12);
  }

  #[test]
  fn abs_arg_and_conjugate() {
    let a = Cx::new(0.0, 2.0);
    assert!((a.abs() - 2.0).abs() < 1e-12);
    assert!((a.abs2() - 4.0).abs() < 1e-12);
    assert!((a.arg() - PI / 2.0).abs() < 1e-12);
    let c = a.conj();
    assert!((c.im + 2.0).abs() < 1e-12);
    // |z|² = z·z̄。
    let z = Cx::new(-3.0, 5.0);
    assert!(((z * z.conj()).re - z.abs2()).abs() < 1e-12);
  }
}
