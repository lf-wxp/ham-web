//! 史密斯圆图：复阻抗到反射系数平面的映射与采样绘制。

/// 由归一化阻抗 z = r + jx 计算反射系数 Γ（实部，虚部）。
///
/// 定义域是 `r ≥ 0`（无源负载）。`z = -1`（`r = -1, x = 0`）是 Γ 的极点，除零会得到
/// NaN 并一路传染到 SWR：这里显式返回 `(NaN, NaN)`，让调用方有机会显示「—」，
/// 而不是把 NaN 静默算成一个巨大的驻波比。
pub fn gamma(r: f64, x: f64) -> (f64, f64) {
  let denom = (r + 1.0) * (r + 1.0) + x * x;
  if denom < 1e-12 {
    return (f64::NAN, f64::NAN);
  }
  let gr = (r * r - 1.0 + x * x) / denom;
  let gi = 2.0 * x / denom;
  (gr, gi)
}

/// 由反射系数计算驻波比与回波损耗（dB）。
///
/// 非法输入（NaN / Inf）**原样传出 NaN**：`f64::min` 遇 NaN 会取另一个操作数，
/// 于是 NaN 会被静默当成「Γ = 1」（SWR ≈ 2e9、RL = 0 dB）—— 数字看着正常，其实全错。
pub fn swr_and_return_loss(gr: f64, gi: f64) -> (f64, f64) {
  let mag = (gr * gr + gi * gi).sqrt();
  if !mag.is_finite() {
    return (f64::NAN, f64::NAN);
  }
  let mag = mag.min(1.0);
  let swr = (1.0 + mag) / (1.0 - mag).max(1e-9);
  let rl = -20.0 * mag.max(1e-9).log10();
  (swr, rl)
}

/// 等电阻圆（归一化 r 固定）的 Γ 采样点，用于绘制。
pub fn resistance_circle(r: f64, n: usize) -> Vec<(f64, f64)> {
  // `n == 0` 会让除数变成 0（采样点全是 NaN），夹到 1（同 `constant_r_arc` 的写法）。
  let n = n.max(1);
  let mut pts = Vec::with_capacity(n + 1);
  for i in 0..=n {
    let phi = std::f64::consts::PI * 0.49 * (2.0 * i as f64 / n as f64 - 1.0);
    pts.push(gamma(r, phi.tan()));
  }
  pts
}

/// 等电抗弧（归一化 x 固定）的 Γ 采样点，用于绘制。
pub fn reactance_arc(x: f64, n: usize) -> Vec<(f64, f64)> {
  let n = n.max(1);
  let mut pts = Vec::with_capacity(n + 1);
  for i in 0..=n {
    let frac = i as f64 / n as f64;
    let r = frac / (1.0 - frac).max(1e-4);
    pts.push(gamma(r, x));
  }
  pts
}

/// 由反射系数反解归一化阻抗：`z = (1+Γ)/(1−Γ)`。
///
/// 圆图上拖动指针走的是这条路径：像素坐标 → Γ → z。`Γ = 1`（全反射开路的
/// 边界）处解不存在，返回 `None` 而不是造一个无穷大的阻抗。
pub fn z_from_gamma(gr: f64, gi: f64) -> Option<(f64, f64)> {
  let d = (1.0 - gr) * (1.0 - gr) + gi * gi;
  if d < 1e-12 {
    return None;
  }
  let r = (1.0 - gr * gr - gi * gi) / d;
  let x = 2.0 * gi / d;
  Some((r, x))
}

/// 串入电抗 `x_add`（归一化）后的阻抗。
///
/// 串联元件不改变电阻分量，轨迹是圆图上的一条**等电阻圆**。
#[must_use]
pub fn series_reactance(z: (f64, f64), x_add: f64) -> (f64, f64) {
  (z.0, z.1 + x_add)
}

