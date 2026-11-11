//! ATU（天线调谐器）覆盖综合：一段负载阻抗范围 × 160–6 m 各波段，Pi / T 网络够不够用。
//!
//! `smith::l_match` 只回答「这一个负载、这一个频率」；真正选调谐器时要问的是另一个问题 ——
//! **我的天线在各波段上阻抗会落在这一片区域里，调谐器的电容 / 电感范围能不能全部兜住**。
//! 这里把这个问题做成纯计算：给定负载矩形区域（`R` 对数采样、`X` 线性采样）、元件范围与 `Z0`，
//! 逐波段（取下沿 / 中心 / 上沿三个频率）判定每个负载点能否被匹配到 `Z0`。
//!
//! # 网络与自由度
//!
//! 三元件网络匹配一个复负载只要两个条件，多出一个自由度 —— 取「虚拟电阻」`Rv`
//! （两段 L 型网络背靠背时中间节点看到的电阻）。`Rv` 越极端，两段的 Q 越高、元件越大：
//!
//! - [`Topology::Pi`]：低通，**源侧并联 C · 串联 L · 负载侧并联 C**。负载先换成并联形式
//!   （`R_p = (R²+X²)/R`，`B = −X/(R²+X²)`），负载自带的电纳并入负载侧电容。
//!   `Rv ≤ min(R_p, Z0)`；负载是容性（`X < 0`）时还要 `Rv ≤ R`，否则负载侧电容会变成负值（要一颗电感）。
//! - [`Topology::T`]：高通，**源侧串联 C · 并联 L · 负载侧串联 C**。`Rv ≥ max(R, Z0)`；
//!   容性负载（`X < 0`）时还要 `Rv ≥ R_p`，否则负载侧串联电容的容抗会变成负值。
//!
//! 约定（页面也照此标注）：`c_in_pf` 总在**源（电台）侧**，`c_out_pf` 总在**负载（天线）侧**；
//! Pi 里它们是并联电容、`l_uh` 是串联电感，T 里它们是串联电容、`l_uh` 是并联电感。
//!
//! # 「所需元件下限」的含义
//!
//! 每个负载点的解是一条以 `Rv` 为参数的曲线，所以「需要多大的电容」没有唯一答案。这里给的是
//! **单独成立的必要条件**：
//!
//! - `need_c_pf` = 各点上 `min_Rv max(C_in, C_out)` 的最大值 —— 电容上限低于它，必有负载点无论 `Rv` 怎么取都配不上；
//! - `need_l_uh` = 各点上 `min_Rv L` 的最大值，同理。
//!
//! 两个下限**各自必要、合起来不充分**（同一个 `Rv` 未必同时让 C 和 L 都取到最小）；
//! 充分性要看 [`analyze`] 的 `covered`，它用的是你给的全部四个元件限值。
//!
//! # 口径与不建模
//!
//! - 元件理想、无损、无杂散；真实调谐器的电感有自谐振、电容有最小值（用 `c_min_pf` / `l_min_uh` 近似）；
//! - 负载按「调谐器输入端看到的阻抗」给，与频率无关（同一个 `R + jX` 对每个波段都试一遍）；
//! - `Rv` 在可行区间的一端向外扫 3 个数量级（对应 Q 到约 30），400 点对数网格；
//! - **Pi-L**（Pi 后面再串一颗输出电感）没做：那颗电感是为抑制谐波加的设计选择，
//!   不由「匹配」这个条件决定，没有唯一解可综合。
//!
//! # 验证
//!
//! 主验证是**往返**：把解代回去，用独立的正向电路分析（逐个元件做导纳 / 阻抗加法，
//! [`input_impedance`]）算输入阻抗，必须回到 `Z0 + j0`。另有手算锚点、必要下限的两侧夹逼
//! （略低一点不覆盖、略高一点覆盖）与 Python 独立实现的数值对拍。

use crate::bands::AMATEUR_BAND_EDGES;

const TAU: f64 = std::f64::consts::TAU;

/// `Rv` 扫描的点数（对数网格，含两端）。
pub const RV_STEPS: usize = 400;
/// `Rv` 从可行区间的一端向外扫的倍数（1000× ≈ 单段 Q 到 30 左右）。
pub const RV_SPAN: f64 = 1000.0;
/// 负载电阻方向的采样点数（对数间隔）。
pub const GRID_R: usize = 7;
/// 负载电抗方向的采样点数（线性间隔）。
pub const GRID_X: usize = 9;
/// 判负值的相对容差：边界上的解（如 `Rv = R`）舍入后可能是 −1e-17，不能因此丢掉。
const REL_EPS: f64 = 1e-9;

/// 参与综合的波段（160–6 m，边界取自全站唯一的 [`AMATEUR_BAND_EDGES`]）。
///
/// 2200 m / 630 m 不收：那两段的天线与调谐器是另一类东西（大型加载线圈），
/// 三元件网络的元件范围对不上。
pub const ATU_BANDS: &[&str] = &[
  "160m", "80m", "60m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m",
];

/// 网络拓扑。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Topology {
  /// 低通 Pi：源侧并联 C · 串联 L · 负载侧并联 C。
  Pi,
  /// 高通 T：源侧串联 C · 并联 L · 负载侧串联 C。
  T,
}

