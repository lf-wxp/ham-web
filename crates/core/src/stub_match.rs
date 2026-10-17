//! 单短截线匹配：给定归一化负载阻抗，给出短路 / 开路短截线的接入点与长度。
//!
//! 短截线是业余台站最常用、最省事的匹配手段之一（在天线馈点并一段开路 / 短路线）。
//! 这里在**导纳域**求解：沿传输线从负载向源移动，直到归一化电导的实部回到 1，
//! 再用一段短截线抵消掉剩余的电纳。距离与长度都按**波长数**返回（`0..0.5`），
//! 换算成物理长度由界面按频率与速率因子负责。
//!
//! # 口径
//!
//! - 只做**并联**单短截线（shunt stub，最常用），不做串联短截线；
//! - 短截线可为**开路**或**短路**，长度取最短的正长度（tan / cot 周期 0.5λ）；
//! - 纯电抗负载（`r ≤ 0`，驻波无穷大）无法用无耗短截线匹配，返回 `None`。
//!
//! 验证用「往返」：由负载算出的 `(d, l)` 代回传输线方程，断言并到短截线后的总
//! 导纳回到 `1 + j0` —— 这独立于求解过程，任一公式抄错都会立刻红。

/// 短截线类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StubKind {
  /// 开路短截线。
  Open,
  /// 短路短截线。
  Short,
}

/// 一组匹配解。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StubMatch {
  /// 接入点距负载的距离（波长数，`0..0.5`）。
  pub distance_wl: f64,
  /// 短截线长度（波长数，`0..0.5`）。
  pub stub_length_wl: f64,
  /// 短截线类型。
  pub kind: StubKind,
}

const TAU: f64 = std::f64::consts::TAU;
const EPS: f64 = 1e-12;

/// 归一化负载 `r + jx` 的导纳 `g + jb`。
fn admittance(r: f64, x: f64) -> (f64, f64) {
  let d = r * r + x * x;
  (r / d, -x / d)
}

/// 由 `t = tan(βd)` 求接入点距离（波长数，`0..0.5`）。
fn distance_from_t(t: f64) -> f64 {
  (t.atan2(1.0) / TAU).rem_euclid(0.5)
}

/// 短截线长度：抵消电纳 `b`（`b` 是接入点的归一化电纳）。
fn stub_length(b: f64, kind: StubKind) -> f64 {
  let theta = match kind {
    // 短路：-cot(βl) = -b → cot(βl) = b → l = arctan(1/b)。
    StubKind::Short => 1.0f64.atan2(b),
    // 开路：tan(βl) = -b → l = arctan(-b)。
    StubKind::Open => (-b).atan2(1.0),
  };
  (theta / TAU).rem_euclid(0.5)
}

/// 单短截线匹配。
///
/// `r` / `x` 是**归一化**到特性阻抗的负载（`zL = ZL/Z0 = r + jx`）。
/// 返回短截线较短的那个解；已匹配的负载返回 `(0, 0)`，不可匹配（`r ≤ 0`）返回 `None`。
#[must_use]
pub fn single_stub(r: f64, x: f64, kind: StubKind) -> Option<StubMatch> {
  if !r.is_finite() || !x.is_finite() || r <= 0.0 {
    return None;
  }
  if (r - 1.0).abs() < 1e-9 && x.abs() < 1e-9 {
    return Some(StubMatch {
      distance_wl: 0.0,
      stub_length_wl: 0.0,
      kind,
    });
  }
  let (g, b) = admittance(r, x);
  // 解 `Re[y(d)] = 1` 的二次方程：A·t² + B·t + C = 0。
  // A = b² + g² − g，B = −2b，C = 1 − g；判别式 = 4·g·((g−1)² + b²) ≥ 0。
  let a = b * b + g * g - g;
  let num = (g * ((g - 1.0).powi(2) + b * b)).sqrt();
  let ts: Vec<f64> = if a.abs() < EPS {
    // 退化：A ≈ 0，退化为线性方程（g ≈ 1 且 b 很小）。
    let lin = (1.0 - g) / (2.0 * b);
    if lin.is_finite() {
      vec![lin]
    } else {
      return None;
    }
  } else {
    vec![(b + num) / a, (b - num) / a]
  };

  let mut best: Option<StubMatch> = None;
  for t in ts {
    if !t.is_finite() {
      continue;
    }
    let distance_wl = distance_from_t(t);
    // 在距离 d 处重新算导纳，取虚部（接入点电纳）。
    let b_sus = y_susceptance(g, b, t);
    let stub_length_wl = stub_length(b_sus, kind);
    let cand = StubMatch {
      distance_wl,
      stub_length_wl,
      kind,
    };
    if best.is_none_or(|cur| cand.stub_length_wl < cur.stub_length_wl) {
      best = Some(cand);
    }
  }
  best
}