/// 并入电纳 `b_add`（归一化）后的阻抗。
///
/// 并联元件不改变电导分量，轨迹是圆图上的一条**等电导圆**。
#[must_use]
pub fn shunt_susceptance(z: (f64, f64), b_add: f64) -> (f64, f64) {
  let (r, x) = z;
  let d = r * r + x * x;
  if d < 1e-12 {
    // 短路点：并联任何元件仍是短路（无法从 Γ = −1 处分辨）。
    return (0.0, 0.0);
  }
  let (g, b) = (r / d, -x / d);
  let bp = b + b_add;
  let dp = g * g + bp * bp;
  if dp < 1e-12 {
    return (0.0, 0.0);
  }
  (g / dp, -bp / dp)
}

/// 等电阻圆（r 固定、x 从 `x_from` 变到 `x_to`）的 Γ 采样点。
///
/// 串联电抗的轨迹用它绘制。
#[must_use]
pub fn constant_r_arc(r: f64, x_from: f64, x_to: f64, n: usize) -> Vec<(f64, f64)> {
  let n = n.max(1);
  (0..=n)
    .map(|i| {
      let x = x_from + (x_to - x_from) * i as f64 / n as f64;
      gamma(r, x)
    })
    .collect()
}

/// 等电导圆（g 固定、b 从 `b_from` 变到 `b_to`）的 Γ 采样点。
///
/// 并联电纳的轨迹用它绘制。导纳平面上的点 `y = g + jb` 在 Γ 平面上落在
/// `-Γ(g, b)` —— 因为 `Γ_y = (y−1)/(y+1) = (1/z−1)/(1/z+1) = −Γ_z`。
#[must_use]
pub fn constant_g_arc(g: f64, b_from: f64, b_to: f64, n: usize) -> Vec<(f64, f64)> {
  constant_r_arc(g, b_from, b_to, n)
    .into_iter()
    .map(|(gr, gi)| (-gr, -gi))
    .collect()
}

/// 理想传输线（特性阻抗 `z0`）的输入阻抗：向信号源方向旋转 `electrical_deg` 度电长度。
///
/// `z_in = (z + j·t)/(1 + j·z·t)`，`t = tan(θ)`。在圆图上表现为绕圆心
/// **顺时针**旋转 —— 这就是「向信号源走」的方向。
#[must_use]
pub fn line_rotate(z: (f64, f64), electrical_deg: f64) -> (f64, f64) {
  let t = electrical_deg.to_radians().tan();
  let (r, x) = z;
  let num = (r, x + t);
  let den = (1.0 - x * t, r * t);
  let d = den.0 * den.0 + den.1 * den.1;
  if d < 1e-12 {
    return (r, x);
  }
  (
    (num.0 * den.0 + num.1 * den.1) / d,
    (num.1 * den.0 - num.0 * den.1) / d,
  )
}

/// L 型匹配网络的一组解（全部为对 `z0` 归一化后的值）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LMatch {
  /// `true` 表示先并后串（负载电阻低于 z0 时），`false` 表示先串后并。
  pub shunt_first: bool,
  /// 需要加入的并联电纳（归一化；正值 = 容性）。
  pub b: f64,
  /// 需要加入的串联电抗（归一化；正值 = 感性）。
  pub x: f64,
}

/// 设计 L 型匹配网络：把归一化负载 `z = r + jx` 匹配到 1。
///
/// 两组解对应「高 Q / 低 Q」两条路径，实际择一时看元件值是否可实现、
/// 以及带宽（低 Q 的那条更宽）。`r` 接近 0（近似短路）时无解，返回 `None`。
///
/// 推导：先在负载并联电纳使**剩余阻抗的实部为 1**，再串入电抗抵消虚部
/// （`r > 1` 时）；`r < 1` 时顺序相反（先串联、后并联）。
#[must_use]
pub fn l_match(r: f64, x: f64) -> Option<[LMatch; 2]> {
  if r < 1e-6 {
    return None;
  }
  if r >= 1.0 {
    // 负载在高阻侧：先并联电纳把电导降到 ≤1，再串联电抗抵消虚部。
    let d = r * r + x * x;
    let (g, b) = (r / d, -x / d);
    let disc = (g - g * g).max(0.0).sqrt();
    let sol = |bp: f64| LMatch {
      shunt_first: true,
      b: bp - b,
      x: bp / g,
    };
    Some([sol(disc), sol(-disc)])
  } else {
    // 负载在低阻侧：先串联电抗把电导降到 ≤1，再并联电纳抵消虚部。
    let disc = (r - r * r).max(0.0).sqrt();
    let sol = |xp: f64| LMatch {
      shunt_first: false,
      b: xp / r,
      x: xp - x,
    };
    Some([sol(disc), sol(-disc)])
  }
}