/// 一组具体的元件值（对应某一个 `Rv`）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Network {
  /// 拓扑。
  pub topology: Topology,
  /// 虚拟电阻（Ω）。
  pub rv_ohm: f64,
  /// 源（电台）侧电容（pF）。Pi 里是并联，T 里是串联。Pi 中可为 0（该侧不需要电容）。
  pub c_in_pf: f64,
  /// 电感（µH）。Pi 里是串联，T 里是并联。
  pub l_uh: f64,
  /// 负载（天线）侧电容（pF）。Pi 里是并联，T 里是串联。Pi 中可为 0。
  pub c_out_pf: f64,
  /// 两段 L 型网络的 Q 之和（越小，环流越小、损耗越低、带宽越宽）。
  pub q_total: f64,
}

/// 调谐器的元件范围。`c_*` 同时约束两颗电容，`l_*` 约束那一颗电感。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
  /// 电容下限（pF）：可变电容的最小值 / 杂散电容。
  pub c_min_pf: f64,
  /// 电容上限（pF）。
  pub c_max_pf: f64,
  /// 电感下限（µH）：滚轮电感的最小值 / 引线电感。
  pub l_min_uh: f64,
  /// 电感上限（µH）。
  pub l_max_uh: f64,
}

impl Limits {
  /// 范围是否合法：全部有限、非负、下限不超过上限。
  #[must_use]
  pub fn is_valid(&self) -> bool {
    let all = [self.c_min_pf, self.c_max_pf, self.l_min_uh, self.l_max_uh];
    all.iter().all(|v| v.is_finite() && *v >= 0.0)
      && self.c_min_pf <= self.c_max_pf
      && self.l_min_uh <= self.l_max_uh
  }

  /// 这组元件值是否全部落在范围内。
  #[must_use]
  pub fn admits(&self, n: &Network) -> bool {
    let c_ok = |c: f64| c >= self.c_min_pf && c <= self.c_max_pf;
    c_ok(n.c_in_pf) && c_ok(n.c_out_pf) && n.l_uh >= self.l_min_uh && n.l_uh <= self.l_max_uh
  }
}

/// 负载阻抗区域（调谐器输入端看到的 `R + jX`，Ω）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoadRegion {
  /// 电阻下限（Ω，> 0）。
  pub r_min: f64,
  /// 电阻上限（Ω）。
  pub r_max: f64,
  /// 电抗下限（Ω，可为负）。
  pub x_min: f64,
  /// 电抗上限（Ω）。
  pub x_max: f64,
}

/// 单个（负载点，频率）的判定。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointResult {
  /// 扫描到的解里，是否有一组全部落在元件范围内。
  pub covered: bool,
  /// 该点上 `min_Rv max(C_in, C_out)`（pF）。
  pub need_c_pf: f64,
  /// 该点上 `min_Rv L`（µH）。
  pub need_l_uh: f64,
}

/// 一个波段的汇总。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandReport {
  /// 波段名（同 [`AMATEUR_BAND_EDGES`]）。
  pub band: &'static str,
  /// 负载采样点数。
  pub total: usize,
  /// 在**下沿 / 中心 / 上沿三个频率上都**被覆盖的负载点数。
  pub covered: usize,
  /// 本波段的电容必要下限（pF），取三个频率与全部负载点的最大值；无解时为 ∞。
  pub need_c_pf: f64,
  /// 本波段的电感必要下限（µH），口径同上。
  pub need_l_uh: f64,
}

fn recip((a, b): (f64, f64)) -> (f64, f64) {
  let d = a * a + b * b;
  (a / d, -b / d)
}

/// 按给定 `Rv` 求一组元件值。
///
/// 返回 `None`：输入非法、`Rv` 超出该拓扑的可行范围，或解需要「负电容」（即这个拓扑在该点
/// 做不到，如 Pi 配容性负载时 `Rv > R`）。T 的任一串联电容容抗为 0 也返回 `None`
/// （`Rv = Z0` 时源侧电容是一根导线，不是一颗可调电容）。
#[must_use]
pub fn design(
  topology: Topology,
  load: (f64, f64),
  z0: f64,
  f_hz: f64,
  rv: f64,
) -> Option<Network> {
  let (r, x) = load;
  if ![r, x, z0, f_hz, rv].iter().all(|v| v.is_finite())
    || r <= 0.0
    || z0 <= 0.0
    || f_hz <= 0.0
    || rv <= 0.0
  {
    return None;
  }
  let w = TAU * f_hz;
  match topology {
    Topology::Pi => {
      let d = r * r + x * x;
      let r_p = d / r;
      let b_load = -x / d;
      if rv > r_p.min(z0) * (1.0 + 1e-12) {
        return None;
      }
      let q1 = (r_p / rv - 1.0).max(0.0).sqrt();
      let q2 = (z0 / rv - 1.0).max(0.0).sqrt();
      let b_out = q1 / r_p - b_load;
      let scale = (q1 / r_p).abs() + b_load.abs();
      if b_out < -REL_EPS * scale.max(1e-30) {
        return None;
      }
      Some(Network {
        topology,
        rv_ohm: rv,
        c_in_pf: q2 / z0 / w * 1e12,
        l_uh: (q1 + q2) * rv / w * 1e6,
        c_out_pf: b_out.max(0.0) / w * 1e12,
        q_total: q1 + q2,
      })
    }
    Topology::T => {
      if rv < r.max(z0) * (1.0 - 1e-12) {
        return None;
      }
      let q1 = (rv / r - 1.0).max(0.0).sqrt();
      let q2 = (rv / z0 - 1.0).max(0.0).sqrt();
      // 负载侧串联总容抗是 q1·R，其中负载自带 X，要补的容抗 = q1·R + X。
      let xc_out = q1 * r + x;
      let xc_in = q2 * z0;
      if xc_out <= 0.0 || xc_in <= 0.0 {
        return None;
      }
      Some(Network {
        topology,
        rv_ohm: rv,
        c_in_pf: 1.0 / (w * xc_in) * 1e12,
        l_uh: rv / (q1 + q2) / w * 1e6,
        c_out_pf: 1.0 / (w * xc_out) * 1e12,
        q_total: q1 + q2,
      })
    }
  }
}