/// 接入点（`t = tan(βd)`）处的归一化电纳：`y(d) = (g + j(b+t)) / (1 + j(g+jb)t)` 的虚部。
fn y_susceptance(g: f64, b: f64, t: f64) -> f64 {
  let (nr, ni) = (g, b + t);
  let (dr, di) = (1.0 - b * t, g * t);
  let den = dr * dr + di * di;
  (ni * dr - nr * di) / den
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 往返验证：把 `(d, l)` 代回传输线方程，并上短截线后的总导纳应回到 `1 + j0`。
  fn matched_y(r: f64, x: f64, m: &StubMatch) -> (f64, f64) {
    let (g, b) = admittance(r, x);
    let t = (TAU * m.distance_wl).tan();
    let y = {
      let (nr, ni) = (g, b + t);
      let (dr, di) = (1.0 - b * t, g * t);
      let den = dr * dr + di * di;
      ((nr * dr + ni * di) / den, (ni * dr - nr * di) / den)
    };
    let beta_l = TAU * m.stub_length_wl;
    let y_stub = match m.kind {
      StubKind::Open => beta_l.tan(),
      StubKind::Short => -beta_l.tan().recip(),
    };
    (y.0, y.1 + y_stub)
  }

  fn assert_matched(r: f64, x: f64, kind: StubKind) {
    let m = single_stub(r, x, kind).expect("应可匹配");
    let (g, b) = matched_y(r, x, &m);
    assert!(
      (g - 1.0).abs() < 1e-6 && b.abs() < 1e-6,
      "{r}+j{x}: y={g}+j{b}"
    );
  }

  #[test]
  fn round_trip_matches_various_loads() {
    // 经典算例：zL = 2 + j0（VSWR = 2）→ 短路线 d = l = 0.152λ（并上后回到 50 Ω）。
    let m = single_stub(2.0, 0.0, StubKind::Short).unwrap();
    assert!((m.distance_wl - 0.152).abs() < 1e-3, "{:?}", m);
    assert!((m.stub_length_wl - 0.152).abs() < 1e-3, "{:?}", m);

    // 网格往返：开路 / 短路 × 若干负载，全部应匹配回 1 + j0。
    for &(r, x) in &[
      (2.0, 0.0),
      (1.0, 1.0),
      (0.5, 0.5),
      (0.3, -0.7),
      (1.5, -0.3),
      (3.0, 1.5),
    ] {
      assert_matched(r, x, StubKind::Short);
      assert_matched(r, x, StubKind::Open);
    }
  }

  #[test]
  fn already_matched_load_needs_no_stub() {
    let m = single_stub(1.0, 0.0, StubKind::Short).unwrap();
    assert_eq!(m.distance_wl, 0.0);
    assert_eq!(m.stub_length_wl, 0.0);
  }

  #[test]
  fn unmatched_or_pure_reactance_load_is_none() {
    assert_eq!(single_stub(0.0, 0.0, StubKind::Short), None);
    assert_eq!(single_stub(0.0, 1.0, StubKind::Open), None);
    assert_eq!(single_stub(-1.0, 0.5, StubKind::Short), None);
    assert_eq!(single_stub(f64::NAN, 0.5, StubKind::Short), None);
  }

  #[test]
  fn stub_lengths_stay_in_half_wavelength() {
    for &(r, x) in &[(2.0, 0.0), (0.5, -0.5), (3.0, 2.0)] {
      for kind in [StubKind::Short, StubKind::Open] {
        let m = single_stub(r, x, kind).unwrap();
        assert!((0.0..0.5).contains(&m.distance_wl), "{:?}", m);
        assert!((0.0..0.5).contains(&m.stub_length_wl), "{:?}", m);
      }
    }
  }
}