/// 由归一化电抗 / 电纳折算出的实际元件。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Element {
  /// 电感（μH）。
  pub l_uh: Option<f64>,
  /// 电容（pF）。
  pub c_pf: Option<f64>,
}

/// 串联电抗折算元件：`x > 0` 为电感，`x < 0` 为电容，`x = 0` 表示无需元件。
///
/// `freq_hz` 必须为正（元件值随频率换算，`f ≤ 0` 无意义）：调用方负责在边界处夹取
/// （见 `pages::smith` 的频率输入，已 `clamp(0.1, 6000)` MHz）。
#[must_use]
pub fn series_element(x: f64, freq_hz: f64, z0: f64) -> Element {
  let w = std::f64::consts::TAU * freq_hz.max(1.0);
  if x > 0.0 {
    Element {
      l_uh: Some(x * z0 / w * 1e6),
      c_pf: None,
    }
  } else if x < 0.0 {
    Element {
      l_uh: None,
      c_pf: Some(1.0 / (w * x.abs() * z0) * 1e12),
    }
  } else {
    Element {
      l_uh: None,
      c_pf: None,
    }
  }
}

/// 并联电纳折算元件：`b > 0` 为电容，`b < 0` 为电感，`b = 0` 表示无需元件。
///
/// `freq_hz` 必须为正，同 [`series_element`]。
#[must_use]
pub fn shunt_element(b: f64, freq_hz: f64, z0: f64) -> Element {
  let w = std::f64::consts::TAU * freq_hz.max(1.0);
  if b > 0.0 {
    Element {
      l_uh: None,
      c_pf: Some(b / (w * z0) * 1e12),
    }
  } else if b < 0.0 {
    Element {
      l_uh: Some(z0 / (w * b.abs()) * 1e6),
      c_pf: None,
    }
  } else {
    Element {
      l_uh: None,
      c_pf: None,
    }
  }
}

/// 在阻抗 `z`（Ω）上**并联**电纳 `b`（S）后的输入阻抗。
///
/// 并联走导纳加法：`Y' = 1/z + jb`，再取倒数。短路点（`z = 0`）并联任何
/// 元件仍是短路，直接返回 0。
#[must_use]
fn add_shunt(z: (f64, f64), b: f64) -> (f64, f64) {
  let d = z.0 * z.0 + z.1 * z.1;
  if d < 1e-12 {
    return (0.0, 0.0);
  }
  let g = z.0 / d;
  let yb = -z.1 / d + b;
  let dp = g * g + yb * yb;
  if dp < 1e-12 {
    return (0.0, 0.0);
  }
  (g / dp, -yb / dp)
}

/// 元件值随频率的缩放因子。
///
/// 电感电抗 `X_L = ωL ∝ f`、电容电抗 `X_C = −1/(ωC) ∝ 1/f`；电纳正好相反
/// （`B_C = ωC ∝ f`、`B_L = −1/(ωL) ∝ 1/f`）。两种量的**符号唯一决定元件类型**
/// —— 电抗为正必是电感、电纳为正必是电容 —— 因此这里只看符号，不需要额外的
/// 元件类型参数。`k = f / f0`。
#[must_use]
pub fn element_scale(value: f64, k: f64) -> f64 {
  let k = k.max(1e-9);
  if value >= 0.0 { value * k } else { value / k }
}