/// 正向电路分析：把 `n` 接在 `load` 上，从负载侧一个元件一个元件加上去，返回电台端看到的输入阻抗（Ω）。
///
/// 独立于 [`design`] 的闭式推导 —— 验证用它把解代回去，应回到 `Z0 + j0`。
#[must_use]
pub fn input_impedance(n: &Network, load: (f64, f64), f_hz: f64) -> (f64, f64) {
  let w = TAU * f_hz;
  let c_in = n.c_in_pf * 1e-12;
  let c_out = n.c_out_pf * 1e-12;
  let l = n.l_uh * 1e-6;
  match n.topology {
    Topology::Pi => {
      let y = recip(load);
      let z = recip((y.0, y.1 + w * c_out));
      let z = (z.0, z.1 + w * l);
      let y = recip(z);
      recip((y.0, y.1 + w * c_in))
    }
    Topology::T => {
      let z = (load.0, load.1 - 1.0 / (w * c_out));
      let y = recip(z);
      let z = recip((y.0, y.1 - 1.0 / (w * l)));
      (z.0, z.1 - 1.0 / (w * c_in))
    }
  }
}

/// 该点上 `Rv` 的扫描区间 `(下端, 上端)`，两端都是可行边界。
fn rv_range(topology: Topology, (r, x): (f64, f64), z0: f64) -> (f64, f64) {
  let r_p = (r * r + x * x) / r;
  match topology {
    Topology::Pi => {
      let hi = z0.min(if x < 0.0 { r } else { r_p });
      (hi / RV_SPAN, hi)
    }
    Topology::T => {
      let lo = z0.max(if x < 0.0 { r_p } else { r });
      (lo, lo * RV_SPAN)
    }
  }
}

/// 沿 `Rv` 对数网格扫一遍，产出所有可行的元件值（含两端）。
fn sweep(
  topology: Topology,
  load: (f64, f64),
  z0: f64,
  f_hz: f64,
) -> impl Iterator<Item = Network> {
  let (lo, hi) = rv_range(topology, load, z0);
  (0..RV_STEPS).filter_map(move |i| {
    let rv = match i {
      0 => lo,
      i if i == RV_STEPS - 1 => hi,
      i => lo * (hi / lo).powf(i as f64 / (RV_STEPS - 1) as f64),
    };
    design(topology, load, z0, f_hz, rv)
  })
}

/// 在元件范围内、总 Q 最小的那一组解。没有可行解时返回 `None`。
///
/// 取最小总 Q 是展示口径（环流小、损耗低、带宽宽），不影响「能不能覆盖」的判定。
#[must_use]
pub fn best_within(
  topology: Topology,
  load: (f64, f64),
  z0: f64,
  f_hz: f64,
  limits: &Limits,
) -> Option<Network> {
  sweep(topology, load, z0, f_hz)
    .filter(|n| limits.admits(n))
    .min_by(|a, b| a.q_total.total_cmp(&b.q_total))
}

/// 判定一个（负载点，频率）：能否覆盖，以及各自的必要下限。
///
/// 没有任何可行解（输入非法，或整条 `Rv` 区间都做不到）返回 `None`。
#[must_use]
pub fn evaluate_point(
  topology: Topology,
  load: (f64, f64),
  z0: f64,
  f_hz: f64,
  limits: &Limits,
) -> Option<PointResult> {
  judge(sweep(topology, load, z0, f_hz), limits)
}

/// 对一批候选解做判定（[`evaluate_point`] 与 [`analyze`] 共用，保证两条路径口径一致）。
fn judge(candidates: impl Iterator<Item = Network>, limits: &Limits) -> Option<PointResult> {
  let mut seen = false;
  let mut covered = false;
  let mut need_c = f64::INFINITY;
  let mut need_l = f64::INFINITY;
  for n in candidates {
    seen = true;
    covered |= limits.admits(&n);
    need_c = need_c.min(n.c_in_pf.max(n.c_out_pf));
    need_l = need_l.min(n.l_uh);
  }
  seen.then_some(PointResult {
    covered,
    need_c_pf: need_c,
    need_l_uh: need_l,
  })
}

/// 参考频率（Hz）：[`analyze`] 每个负载点只在这一个频率上求一遍解，其余频率按比例换算。
const F_REF_HZ: f64 = 1e6;

