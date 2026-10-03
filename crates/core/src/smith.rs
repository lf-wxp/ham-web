//! 史密斯圆图：复阻抗到反射系数平面的映射与采样绘制。

/// 由归一化阻抗 z = r + jx 计算反射系数 Γ（实部，虚部）。
pub fn gamma(r: f64, x: f64) -> (f64, f64) {
  let denom = (r + 1.0) * (r + 1.0) + x * x;
  let gr = (r * r - 1.0 + x * x) / denom;
  let gi = 2.0 * x / denom;
  (gr, gi)
}

/// 由反射系数计算驻波比与回波损耗（dB）。
pub fn swr_and_return_loss(gr: f64, gi: f64) -> (f64, f64) {
  let mag = (gr * gr + gi * gi).sqrt().min(1.0);
  let swr = (1.0 + mag) / (1.0 - mag).max(1e-9);
  let rl = -20.0 * mag.max(1e-9).log10();
  (swr, rl)
}

/// 等电阻圆（归一化 r 固定）的 Γ 采样点，用于绘制。
pub fn resistance_circle(r: f64, n: usize) -> Vec<(f64, f64)> {
  let mut pts = Vec::with_capacity(n + 1);
  for i in 0..=n {
    let phi = std::f64::consts::PI * 0.49 * (2.0 * i as f64 / n as f64 - 1.0);
    pts.push(gamma(r, phi.tan()));
  }
  pts
}

/// 等电抗弧（归一化 x 固定）的 Γ 采样点，用于绘制。
pub fn reactance_arc(x: f64, n: usize) -> Vec<(f64, f64)> {
  let mut pts = Vec::with_capacity(n + 1);
  for i in 0..=n {
    let frac = i as f64 / n as f64;
    let r = frac / (1.0 - frac).max(1e-4);
    pts.push(gamma(r, x));
  }
  pts
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn gamma_known_points() {
    // 短路 z=0 → Γ=-1。
    let (gr, gi) = gamma(0.0, 0.0);
    assert!((gr + 1.0).abs() < 1e-9);
    assert!(gi.abs() < 1e-9);
    // 匹配 z=1 → Γ=0。
    let (gr, gi) = gamma(1.0, 0.0);
    assert!(gr.abs() < 1e-9 && gi.abs() < 1e-9);
    // 纯感抗 z=j1 → Γ=(0,1)，位于上半平面。
    let (gr, gi) = gamma(0.0, 1.0);
    assert!(gr.abs() < 1e-9);
    assert!((gi - 1.0).abs() < 1e-9);
  }

  #[test]
  fn swr_values() {
    // 匹配 Γ=0 → SWR=1，回波损耗很大。
    let (swr, rl) = swr_and_return_loss(0.0, 0.0);
    assert!((swr - 1.0).abs() < 1e-6);
    assert!(rl > 60.0);
    // 全反射 Γ=1 → SWR 很大。
    let (swr, _) = swr_and_return_loss(1.0, 0.0);
    assert!(swr > 1000.0);
  }

  #[test]
  fn samples_within_unit_circle() {
    for r in [0.5, 1.0, 2.0, 5.0] {
      for (gr, gi) in resistance_circle(r, 64) {
        assert!(gr * gr + gi * gi <= 1.0 + 1e-6);
      }
    }
    for x in [-5.0, -1.0, 1.0, 5.0] {
      for (gr, gi) in reactance_arc(x, 64) {
        assert!(gr * gr + gi * gi <= 1.0 + 1e-6);
      }
    }
  }
}