/// 匹配网络在频率比 `k = f / f0` 处的输入阻抗（Ω）。
///
/// `series_x0` 为 `f0` 处的串联电抗（Ω），`shunt_b0` 为 `f0` 处的并联电纳（S）；
/// `shunt_first = true` 表示并联元件在负载侧（先并后串），否则并联元件在源侧
/// （先串后并，并联跨在输入端）。
///
/// **负载按与频率无关处理** —— 这是理想化假设。真实天线的阻抗会随频率变化，
/// 因此这里算出的带宽偏乐观，只适合比较不同匹配方案相对宽窄。
#[must_use]
pub fn network_input_z(
  load: (f64, f64),
  series_x0: f64,
  shunt_b0: f64,
  shunt_first: bool,
  k: f64,
) -> (f64, f64) {
  let x = element_scale(series_x0, k);
  let b = element_scale(shunt_b0, k);
  if shunt_first {
    // 负载侧并联 → 源侧串联。
    let z1 = add_shunt(load, b);
    (z1.0, z1.1 + x)
  } else {
    // 负载侧串联 → 源侧并联（并联跨在输入端，与源并联）。
    add_shunt((load.0, load.1 + x), b)
  }
}

/// 匹配网络在频率比 `k` 处的驻波比（相对特性阻抗 `z0`）。
#[must_use]
pub fn network_swr(
  load: (f64, f64),
  series_x0: f64,
  shunt_b0: f64,
  shunt_first: bool,
  z0: f64,
  k: f64,
) -> f64 {
  let z = network_input_z(load, series_x0, shunt_b0, shunt_first, k);
  let (gr, gi) = gamma(z.0 / z0, z.1 / z0);
  swr_and_return_loss(gr, gi).0
}

/// 扫描频率比的下限 / 上限（各 3 个倍频程）。匹配网络的可用带宽极少超过这个范围，
/// 真超出时结果会被夹到边界（调用方按「更宽」理解即可）。
const BAND_K_LO: f64 = 0.125;
/// 见 [`BAND_K_LO`]。
const BAND_K_HI: f64 = 8.0;
/// 粗扫点数：对数刻度下相邻点相差约 0.7%，足以分辨常见的匹配带宽。
const BAND_SCAN: usize = 600;