/// 把参考频率上的解换算到另一个频率（`k = F_REF_HZ / f`）。
///
/// 对**同一个 `Rv`**，`design` 里的电容与电感都只通过 `1/ω` 出现（Pi 的电纳 / ω，T 的 1/(ω·容抗)
/// 与感抗 / ω），所以元件值严格 ∝ 1/f；`Rv` 网格与可行边界又都与频率无关。
/// 这让逐波段综合从「点数 × 频率数 × 400 次求解」降到「点数 × 400 次」。
fn scaled(n: &Network, k: f64) -> Network {
  Network {
    c_in_pf: n.c_in_pf * k,
    c_out_pf: n.c_out_pf * k,
    l_uh: n.l_uh * k,
    ..*n
  }
}

/// 波段的下沿 / 中心 / 上沿频率（Hz）。波段名不在 [`AMATEUR_BAND_EDGES`] 里返回 `None`。
#[must_use]
pub fn band_freqs_hz(band: &str) -> Option<[f64; 3]> {
  AMATEUR_BAND_EDGES
    .iter()
    .find(|(name, _, _)| *name == band)
    .map(|&(_, lo, hi)| [lo * 1e6, (lo + hi) / 2.0 * 1e6, hi * 1e6])
}

/// 把负载区域铺成采样点：`R` 对数间隔 [`GRID_R`] 点、`X` 线性间隔 [`GRID_X`] 点；
/// 某一维上下限相等时该维只取一个点。区域非法返回 `None`。
#[must_use]
pub fn sample_loads(region: &LoadRegion) -> Option<Vec<(f64, f64)>> {
  let LoadRegion {
    r_min,
    r_max,
    x_min,
    x_max,
  } = *region;
  if ![r_min, r_max, x_min, x_max].iter().all(|v| v.is_finite())
    || r_min <= 0.0
    || r_max < r_min
    || x_max < x_min
  {
    return None;
  }
  let rs: Vec<f64> = if r_max == r_min {
    vec![r_min]
  } else {
    (0..GRID_R)
      .map(|i| r_min * (r_max / r_min).powf(i as f64 / (GRID_R - 1) as f64))
      .collect()
  };
  let xs: Vec<f64> = if x_max == x_min {
    vec![x_min]
  } else {
    (0..GRID_X)
      .map(|i| x_min + (x_max - x_min) * i as f64 / (GRID_X - 1) as f64)
      .collect()
  };
  Some(
    rs.iter()
      .flat_map(|&r| xs.iter().map(move |&x| (r, x)))
      .collect(),
  )
}