/// 匹配带宽：输入驻波比不超过 `swr_limit` 的**连续**频率区间（Hz）。
///
/// 在 `f0` 两侧按对数刻度粗扫，取包含 `f0` 的那一段连续区间，再用二分法把两个
/// 边界收紧。返回 `None` 表示 `f0` 处本身就没达标（例如元件是手调的、并未真正
/// 匹配到 50Ω）—— 这时谈「带宽」没有意义。
///
/// 元件按自身频率特性缩放（见 [`element_scale`]），负载按与频率无关处理。
#[must_use]
pub fn matched_bandwidth(
  load: (f64, f64),
  series_x0: f64,
  shunt_b0: f64,
  shunt_first: bool,
  f0_hz: f64,
  z0: f64,
  swr_limit: f64,
) -> Option<(f64, f64)> {
  let swr_at = |k: f64| network_swr(load, series_x0, shunt_b0, shunt_first, z0, k);
  let limit = swr_limit.max(1.0);
  // 中心点先判一次：不达标（或算出非有限值）时谈带宽没有意义。
  let at_center = swr_at(1.0);
  if !at_center.is_finite() || at_center > limit {
    return None;
  }

  let ratio = (BAND_K_HI / BAND_K_LO).ln();
  let ks: Vec<f64> = (0..BAND_SCAN)
    .map(|i| BAND_K_LO * (ratio * i as f64 / (BAND_SCAN - 1) as f64).exp())
    .collect();
  let in_band: Vec<bool> = ks.iter().map(|&k| swr_at(k) <= limit).collect();

  // 粗扫点里一定有 k = 1 的邻居而不一定有 k = 1 本身，用最接近 k = 1 的带内点起步。
  let start = ks
    .iter()
    .enumerate()
    .filter(|&(i, _)| in_band[i])
    .min_by(|a, b| (a.1 - 1.0).abs().total_cmp(&(b.1 - 1.0).abs()))
    .map(|(i, _)| i)?;

  let lo_i = (0..=start).rev().find(|&i| !in_band[i]);
  let hi_i = (start..BAND_SCAN).find(|&i| !in_band[i]);

  // 二分收紧：`in` 在带内、`out` 在带外，逐步逼近交界。
  let refine = |mut inside: f64, mut outside: f64| -> f64 {
    for _ in 0..48 {
      let mid = 0.5 * (inside + outside);
      if swr_at(mid) <= limit {
        inside = mid;
      } else {
        outside = mid;
      }
    }
    inside
  };

  let k_lo = match lo_i {
    Some(i) => refine(ks[i + 1], ks[i]),
    None => BAND_K_LO,
  };
  let k_hi = match hi_i {
    Some(i) => refine(ks[i - 1], ks[i]),
    None => BAND_K_HI,
  };
  Some((f0_hz * k_lo, f0_hz * k_hi))
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

  #[test]
  fn gamma_round_trip() {
    for &(r, x) in &[(0.2, -1.0), (1.0, 0.0), (2.0, 1.0), (5.0, -3.0)] {
      let (gr, gi) = gamma(r, x);
      let (r2, x2) = z_from_gamma(gr, gi).expect("非全反射点应可反解");
      assert!(
        (r2 - r).abs() < 1e-9 && (x2 - x).abs() < 1e-9,
        "{r} {x} → {r2} {x2}"
      );
    }
    // Γ = 1（开路边界）无解，不能返回无穷大。
    assert_eq!(z_from_gamma(1.0, 0.0), None);
  }

  #[test]
  fn series_reactance_keeps_resistance() {
    let z = (2.0, 0.5);
    assert_eq!(series_reactance(z, -1.5), (2.0, -1.0));
    // 轨迹点应落在同一个等电阻圆上（x 恰好经过中点）。
    let arc = constant_r_arc(2.0, 0.5, -1.0, 8);
    for (gr, gi) in arc {
      let (r, _) = z_from_gamma(gr, gi).unwrap();
      assert!((r - 2.0).abs() < 1e-9, "等电阻圆上 r 应为 2，得到 {r}");
    }
  }

  #[test]
  fn shunt_susceptance_keeps_conductance() {
    let z = (0.5, 0.5);
    // 先串联到 y 的实部为 1，再并联 -1 使其回到中心。
    let y_re = |z: (f64, f64)| {
      let d = z.0 * z.0 + z.1 * z.1;
      z.0 / d
    };
    let z2 = shunt_susceptance(z, 0.5);
    assert!((y_re(z2) - y_re(z)).abs() < 1e-12, "并联不改变电导");
    // 等电导圆上的点，其导纳实部应恒为该 g。
    let g = y_re((0.5, 0.5));
    for b in [-2.0, -0.5, 0.5, 2.0] {
      for (gr, gi) in constant_g_arc(g, b, b + 1.5, 6) {
        let (r, x) = z_from_gamma(gr, gi).unwrap();
        let d = r * r + x * x;
        assert!((r / d - g).abs() < 1e-9, "等电导圆上 g 应为 {g}");
      }
    }
  }

  #[test]
  fn quarter_wave_line_inverts_impedance() {
    // 90° 电长度的理想线是阻抗倒相器：z = 2 → z_in = 0.5。
    let (r, x) = line_rotate((2.0, 0.0), 90.0);
    assert!((r - 0.5).abs() < 1e-9, "实部 {r}");
    assert!(x.abs() < 1e-9, "虚部 {x}");
    // 匹配点旋转后仍是匹配点。
    let (r0, x0) = line_rotate((1.0, 0.0), 37.0);
    assert!((r0 - 1.0).abs() < 1e-9 && x0.abs() < 1e-9);
  }

  /// 把一组 L 型解施加到负载上，返回匹配后的归一化阻抗。
  fn apply_l_match(r: f64, x: f64, m: &LMatch) -> (f64, f64) {
    if m.shunt_first {
      let after_shunt = shunt_susceptance((r, x), m.b);
      series_reactance(after_shunt, m.x)
    } else {
      let after_series = series_reactance((r, x), m.x);
      shunt_susceptance(after_series, m.b)
    }
  }

  #[test]
  fn l_match_matches_both_high_and_low_impedance_loads() {
    for &(r, x) in &[(2.0, 1.0), (3.0, -2.0), (0.5, 0.0), (0.2, 0.8), (1.0, 1.5)] {
      let sols = l_match(r, x).expect("可匹配负载应给出解");
      for m in &sols {
        let z = apply_l_match(r, x, m);
        let (gr, gi) = gamma(z.0, z.1);
        let mag = (gr * gr + gi * gi).sqrt();
        assert!(mag < 1e-9, "负载 {r}+j{x} 匹配后 |Γ| = {mag}（解 {m:?}）");
      }
      // 拓扑与负载所在侧一致：高阻侧先并、低阻侧先串。
      assert_eq!(sols[0].shunt_first, r >= 1.0);
      assert_eq!(sols[1].shunt_first, sols[0].shunt_first);
    }
    // 近似短路无法用 L 网络匹配（需要变换到极高的 Q）。
    assert_eq!(l_match(0.0, 0.0), None);
  }

  #[test]
  fn element_values_follow_reactance_formulas() {
    // 7.1 MHz、50Ω：x = 1 → L = 50/ω = 1.1208 µH。
    let e = series_element(1.0, 7.1e6, 50.0);
    let l = e.l_uh.expect("感性应为电感");
    assert!((l - 1.1208).abs() < 1e-3, "L = {l} µH");
    assert!(e.c_pf.is_none());
    // x = -1（即 X = -50Ω）→ C = 1/(ω·|x|·Z0) = 448.3 pF。
    let c = series_element(-1.0, 7.1e6, 50.0)
      .c_pf
      .expect("容性应为电容");
    assert!((c - 448.32).abs() < 0.05, "C = {c} pF");
    // b = 0.01 → C = 4.483 pF；b = -0.01 → L = 112.1 µH。
    let bc = shunt_element(0.01, 7.1e6, 50.0).c_pf.expect("正电纳为电容");
    assert!((bc - 4.483).abs() < 0.01, "C = {bc} pF");
    let bl = shunt_element(-0.01, 7.1e6, 50.0)
      .l_uh
      .expect("负电纳为电感");
    assert!((bl - 112.08).abs() < 0.1, "L = {bl} µH");
    // 零元件表示无需接入。
    assert_eq!(series_element(0.0, 7.1e6, 50.0).l_uh, None);
  }

  #[test]
  fn element_scale_follows_reactance_laws() {
    // 电感：k = 2 时电抗翻倍；电容：k = 2 时电抗减半（电抗为负）。
    assert!((element_scale(100.0, 2.0) - 200.0).abs() < 1e-9);
    assert!((element_scale(-100.0, 2.0) + 50.0).abs() < 1e-9);
    // 电纳符号相反：正电纳（电容）随频率线性增大，负电纳（电感）减小。
    assert!((element_scale(0.02, 2.0) - 0.04).abs() < 1e-12);
    assert!((element_scale(-0.02, 2.0) + 0.01).abs() < 1e-12);
    // k = 1 时是恒等变换。
    assert!((element_scale(-100.0, 1.0) + 100.0).abs() < 1e-12);
  }

  #[test]
  fn network_input_z_matches_normalized_math_at_f0() {
    // 归一化域与 Ω 域的运算必须一致：f0 处网络输入阻抗应等于 z × Z0。
    let z0 = 50.0;
    for &(r, x) in &[(2.0, -1.0), (0.4, 0.8), (1.0, 0.0)] {
      let sols = l_match(r, x).expect("可匹配负载");
      for m in &sols {
        let got = network_input_z((r * z0, x * z0), m.x * z0, m.b / z0, m.shunt_first, 1.0);
        assert!(
          (got.0 - z0).abs() < 1e-6 && got.1.abs() < 1e-6,
          "负载 {r}+j{x} 在 f0 处应匹配到 {z0}Ω，实际 {got:?}"
        );
      }
    }
  }

  #[test]
  fn network_swr_rises_on_both_sides_of_center() {
    let z0 = 50.0;
    let load = (100.0, -50.0);
    let m = l_match(load.0 / z0, load.1 / z0).unwrap()[0];
    let swr = |k: f64| network_swr(load, m.x * z0, m.b / z0, m.shunt_first, z0, k);
    assert!(swr(1.0) < 1.000_001, "f0 处应匹配：{}", swr(1.0));
    assert!(swr(0.5) > 1.0 && swr(2.0) > 1.0, "两侧都应变差");
    // 单调性只在近端成立，这里取离中心不太远的点，避免撞上带外的再入谐振。
    assert!(swr(1.2) > swr(1.0) && swr(0.8) > swr(1.0));
  }

  #[test]
  fn matched_bandwidth_is_contiguous_and_brackets_swr_limit() {
    let z0 = 50.0;
    let f0 = 7.1e6;
    let load = (100.0, -50.0);
    let m = l_match(load.0 / z0, load.1 / z0).unwrap()[0];
    let (lo, hi) = matched_bandwidth(load, m.x * z0, m.b / z0, m.shunt_first, f0, z0, 2.0)
      .expect("已匹配的网络必有带宽");
    assert!(lo < f0 && f0 < hi, "{lo} < {f0} < {hi}");
    // 边界处 SWR 应贴着限值；带内取样应全部达标。
    let at = |f: f64| {
      let z = network_input_z(load, m.x * z0, m.b / z0, m.shunt_first, f / f0);
      let (gr, gi) = gamma(z.0 / z0, z.1 / z0);
      swr_and_return_loss(gr, gi).0
    };
    assert!((at(lo) - 2.0).abs() < 1e-3, "下边界 SWR {}", at(lo));
    assert!((at(hi) - 2.0).abs() < 1e-3, "上边界 SWR {}", at(hi));
    for i in 1..20 {
      let f = lo + (hi - lo) * i as f64 / 20.0;
      assert!(at(f) <= 2.0 + 1e-9, "带内 {f} Hz 超标：{}", at(f));
    }
    // 边界外一点点必须超标，否则说明「连续区间」算宽了。
    assert!(at(lo * 0.97) > 2.0, "下边界外应超标");
    assert!(at(hi * 1.03) > 2.0, "上边界外应超标");
  }

  #[test]
  fn matched_bandwidth_needs_a_real_match() {
    let z0 = 50.0;
    // 未接任何元件：100 − j50Ω 的 SWR 约 2.6，谈不上「匹配带宽」。
    assert_eq!(
      matched_bandwidth((100.0, -50.0), 0.0, 0.0, false, 7.1e6, z0, 2.0),
      None
    );
  }

  #[test]
  fn higher_load_q_gives_narrower_relative_bandwidth() {
    // r = 1 时 L 网络退化为一颗串联电抗，带宽完全由负载 Q 决定：
    // 电抗越大（Q 越高）相对带宽越窄。
    let z0 = 50.0;
    let f0 = 14.1e6;
    let rel_bw = |x: f64| {
      let m = l_match(1.0, x).unwrap()[0];
      let (lo, hi) =
        matched_bandwidth((z0, x * z0), m.x * z0, m.b / z0, m.shunt_first, f0, z0, 2.0)
          .expect("应给出带宽");
      (hi - lo) / f0
    };
    let narrow = rel_bw(2.0);
    let wide = rel_bw(0.3);
    assert!(
      narrow < wide,
      "高 Q 负载的相对带宽应更窄：{narrow} vs {wide}"
    );
    assert!(narrow > 0.0 && wide < 4.0, "{narrow} {wide}");
  }
}