/// 逐波段综合：对 [`ATU_BANDS`] 里每个波段，把负载区域的每个点在三个频率上都试一遍。
///
/// 区域、元件范围或 `z0` 非法时返回 `None`。某个点没有任何可行解时，该点按「未覆盖」计、
/// 必要下限记 ∞（页面显示为 `—`）。
#[must_use]
pub fn analyze(
  topology: Topology,
  region: &LoadRegion,
  z0: f64,
  limits: &Limits,
) -> Option<Vec<BandReport>> {
  if !limits.is_valid() || !z0.is_finite() || z0 <= 0.0 {
    return None;
  }
  let loads = sample_loads(region)?;
  // 每个负载点只扫一次 `Rv`（见 [`scaled`]），33 个频率共用。
  let sweeps: Vec<Vec<Network>> = loads
    .iter()
    .map(|&load| sweep(topology, load, z0, F_REF_HZ).collect())
    .collect();
  ATU_BANDS
    .iter()
    .map(|&band| {
      let freqs = band_freqs_hz(band)?;
      let mut covered = 0;
      let mut need_c: f64 = 0.0;
      let mut need_l: f64 = 0.0;
      for base in &sweeps {
        let mut all_ok = true;
        for &f in &freqs {
          let k = F_REF_HZ / f;
          match judge(base.iter().map(|n| scaled(n, k)), limits) {
            Some(p) => {
              all_ok &= p.covered;
              need_c = need_c.max(p.need_c_pf);
              need_l = need_l.max(p.need_l_uh);
            }
            None => {
              all_ok = false;
              need_c = f64::INFINITY;
              need_l = f64::INFINITY;
            }
          }
        }
        covered += usize::from(all_ok);
      }
      Some(BandReport {
        band,
        total: loads.len(),
        covered,
        need_c_pf: need_c,
        need_l_uh: need_l,
      })
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  const Z0: f64 = 50.0;

  fn open_limits() -> Limits {
    Limits {
      c_min_pf: 0.0,
      c_max_pf: 1e9,
      l_min_uh: 0.0,
      l_max_uh: 1e9,
    }
  }

  /// 与页面默认值一致的元件范围（也是 Python 对拍用的那组）。
  fn bench_limits() -> Limits {
    Limits {
      c_min_pf: 10.0,
      c_max_pf: 500.0,
      l_min_uh: 0.3,
      l_max_uh: 30.0,
    }
  }

  fn bench_region() -> LoadRegion {
    LoadRegion {
      r_min: 10.0,
      r_max: 500.0,
      x_min: -200.0,
      x_max: 200.0,
    }
  }

  fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * b.abs().max(1e-30)
  }

  #[test]
  fn pi_matches_hand_calculation() {
    // 100 Ω 纯阻 → 50 Ω，Rv = 25：Q1 = √3、Q2 = 1。
    // B_out = √3/100、B_in = 1/50、X_L = (√3 + 1)·25，7.1 MHz 下手算：
    //   C_out = 388.26 pF，C_in = 448.32 pF，L = 1.5311 µH。
    let n = design(Topology::Pi, (100.0, 0.0), Z0, 7.1e6, 25.0).expect("可行");
    assert!(close(n.c_out_pf, 388.26, 1e-4), "C_out = {}", n.c_out_pf);
    assert!(close(n.c_in_pf, 448.32, 1e-4), "C_in = {}", n.c_in_pf);
    assert!(close(n.l_uh, 1.5311, 1e-4), "L = {}", n.l_uh);
    assert!(close(n.q_total, 1.0 + 3f64.sqrt(), 1e-12));
  }

  #[test]
  fn t_matches_hand_calculation() {
    // 10 Ω 纯阻 → 50 Ω，Rv = 100：Q1 = 3、Q2 = 1。
    // 负载侧容抗 3·10 = 30 Ω、源侧 1·50 = 50 Ω、并联感抗 100/4 = 25 Ω，7.1 MHz 下手算：
    //   C_out = 1/(ω·30) = 747.2 pF，C_in = 1/(ω·50) = 448.3 pF，L = 25/ω = 0.5604 µH。
    let n = design(Topology::T, (10.0, 0.0), Z0, 7.1e6, 100.0).expect("可行");
    assert!(close(n.c_out_pf, 747.2, 1e-3), "C_out = {}", n.c_out_pf);
    assert!(close(n.c_in_pf, 448.32, 1e-4), "C_in = {}", n.c_in_pf);
    assert!(close(n.l_uh, 0.5604, 1e-3), "L = {}", n.l_uh);
  }

  #[test]
  fn every_candidate_round_trips_to_z0() {
    // 往返：不用闭式，逐元件做正向电路分析，输入阻抗必须回到 Z0 + j0。
    // 覆盖两种拓扑 × 高 / 低阻 × 感 / 容性负载 × 多个频率。
    let loads = [
      (10.0, -200.0),
      (10.0, 200.0),
      (25.0, 0.0),
      (50.0, 80.0),
      (100.0, -50.0),
      (500.0, 200.0),
      (500.0, -200.0),
      (3.0, 15.0),
    ];
    let mut checked = 0;
    for topology in [Topology::Pi, Topology::T] {
      for load in loads {
        for f in [1.9e6, 7.1e6, 14.2e6, 52e6] {
          for n in sweep(topology, load, Z0, f) {
            let z = input_impedance(&n, load, f);
            assert!(
              (z.0 - Z0).abs() < 1e-6 && z.1.abs() < 1e-6,
              "{topology:?} 负载 {load:?} @ {f} Hz, Rv {}: 输入阻抗 {z:?}",
              n.rv_ohm
            );
            checked += 1;
          }
        }
      }
    }
    // 防止「扫描什么也没产出」时测试空转通过。
    assert!(checked > 5000, "只验了 {checked} 个解");
  }

  #[test]
  fn rv_outside_feasible_range_is_rejected() {
    // Pi：Rv 不能超过 min(R_p, Z0)。
    assert!(design(Topology::Pi, (100.0, 0.0), Z0, 7.1e6, 50.0).is_some());
    assert!(design(Topology::Pi, (100.0, 0.0), Z0, 7.1e6, 50.1).is_none());
    // T：Rv 不能低于 max(R, Z0)。
    assert!(
      design(Topology::T, (10.0, 0.0), Z0, 7.1e6, 50.0).is_none(),
      "Rv = Z0 时源侧电容是导线"
    );
    assert!(design(Topology::T, (10.0, 0.0), Z0, 7.1e6, 49.9).is_none());
    assert!(design(Topology::T, (10.0, 0.0), Z0, 7.1e6, 50.1).is_some());
  }

  #[test]
  fn pi_with_capacitive_load_needs_rv_not_above_r() {
    // 10 − j50 Ω：R_p = 260 Ω、Z0 = 50 Ω，数值上 Rv = 20 过得了 Q 的实数检查，
    // 但负载侧并联电容会变成负值（要一颗电感）—— Pi 在这里做不到，必须拒绝。
    let load = (10.0, -50.0);
    assert!(design(Topology::Pi, load, Z0, 7.1e6, 20.0).is_none());
    assert!(
      design(Topology::Pi, load, Z0, 7.1e6, 10.0).is_some(),
      "Rv = R 恰是边界，应可行"
    );
    // 感性负载没有这条限制。
    assert!(design(Topology::Pi, (10.0, 50.0), Z0, 7.1e6, 20.0).is_some());
  }

  #[test]
  fn t_with_capacitive_load_needs_rv_not_below_rp() {
    // 对偶：T 配容性负载，Rv 必须 ≥ R_p（负载自带的容抗已超过所需，不能再串电容去抵消）。
    let load = (10.0, -50.0); // R_p = 260
    assert!(design(Topology::T, load, Z0, 7.1e6, 200.0).is_none());
    assert!(design(Topology::T, load, Z0, 7.1e6, 300.0).is_some());
    // 感性负载只需 Rv ≥ max(R, Z0)。
    assert!(design(Topology::T, (10.0, 50.0), Z0, 7.1e6, 60.0).is_some());
  }

  #[test]
  fn invalid_inputs_are_rejected() {
    for bad in [
      design(Topology::Pi, (0.0, 0.0), Z0, 7.1e6, 10.0),
      design(Topology::Pi, (-5.0, 0.0), Z0, 7.1e6, 10.0),
      design(Topology::Pi, (50.0, 0.0), 0.0, 7.1e6, 10.0),
      design(Topology::Pi, (50.0, 0.0), Z0, 0.0, 10.0),
      design(Topology::Pi, (50.0, 0.0), Z0, 7.1e6, 0.0),
      design(Topology::T, (f64::NAN, 0.0), Z0, 7.1e6, 100.0),
      design(Topology::T, (50.0, f64::INFINITY), Z0, 7.1e6, 100.0),
    ] {
      assert_eq!(bad, None);
    }
  }

  #[test]
  fn component_values_scale_inversely_with_frequency() {
    // 同一个 Rv：元件电抗 / 电纳固定，频率翻倍 → C、L 都减半。
    let a = design(Topology::Pi, (100.0, 30.0), Z0, 7.0e6, 20.0).expect("可行");
    let b = design(Topology::Pi, (100.0, 30.0), Z0, 14.0e6, 20.0).expect("可行");
    assert!(close(b.c_in_pf, a.c_in_pf / 2.0, 1e-12));
    assert!(close(b.c_out_pf, a.c_out_pf / 2.0, 1e-12));
    assert!(close(b.l_uh, a.l_uh / 2.0, 1e-12));
    let a = design(Topology::T, (20.0, -10.0), Z0, 7.0e6, 150.0).expect("可行");
    let b = design(Topology::T, (20.0, -10.0), Z0, 14.0e6, 150.0).expect("可行");
    assert!(close(b.c_in_pf, a.c_in_pf / 2.0, 1e-12));
    assert!(close(b.c_out_pf, a.c_out_pf / 2.0, 1e-12));
    assert!(close(b.l_uh, a.l_uh / 2.0, 1e-12));
  }

  #[test]
  fn best_within_respects_limits_and_picks_lowest_q() {
    let load = (25.0, 40.0);
    let f = 7.1e6;
    let best = best_within(Topology::Pi, load, Z0, f, &open_limits()).expect("无限制必有解");
    // 全部候选里没有比它 Q 更低的。
    assert!(sweep(Topology::Pi, load, Z0, f).all(|n| n.q_total >= best.q_total - 1e-12));
    // 收紧电感上限到比最优解还小：换成别的解，或者没有解，但一定不超限。
    let tight = Limits {
      l_max_uh: best.l_uh * 0.9,
      ..open_limits()
    };
    if let Some(n) = best_within(Topology::Pi, load, Z0, f, &tight) {
      assert!(n.l_uh <= tight.l_max_uh && n.q_total >= best.q_total);
    }
    // 上限低到谁都过不了：None。
    let none = Limits {
      c_max_pf: 1e-3,
      ..open_limits()
    };
    assert_eq!(best_within(Topology::T, load, Z0, f, &none), None);
  }

  #[test]
  fn need_is_a_tight_necessary_bound_for_each_axis() {
    // 必要下限的两侧夹逼：电容上限略低于 need_c → 一定不覆盖；略高于 → 覆盖（电感不设限）。
    // 电感同理。这同时证明了「下限」既必要又可达，不是随便给的数。
    for topology in [Topology::Pi, Topology::T] {
      for load in [(10.0, -200.0), (200.0, 100.0), (50.0, 0.0), (7.0, 30.0)] {
        for f in [1.9e6, 14.2e6, 52e6] {
          let p = evaluate_point(topology, load, Z0, f, &open_limits()).expect("有解");
          assert!(p.covered, "无限制必覆盖");
          let c_lo = Limits {
            c_max_pf: p.need_c_pf * 0.999,
            ..open_limits()
          };
          let c_hi = Limits {
            c_max_pf: p.need_c_pf * 1.001,
            ..open_limits()
          };
          let l_lo = Limits {
            l_max_uh: p.need_l_uh * 0.999,
            ..open_limits()
          };
          let l_hi = Limits {
            l_max_uh: p.need_l_uh * 1.001,
            ..open_limits()
          };
          let cov = |lim: &Limits| evaluate_point(topology, load, Z0, f, lim).unwrap().covered;
          // 需求为 0（如 Pi 配 50 Ω 纯阻：Rv = Z0，两颗电容与电感都不需要）时没有「略低于」可言，
          // 只验上侧；需求为正的点两侧都要夹得住。
          if p.need_c_pf > 0.0 {
            assert!(
              !cov(&c_lo),
              "{topology:?} {load:?} {f}: C 上限低于下限仍覆盖"
            );
          }
          assert!(
            cov(&c_hi),
            "{topology:?} {load:?} {f}: C 上限略高于下限却不覆盖"
          );
          if p.need_l_uh > 0.0 {
            assert!(
              !cov(&l_lo),
              "{topology:?} {load:?} {f}: L 上限低于下限仍覆盖"
            );
          }
          assert!(
            cov(&l_hi),
            "{topology:?} {load:?} {f}: L 上限略高于下限却不覆盖"
          );
        }
      }
    }
  }

  #[test]
  fn coverage_is_monotone_in_limits() {
    // 放宽任何一个上限、收紧任何一个下限，覆盖数只增不减。
    let region = bench_region();
    let base = analyze(Topology::Pi, &region, Z0, &bench_limits()).expect("合法");
    let wider = Limits {
      c_max_pf: 2000.0,
      l_max_uh: 100.0,
      c_min_pf: 5.0,
      l_min_uh: 0.1,
    };
    let more = analyze(Topology::Pi, &region, Z0, &wider).expect("合法");
    for (a, b) in base.iter().zip(&more) {
      assert!(
        b.covered >= a.covered,
        "{}: {} → {}",
        a.band,
        a.covered,
        b.covered
      );
    }
    assert!(
      more.iter().map(|r| r.covered).sum::<usize>() > base.iter().map(|r| r.covered).sum::<usize>(),
      "放宽后应至少多覆盖一些点，否则这条单调性断言什么也没测"
    );
  }

  #[test]
  fn analyze_matches_independent_python_implementation() {
    // 对拍值来自独立的 Python 实现（单独写的闭式 + 同一份 Rv 网格；Python 侧另用正向电路分析
    // 对 4000 组随机解验证过往返，最大误差 7e-11 Ω）：R 10–500 Ω、X −200…+200 Ω，
    // C 10–500 pF、L 0.3–30 µH。
    let region = bench_region();
    let limits = bench_limits();
    let pi = analyze(Topology::Pi, &region, Z0, &limits).expect("合法");
    let t = analyze(Topology::T, &region, Z0, &limits).expect("合法");
    let row =
      |rows: &[BandReport], band: &str| *rows.iter().find(|r| r.band == band).expect("波段在表内");
    // (拓扑, 波段, 覆盖, 电容下限 pF, 电感下限 µH)
    let expect: &[(&[BandReport], &str, usize, f64, f64)] = &[
      (&pi, "160m", 12, 3536.77651315323, 1.3917270437393912),
      (&pi, "20m", 59, 454.7284088339867, 0.17893633419506455),
      (&pi, "6m", 46, 127.32395447351628, 0.05010217357461808),
      (&t, "160m", 63, 125.0564390688497, 14.05947104155554),
      (&t, "20m", 63, 16.078685023137815, 1.8076462767714265),
      (&t, "6m", 49, 4.502031806478589, 0.5061409574959994),
    ];
    for &(rows, band, covered, need_c, need_l) in expect {
      let r = row(rows, band);
      assert_eq!(r.total, 63, "{band}");
      assert_eq!(r.covered, covered, "{band} 覆盖数");
      assert!(
        close(r.need_c_pf, need_c, 1e-6),
        "{band} C 下限 {} vs {need_c}",
        r.need_c_pf
      );
      assert!(
        close(r.need_l_uh, need_l, 1e-6),
        "{band} L 下限 {} vs {need_l}",
        r.need_l_uh
      );
    }
  }

  #[test]
  fn lower_bands_need_bigger_components() {
    // 元件值 ∝ 1/f：波段越低，必要下限越大。160 m 与 6 m 差一个数量级以上。
    for topology in [Topology::Pi, Topology::T] {
      let rows = analyze(topology, &bench_region(), Z0, &open_limits()).expect("合法");
      for w in rows.windows(2) {
        assert!(
          w[0].need_c_pf > w[1].need_c_pf,
          "{topology:?} {} vs {}",
          w[0].band,
          w[1].band
        );
        assert!(
          w[0].need_l_uh > w[1].need_l_uh,
          "{topology:?} {} vs {}",
          w[0].band,
          w[1].band
        );
      }
      // 不设限时，任何负载都应被覆盖。
      assert!(rows.iter().all(|r| r.covered == r.total), "{topology:?}");
    }
  }

  #[test]
  fn minimum_limits_can_leave_a_band_uncovered() {
    // 电容下限 10 pF：6 m 上 T 网络需要的串联电容远小于 10 pF，可变电容转不到那么小 ——
    // 这是「最小值」约束独有的失败方式，上限怎么放宽都救不了。
    let loose_max = Limits {
      c_max_pf: 1e6,
      l_max_uh: 1e6,
      ..bench_limits()
    };
    let rows = analyze(Topology::T, &bench_region(), Z0, &loose_max).expect("合法");
    let six = rows.iter().find(|r| r.band == "6m").expect("6m");
    assert!(
      six.covered < six.total,
      "6 m 应有点被最小电容挡住：{}/{}",
      six.covered,
      six.total
    );
  }

  #[test]
  fn band_table_comes_from_the_single_source() {
    for band in ATU_BANDS {
      let f = band_freqs_hz(band).unwrap_or_else(|| panic!("{band} 不在 AMATEUR_BAND_EDGES"));
      assert!(f[0] < f[1] && f[1] < f[2], "{band}: {f:?}");
    }
    // 160 m：1.8–2.0 MHz。
    assert_eq!(band_freqs_hz("160m"), Some([1.8e6, 1.9e6, 2.0e6]));
    assert_eq!(band_freqs_hz("2200m").map(|f| f[0] < 2e5), Some(true));
    assert_eq!(band_freqs_hz("nope"), None);
    // 表内波段必须按频率由低到高，页面表格才是从 160 m 排到 6 m。
    let lows: Vec<f64> = ATU_BANDS
      .iter()
      .map(|b| band_freqs_hz(b).unwrap()[0])
      .collect();
    assert!(lows.windows(2).all(|w| w[0] < w[1]));
  }

  #[test]
  fn sample_loads_grid_shape() {
    let pts = sample_loads(&bench_region()).expect("合法");
    assert_eq!(pts.len(), GRID_R * GRID_X);
    // 两端取到、R 对数间隔（相邻比相等）、X 线性间隔。
    assert_eq!(pts.first(), Some(&(10.0, -200.0)));
    let last = pts.last().expect("非空");
    assert!(close(last.0, 500.0, 1e-12) && close(last.1, 200.0, 1e-12));
    let r1 = pts[GRID_X].0 / pts[0].0;
    let r2 = pts[2 * GRID_X].0 / pts[GRID_X].0;
    assert!(close(r1, r2, 1e-12));
    assert!(close(pts[1].1 - pts[0].1, 50.0, 1e-12));
    // 某一维退化成一个值时该维只取一点。
    let one = LoadRegion {
      r_min: 50.0,
      r_max: 50.0,
      x_min: 0.0,
      x_max: 0.0,
    };
    assert_eq!(sample_loads(&one), Some(vec![(50.0, 0.0)]));
    let line = LoadRegion {
      r_min: 50.0,
      r_max: 50.0,
      x_min: -80.0,
      x_max: 80.0,
    };
    assert_eq!(sample_loads(&line).map(|v| v.len()), Some(GRID_X));
  }

  #[test]
  fn analyze_agrees_with_point_by_point_evaluation() {
    // `analyze` 每个负载点只扫一遍 Rv、再按 1/f 换算；这里用「每个点、每个频率各自重新求解」
    // 的 `evaluate_point` 逐项复算，覆盖数必须相同、必要下限在浮点误差内相同。
    // 取两组限值：页面默认的，以及故意卡得很紧的（让「覆盖」与「不覆盖」都出现）。
    let tight = Limits {
      c_min_pf: 30.0,
      c_max_pf: 200.0,
      l_min_uh: 1.0,
      l_max_uh: 8.0,
    };
    for topology in [Topology::Pi, Topology::T] {
      for limits in [bench_limits(), tight] {
        let region = bench_region();
        let fast = analyze(topology, &region, Z0, &limits).expect("合法");
        let loads = sample_loads(&region).expect("合法");
        for row in &fast {
          let freqs = band_freqs_hz(row.band).expect("波段在表内");
          let mut covered = 0;
          let (mut need_c, mut need_l) = (0.0f64, 0.0f64);
          for &load in &loads {
            let pts: Vec<PointResult> = freqs
              .iter()
              .map(|&f| evaluate_point(topology, load, Z0, f, &limits).expect("有解"))
              .collect();
            covered += usize::from(pts.iter().all(|p| p.covered));
            for p in &pts {
              need_c = need_c.max(p.need_c_pf);
              need_l = need_l.max(p.need_l_uh);
            }
          }
          let tag = format!("{topology:?} {} {limits:?}", row.band);
          assert_eq!(row.covered, covered, "{tag} 覆盖数");
          assert!(
            close(row.need_c_pf, need_c, 1e-12),
            "{tag} C 下限 {} vs {need_c}",
            row.need_c_pf
          );
          assert!(
            close(row.need_l_uh, need_l, 1e-12),
            "{tag} L 下限 {} vs {need_l}",
            row.need_l_uh
          );
        }
        // 紧限值下覆盖数必须既不是全部也不是 0，否则这条对拍没有覆盖到判定分支。
        if limits == tight {
          let sum: usize = fast.iter().map(|r| r.covered).sum();
          let all: usize = fast.iter().map(|r| r.total).sum();
          assert!(
            sum > 0 && sum < all,
            "{topology:?} 紧限值下覆盖 {sum}/{all}"
          );
        }
      }
    }
  }

  #[test]
  fn invalid_regions_and_limits_are_rejected() {
    let ok = bench_region();
    for bad in [
      LoadRegion { r_min: 0.0, ..ok },
      LoadRegion { r_min: -1.0, ..ok },
      LoadRegion { r_max: 5.0, ..ok },
      LoadRegion {
        x_max: -300.0,
        ..ok
      },
      LoadRegion {
        x_min: f64::NAN,
        ..ok
      },
    ] {
      assert_eq!(sample_loads(&bad), None, "{bad:?}");
      assert!(analyze(Topology::Pi, &bad, Z0, &bench_limits()).is_none());
    }
    for bad in [
      Limits {
        c_min_pf: 600.0,
        ..bench_limits()
      },
      Limits {
        l_min_uh: 40.0,
        ..bench_limits()
      },
      Limits {
        c_min_pf: -1.0,
        ..bench_limits()
      },
      Limits {
        l_max_uh: f64::INFINITY,
        ..bench_limits()
      },
    ] {
      assert!(!bad.is_valid(), "{bad:?}");
      assert!(analyze(Topology::T, &ok, Z0, &bad).is_none());
    }
    assert!(analyze(Topology::T, &ok, 0.0, &bench_limits()).is_none());
    assert!(bench_limits().is_valid());
  }
}
