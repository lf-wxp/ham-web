//! 整段录音上的时钟漂移与频率漂移跟踪（离线版 AFC）。
//!
//! [`crate::front`] 的对齐与中心频率都是「整段录音搜一次」：符号边界只有一个起点，音调梳只有一个
//! 基频。录音机 / 声卡时钟与发端差上千 ppm、或发射机在录音期间频率漂了半个音调间隔时，两端就会错位。
//!
//! 这里把录音切成 [`BLOCK`] 个符号一块，每块各自测一次「符号边界偏多少」和「音调梳偏多少」，
//! 再对块序号拟合一条**低阶多项式曲线**（1–3 阶，按 BIC 挑，见 [`fit_curve`]）：
//! - 符号边界 = `offset + n × sym + 曲线(n)`（曲线的平均斜率除以 `sym` 就是时钟误差）；
//! - 最低音调频率 = `base + 曲线(n)`（曲线的平均斜率就是频漂）。
//!
//! 1 阶抓线性漂移（固定的时钟比例误差、预热阶段的匀速频漂），2–3 阶抓匀加速与更弯的曲线（转速逐渐
//! 变化的录音机、指数式收敛的发射机预热）。曲线之外的东西（突然跳频、断续的时钟跳变）跟不了：
//! 残差大到不可信时整条拟合作废、退回「不跟踪」。fldigi 是逐样本跟踪（滑动 FFT + 每符号的相位 AFC），
//! 能跟任意形状的漂移，这里不能。
//!
//! **不改变干净录音的结果**：拟合出的修正小到不值得（符号边界偏移不到 1/32 个符号、频率偏移不到
//! 1/16 个波特率）就放弃（[`track`] 返回 `None`，[`bins_tracked`] 给出 [`Drift::fixed`] 与
//! [`crate::front::symbol_bins`] 的原结果），与没有这一步时逐位相同。

use crate::front::{Grid, peak_ratios, symbol_bins};
use crate::tone_magnitudes;
use ham_web_dsp::tone_power;

/// 一块的符号数。块越长单次估计越稳，但对「块内漂移」越不敏感；漂移速率在 1 块内
/// 应远小于 1 个符号（1000 ppm 时 48 个符号只漂 0.05 个符号）。
const BLOCK: usize = 48;

/// 每块只取其中均匀分布的这么多个窗口来评分（代价与它成正比）。
const BLOCK_WINDOWS: usize = 16;

/// 至少这么多块「有信号」才拟合（约 190 个符号；更短的录音漂移本来也不大）。
const MIN_BLOCKS: usize = 4;

/// 一块的平均峰均比达到它才算「有信号」（与 [`crate::front::gate`] 的区间门限同一个量级）。
const LIVE_RATIO: f64 = 2.4;

/// 一遍扫描的计算预算（Goertzel 乘加次数的量级）：超出就把块抽稀。
const PASS_BUDGET: usize = 200_000_000;

/// 符号边界修正小于这个比例（相对一个符号）就不用。
const TIMING_APPLY: f64 = 1.0 / 32.0;

/// 频率修正小于这个比例（相对波特率）就不用。
const FREQ_APPLY: f64 = 1.0 / 16.0;

/// 外推下一块的搜索中心时参考最近多少个点。
const PREDICT_POINTS: usize = 8;

/// 曲线把开头推到录音之前时最多丢掉的符号数，超过就当曲线畸形（见 `Drift::drop_unreachable`）。
const MAX_DROPPED_SYMBOLS: f64 = 4096.0;

/// 曲线的最高阶数。再高的阶数在 20–40 块的数据上只会去拟合测量噪声。
const MAX_DEGREE: usize = 3;

/// 一条随符号序号变化的修正曲线（`n` 是名义网格上的符号序号）。
///
/// 在拟合范围 `[x_min, x_max]` 内是多项式；范围之外按范围两端之间的平均斜率线性外推 ——
/// 高阶多项式一出范围就发散，而范围外只有开头 / 结尾的几块，用不着更精细的外推。
#[derive(Debug, Clone)]
struct Curve {
  /// 变量 `u = (n − centre) / half` 的多项式系数，常数项在前；空 = 恒为 0。
  coef: Vec<f64>,
  centre: f64,
  half: f64,
  x_min: f64,
  x_max: f64,
}

impl Curve {
  fn zero() -> Self {
    Self {
      coef: Vec::new(),
      centre: 0.0,
      half: 1.0,
      x_min: 0.0,
      x_max: 0.0,
    }
  }

  fn poly(&self, n: f64) -> f64 {
    let u = (n - self.centre) / self.half;
    self.coef.iter().rev().fold(0.0, |acc, &c| acc * u + c)
  }

  fn at(&self, n: f64) -> f64 {
    if self.coef.is_empty() {
      return 0.0;
    }
    if n < self.x_min {
      self.poly(self.x_min) + self.mean_slope() * (n - self.x_min)
    } else if n > self.x_max {
      self.poly(self.x_max) + self.mean_slope() * (n - self.x_max)
    } else {
      self.poly(n)
    }
  }

  /// 范围两端之间的平均斜率（每个符号）。
  fn mean_slope(&self) -> f64 {
    if self.coef.is_empty() || self.x_max <= self.x_min {
      return 0.0;
    }
    (self.poly(self.x_max) - self.poly(self.x_min)) / (self.x_max - self.x_min)
  }

  /// 曲线在拟合范围内偏离 0 最远的量（绝对值）。
  fn worst(&self) -> f64 {
    if self.coef.is_empty() {
      return 0.0;
    }
    (0..=32)
      .map(|i| {
        self
          .poly(self.x_min + (self.x_max - self.x_min) * f64::from(i) / 32.0)
          .abs()
      })
      .fold(0.0, f64::max)
  }
}

/// 符号边界与音调梳随符号序号的漂移模型。
#[derive(Debug, Clone)]
pub struct Drift {
  /// 名义网格第 0 个符号窗口的起点（样本）。
  offset: f64,
  /// 名义符号长度（样本，小数）。
  sym: f64,
  /// 名义音调梳的最低音调频率（Hz）。
  base: f64,
  /// 每个符号的窗口起点偏离名义网格的量（样本）。
  timing: Curve,
  /// 每个符号的最低音调频率偏离 `base` 的量（Hz）。
  freq: Curve,
  /// 输出的第 0 个符号对应名义网格的第几个（开头够不着的符号被丢掉）。
  first: f64,
  /// 采样率（Hz），只用来把「每符号」的斜率换算成「每秒」。
  rate: f64,
}

impl Drift {
  /// 不跟踪：符号边界等间隔、音调梳固定。
  #[must_use]
  pub fn fixed(grid: &Grid, base: f64, offset: f64) -> Self {
    Self {
      offset,
      sym: grid.sym,
      base,
      timing: Curve::zero(),
      freq: Curve::zero(),
      first: 0.0,
      rate: f64::from(grid.rate),
    }
  }

  /// 曲线可能把开头的窗口推到录音起点之前（`start < 0`），取不到窗口会让 [`symbol_bins_tracked`] 一个符号
  /// 也出不来：丢掉够不着的那几个符号，其余整体前移（`first`），曲线本身不变。
  ///
  /// 要丢超过 [`MAX_DROPPED_SYMBOLS`] 个说明曲线是畸形的（真实的漂移只会推出一两个符号），返回
  /// `false`，调用方应整体放弃跟踪，而不是带着被截掉一大段的结果继续解码。
  #[must_use]
  fn drop_unreachable(&mut self) -> bool {
    while self.start_at(0.0) < 0.0 {
      if self.first >= MAX_DROPPED_SYMBOLS {
        return false;
      }
      self.first += 1.0;
    }
    true
  }

  /// 输出的第 `i` 个符号的窗口起点（样本）。
  #[must_use]
  pub fn start_at(&self, i: f64) -> f64 {
    let n = i + self.first;
    let s = self.offset + n * self.sym + self.timing.at(n);
    // 差不到半个样本的就地归零（窗口起点本来就四舍五入到样本）。
    if s < 0.0 && s > -0.5 { 0.0 } else { s }
  }

  /// 输出的第 `i` 个符号时最低音调的频率（Hz）。
  #[must_use]
  pub fn base_at(&self, i: f64) -> f64 {
    let n = i + self.first;
    self.base + self.freq.at(n)
  }

  /// 符号周期相对名义值的平均偏差（ppm）：正 = 录音里的符号比名义的长（录音时钟偏慢）。
  #[must_use]
  pub fn clock_ppm(&self) -> f64 {
    self.timing.mean_slope() / self.sym * 1e6
  }

  /// 平均频漂（Hz / 秒，以录音自己的时间轴计）：正 = 越往后频率越高。
  #[must_use]
  pub fn drift_hz_per_s(&self) -> f64 {
    self.freq.mean_slope() * self.rate / self.sym
  }
}

/// 每个符号各音调的幅度，按符号顺序平铺（`ntones` 个一组），按 `drift` 逐符号落点。
///
/// `drift = Drift::fixed(..)` 时与 [`crate::front::symbol_bins`] 逐位相同。
#[must_use]
pub fn symbol_bins_tracked(samples: &[f32], grid: &Grid, drift: &Drift) -> Vec<f64> {
  let mut bins = Vec::new();
  let mut buf = vec![0.0f64; grid.ntones];
  let mut i = 0usize;
  while tone_magnitudes(
    samples,
    grid.rate,
    drift.start_at(i as f64),
    grid.win,
    drift.base_at(i as f64),
    grid.spacing,
    &mut buf,
  ) {
    bins.extend_from_slice(&buf);
    i += 1;
  }
  bins
}

/// 解 `a · x = b`（高斯消元 + 列主元）；奇异返回 `None`。
fn solve(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
  let n = b.len();
  for col in 0..n {
    let pivot = (col..n).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
    if a[pivot][col].abs() < 1e-12 {
      return None;
    }
    a.swap(col, pivot);
    b.swap(col, pivot);
    let pivot_row = a[col].clone();
    for row in col + 1..n {
      let f = a[row][col] / pivot_row[col];
      for (x, p) in a[row][col..].iter_mut().zip(&pivot_row[col..]) {
        *x -= f * p;
      }
      b[row] -= f * b[col];
    }
  }
  let mut x = vec![0.0; n];
  for row in (0..n).rev() {
    let tail: f64 = (row + 1..n).map(|k| a[row][k] * x[k]).sum();
    x[row] = (b[row] - tail) / a[row][row];
  }
  Some(x)
}

/// 在 `[-1, 1]` 的归一化变量上做 `degree` 阶多项式最小二乘；返回常数项在前的系数。
fn poly_fit(points: &[(f64, f64)], centre: f64, half: f64, degree: usize) -> Option<Vec<f64>> {
  let k = degree + 1;
  if points.len() < k {
    return None;
  }
  let mut ata = vec![vec![0.0; k]; k];
  let mut aty = vec![0.0; k];
  for &(x, y) in points {
    let u = (x - centre) / half;
    let pow: Vec<f64> = (0..k)
      .scan(1.0, |p, _| {
        let v = *p;
        *p *= u;
        Some(v)
      })
      .collect();
    for i in 0..k {
      for j in 0..k {
        ata[i][j] += pow[i] * pow[j];
      }
      aty[i] += pow[i] * y;
    }
  }
  solve(ata, aty)
}

fn eval(coef: &[f64], u: f64) -> f64 {
  coef.iter().rev().fold(0.0, |acc, &c| acc * u + c)
}

/// 稳健曲线拟合。
///
/// 1. 先用 2 阶曲线做两轮「拟合 → 丢掉残差超过 `tol` 的离群点」；
/// 2. 在留下的点上试 1–[`MAX_DEGREE`] 阶，按 BIC（`k·ln(mse) + 参数个数·ln k`）挑；`mse` 先垫上
///    `(tol / 16)²` 的底：测量噪声以下的改进不值得多一个参数。
///
/// 留下的点不足 [`MIN_BLOCKS`] 或不足总数的 60%、或挑中的曲线残差均方根超过 `tol / 2`，说明这些点
/// 不像一条平滑曲线（低信噪比下的乱数、或突变），返回 `None`，调用方就退回「不跟踪」。
fn fit_curve(points: &[(f64, f64)], tol: f64) -> Option<Curve> {
  if points.len() < MIN_BLOCKS {
    return None;
  }
  let x_min = points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
  let x_max = points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
  let (centre, half) = ((x_min + x_max) / 2.0, ((x_max - x_min) / 2.0).max(1.0));
  let residual = |coef: &[f64], (x, y): (f64, f64)| y - eval(coef, (x - centre) / half);

  let mut kept: Vec<(f64, f64)> = points.to_vec();
  for _ in 0..2 {
    let coef = poly_fit(&kept, centre, half, 2.min(kept.len().saturating_sub(1)))?;
    kept.retain(|&p| residual(&coef, p).abs() <= tol);
    if kept.len() < MIN_BLOCKS {
      return None;
    }
  }
  if (kept.len() as f64) < 0.6 * points.len() as f64 {
    return None;
  }

  let k = kept.len() as f64;
  let floor = (tol / 16.0).powi(2);
  let mut best: Option<(f64, Vec<f64>, f64)> = None;
  for degree in 1..=MAX_DEGREE.min(kept.len() - 2) {
    let Some(coef) = poly_fit(&kept, centre, half, degree) else {
      continue;
    };
    let mse = kept
      .iter()
      .map(|&p| residual(&coef, p).powi(2))
      .sum::<f64>()
      / k;
    let bic = k * mse.max(floor).ln() + (degree + 1) as f64 * k.ln();
    if best.as_ref().is_none_or(|b| bic < b.0) {
      best = Some((bic, coef, mse));
    }
  }
  let (bic, coef, mse) = best?;
  // 与「恒为 0」比：低信噪比下测量值围着 0 乱抖，拟合出的斜率会凭空带来几十 ppm，必须有压倒性的证据
  // （BIC 差 10 以上，相当于似然比 150 倍）才认它是真的漂移。
  let mean_sq = kept.iter().map(|p| p.1 * p.1).sum::<f64>() / k;
  let bic_zero = k * mean_sq.max(floor).ln();
  if bic > bic_zero - 10.0 {
    return None;
  }
  (mse.sqrt() <= tol / 2.0).then_some(Curve {
    coef,
    centre,
    half,
    x_min,
    x_max,
  })
}

/// 用最近几个点外推到 `x`（定位下一块的搜索中心）；没有点就是 `fallback`。
///
/// 取最近 [`PREDICT_POINTS`] 个点做 Theil–Sen 直线（斜率取两两斜率的中位数）：个别块被噪声带偏时
/// 不会把后面所有块的搜索中心一起带偏。
fn predict(points: &[(f64, f64)], x: f64, fallback: f64) -> f64 {
  let tail = &points[points.len().saturating_sub(PREDICT_POINTS)..];
  let Some(last) = tail.last() else {
    return fallback;
  };
  if tail.len() < 3 {
    return last.1;
  }
  let median = |mut v: Vec<f64>| {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
  };
  let slopes: Vec<f64> = (0..tail.len())
    .flat_map(|i| (i + 1..tail.len()).map(move |j| (i, j)))
    .filter(|&(i, j)| tail[j].0 > tail[i].0)
    .map(|(i, j)| (tail[j].1 - tail[i].1) / (tail[j].0 - tail[i].0))
    .collect();
  if slopes.is_empty() {
    return last.1;
  }
  let slope = median(slopes);
  let intercept = median(tail.iter().map(|p| p.1 - slope * p.0).collect());
  intercept + slope * x
}

fn argmax(values: &[f64]) -> usize {
  values
    .iter()
    .enumerate()
    .fold((0, f64::NEG_INFINITY), |best, (i, &v)| {
      if v > best.1 { (i, v) } else { best }
    })
    .0
}

/// 三角形峰的亚步长插值（`l` / `c` / `r` 是峰值点与两侧邻点的**幅度**），返回相对峰值点的偏移（步长的倍数）。
///
/// 符号边界错位 `x`（相对一个符号）时，窗口里目标音调只剩 `1 − x` 的幅度，所以幅度随错位呈两边直线下降的
/// 三角形，而不是抛物线。
fn triangle_peak(l: f64, c: f64, r: f64) -> f64 {
  let (side, sign) = if r > l { (l, 1.0) } else { (r, -1.0) };
  let denom = 2.0 * (c - side);
  if denom <= 0.0 {
    return 0.0;
  }
  (sign * (r - l).abs() / denom).clamp(-0.5, 0.5)
}

/// 抛物线峰的亚步长插值（`l` / `c` / `r` 是峰值点与两侧邻点的功率）。频率失配时功率按 sinc² 下降，
/// 峰顶附近是抛物线。
fn parabola_peak(l: f64, c: f64, r: f64) -> f64 {
  let denom = 2.0 * (l - 2.0 * c + r);
  if denom >= 0.0 {
    return 0.0;
  }
  ((l - r) / denom).clamp(-0.5, 0.5)
}

/// 挑出「有信号」的块（块序号），超出预算就均匀抽稀。
fn live_blocks(bins: &[f64], grid: &Grid) -> Vec<usize> {
  let ratios = peak_ratios(bins, grid.ntones);
  let live: Vec<usize> = ratios
    .as_chunks::<BLOCK>()
    .0
    .iter()
    .enumerate()
    .filter(|(_, c)| c.iter().sum::<f64>() / BLOCK as f64 >= LIVE_RATIO)
    .map(|(b, _)| b)
    .collect();
  // 每块最坏扫 9 个候选，每个候选 `BLOCK_WINDOWS` 个窗口 × 全部音调。
  let per_block = 9 * BLOCK_WINDOWS * grid.ntones * grid.win;
  let cap = (PASS_BUDGET / per_block).max(MIN_BLOCKS + 2);
  if live.len() <= cap {
    return live;
  }
  (0..cap).map(|k| live[k * live.len() / cap]).collect()
}

/// 块 `b` 里均匀取出的窗口的符号序号。
fn block_symbols(b: usize) -> impl Iterator<Item = f64> {
  (0..BLOCK_WINDOWS).map(move |i| (b * BLOCK + i * BLOCK / BLOCK_WINDOWS) as f64)
}

/// 块中心的符号序号（拟合的横坐标）。
fn block_centre(b: usize) -> f64 {
  (b * BLOCK) as f64 + (BLOCK - 1) as f64 / 2.0
}

/// 每块的符号边界偏移（相对名义网格 `offset + n × sym`），样本；返回 `(块中心, 偏移)`。
fn timing_points(
  samples: &[f32],
  grid: &Grid,
  base: f64,
  offset: f64,
  blocks: &[usize],
) -> Vec<(f64, f64)> {
  let tone_max = |w: &[f32]| {
    (0..grid.ntones)
      .map(|k| tone_power(w, grid.rate, base + k as f64 * grid.spacing))
      .fold(0.0, f64::max)
  };
  // 偏移 `delta` 时块 `b` 的幅度和（开方：错位时幅度线性下降，见 `triangle_peak`）。
  let amplitude = |b: usize, delta: f64| -> f64 {
    block_symbols(b)
      .filter_map(|n| {
        let start = offset + n * grid.sym + delta;
        if start < 0.0 {
          return None;
        }
        let s = start.round() as usize;
        samples.get(s..s + grid.win).map(tone_max)
      })
      .sum::<f64>()
      .sqrt()
  };
  let h = grid.sym / 16.0;
  let mut points: Vec<(f64, f64)> = Vec::new();
  for &b in blocks {
    let x = block_centre(b);
    // 整个符号周期（环形）16 档全搜：搜索范围不依赖预测，个别块出错也不会让后面的块够不着。
    let s: Vec<f64> = (0..16)
      .map(|i| amplitude(b, -grid.sym / 2.0 + f64::from(i) * h))
      .collect();
    let i = argmax(&s);
    let (lo, hi) = (s.iter().copied().fold(f64::INFINITY, f64::min), s[i]);
    // 一整块都是同一个音调（MFSK 的前导 / 收尾冲刷）时，错位不改变任何东西，没有定时信息。
    if hi <= 0.0 || (hi - lo) / hi < 0.12 {
      continue;
    }
    let circular =
      -grid.sym / 2.0 + (i as f64 + triangle_peak(s[(i + 15) % 16], s[i], s[(i + 1) % 16])) * h;
    // 环形搜索只知道偏移对一个符号周期取模的值；落在预测值最近的那一支上。
    let delta = match points.is_empty() {
      true => circular,
      false => {
        let predicted = predict(&points, x, 0.0);
        circular + ((predicted - circular) / grid.sym).round() * grid.sym
      }
    };
    points.push((x, delta));
  }
  points
}

/// 每块的音调梳频率偏差（Hz，相对 `base`），在 `drift` 给出的符号边界上测；返回 `(块中心, 偏差)`。
///
/// `drift.first` 必须还是 0（块序号 = 名义网格序号）。
fn freq_points(samples: &[f32], grid: &Grid, drift: &Drift, blocks: &[usize]) -> Vec<(f64, f64)> {
  let power = |b: usize, e: f64| -> f64 {
    block_symbols(b)
      .filter_map(|n| {
        let start = drift.start_at(n);
        if start < 0.0 {
          return None;
        }
        let s = start.round() as usize;
        let w = samples.get(s..s + grid.win)?;
        Some(
          (0..grid.ntones)
            .map(|k| tone_power(w, grid.rate, drift.base + e + k as f64 * grid.spacing))
            .fold(0.0, f64::max),
        )
      })
      .sum()
  };
  let h = grid.baud / 8.0;
  let mut points: Vec<(f64, f64)> = Vec::new();
  for &b in blocks {
    let x = block_centre(b);
    let centre = predict(&points, x, 0.0);
    let s: Vec<f64> = (0..9)
      .map(|i| power(b, centre + (f64::from(i) - 4.0) * h))
      .collect();
    let i = argmax(&s);
    if i == 0 || i == 8 || s[i] <= 0.0 {
      continue;
    }
    points.push((
      x,
      centre + ((i as f64 - 4.0) + parabola_peak(s[i - 1], s[i], s[i + 1])) * h,
    ));
  }
  points
}

/// 估计时钟漂移与频率漂移。`bins` 是按 `Drift::fixed(grid, base, offset)` 取出的各符号音调幅度
/// （调用方本来就要算，复用它来判断哪些块有信号）。
///
/// 没有可信、且值得做的修正时返回 `None`（调用方沿用不跟踪的网格）；多数调用方直接用 [`bins_tracked`]。
#[must_use]
pub fn track(samples: &[f32], grid: &Grid, base: f64, offset: f64, bins: &[f64]) -> Option<Drift> {
  let blocks = live_blocks(bins, grid);
  if blocks.len() < MIN_BLOCKS {
    return None;
  }
  let mut drift = Drift::fixed(grid, base, offset);
  let tp = timing_points(samples, grid, base, offset, &blocks);
  let timing = fit_curve(&tp, grid.sym / 8.0);
  if let Some(curve) = timing
    && curve.worst() >= grid.sym * TIMING_APPLY
  {
    drift.timing = curve;
  }
  let freq = fit_curve(
    &freq_points(samples, grid, &drift, &blocks),
    grid.baud / 4.0,
  );
  if let Some(curve) = freq
    && curve.worst() >= grid.baud * FREQ_APPLY
  {
    let mut candidate = drift.clone();
    candidate.freq = curve;
    let last = (bins.len() / grid.ntones) as f64;
    if grid.fits(candidate.base_at(0.0)) && grid.fits(candidate.base_at(last)) {
      drift = candidate;
    }
  }
  let applied = !drift.timing.coef.is_empty() || !drift.freq.coef.is_empty();
  (applied && drift.drop_unreachable()).then_some(drift)
}

/// 各符号的音调幅度（`ntones` 个一组、按符号顺序平铺）与对应的 [`Drift`]：MFSK 与 THOR 收端共用的入口。
///
/// 没有值得做的修正时返回 [`symbol_bins`] 的原结果与 [`Drift::fixed`]，与没有跟踪这一步时逐位相同
/// （也不重算一遍）。
#[must_use]
pub fn bins_tracked(samples: &[f32], grid: &Grid, base: f64, offset: f64) -> (Vec<f64>, Drift) {
  let fixed_bins = symbol_bins(samples, grid, base, offset);
  match track(samples, grid, base, offset, &fixed_bins) {
    Some(drift) => (symbol_bins_tracked(samples, grid, &drift), drift),
    None => (fixed_bins, Drift::fixed(grid, base, offset)),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 固定种子的均匀噪声 ∈ `[-amp, amp]`。
  struct Noise(u32);
  impl Noise {
    fn next(&mut self, amp: f64) -> f64 {
      self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
      ((self.0 >> 8) as f64 / 8_388_608.0 - 1.0) * amp
    }
  }

  /// 块中心序号 `24, 72, …`，共 `n` 个。
  fn xs(n: usize) -> Vec<f64> {
    (0..n).map(block_centre).collect()
  }

  fn grid() -> Grid {
    Grid::new(8000, 15.625, 15.625, 16).unwrap()
  }

  #[test]
  fn a_line_is_recovered_with_its_slope() {
    let mut noise = Noise(1);
    let pts: Vec<(f64, f64)> = xs(25)
      .into_iter()
      .map(|x| (x, 0.5 * x + 3.0 + noise.next(2.0)))
      .collect();
    let c = fit_curve(&pts, 64.0).expect("直线应被拟合");
    assert!((c.mean_slope() - 0.5).abs() < 0.01, "{}", c.mean_slope());
    assert!((c.at(600.0) - 303.0).abs() < 2.0, "{}", c.at(600.0));
  }

  #[test]
  fn a_bend_is_followed_by_a_higher_order() {
    // 转速逐渐变化的录音机：偏移是序号的二次函数。直线在两端会差几十个样本。
    let f = |x: f64| 2.0e-4 * x * x - 0.1 * x;
    let mut noise = Noise(2);
    let pts: Vec<(f64, f64)> = xs(25)
      .into_iter()
      .map(|x| (x, f(x) + noise.next(1.0)))
      .collect();
    let c = fit_curve(&pts, 64.0).expect("二次曲线应被拟合");
    for x in [100.0, 500.0, 900.0] {
      assert!(
        (c.at(x) - f(x)).abs() < 3.0,
        "x = {x}: {} vs {}",
        c.at(x),
        f(x)
      );
    }
  }

  #[test]
  fn extrapolation_outside_the_range_is_linear_not_polynomial() {
    let f = |x: f64| 2.0e-4 * x * x;
    let pts: Vec<(f64, f64)> = xs(25).into_iter().map(|x| (x, f(x))).collect();
    let c = fit_curve(&pts, 64.0).unwrap();
    // 范围外只按平均斜率线性外推：不会像二次式那样一路冲上去。
    let far = c.at(5000.0);
    assert!(far < f(5000.0) / 2.0, "{far}");
    let edge = c.at(c.x_max);
    assert!(
      (c.at(c.x_max + 100.0) - edge - 100.0 * c.mean_slope()).abs() < 1e-9,
      "范围外应沿平均斜率走"
    );
  }

  #[test]
  fn pure_noise_around_zero_is_rejected() {
    // 低信噪比下测量值围着 0 乱抖：拟合出的任何斜率都不能被当成真的漂移。
    for seed in 1..=20 {
      let mut noise = Noise(seed);
      let pts: Vec<(f64, f64)> = xs(25).into_iter().map(|x| (x, noise.next(10.0))).collect();
      assert!(fit_curve(&pts, 64.0).is_none(), "seed {seed}");
    }
  }

  #[test]
  fn a_constant_offset_is_real_when_it_stands_out_of_the_noise() {
    let mut noise = Noise(3);
    let pts: Vec<(f64, f64)> = xs(25)
      .into_iter()
      .map(|x| (x, 20.0 + noise.next(1.0)))
      .collect();
    let c = fit_curve(&pts, 64.0).expect("稳定的常数偏移是真的");
    assert!((c.at(500.0) - 20.0).abs() < 1.0);
    assert!(c.worst() > 19.0);
  }

  #[test]
  fn outliers_are_dropped_before_fitting() {
    let mut noise = Noise(4);
    let mut pts: Vec<(f64, f64)> = xs(25)
      .into_iter()
      .map(|x| (x, 0.3 * x + noise.next(1.0)))
      .collect();
    // 三个被噪声带偏的块。
    pts[5].1 += 150.0;
    pts[11].1 -= 200.0;
    pts[19].1 += 90.0;
    let c = fit_curve(&pts, 64.0).expect("少量离群点不该让拟合作废");
    assert!((c.mean_slope() - 0.3).abs() < 0.01, "{}", c.mean_slope());
  }

  #[test]
  fn too_few_or_too_scattered_points_give_nothing() {
    assert!(fit_curve(&[(24.0, 1.0), (72.0, 2.0), (120.0, 3.0)], 64.0).is_none());
    // 一半的点离群：留下的不到 60%。
    let mut pts: Vec<(f64, f64)> = xs(20).into_iter().map(|x| (x, 0.2 * x)).collect();
    for p in pts.iter_mut().step_by(2) {
      p.1 += 500.0;
    }
    assert!(fit_curve(&pts, 64.0).is_none());
  }

  #[test]
  fn prediction_ignores_a_single_bad_point() {
    // 最近 8 个点里有 1 个被带偏：中位数斜率与中位数截距都不受它影响。
    let mut pts: Vec<(f64, f64)> = xs(8).into_iter().map(|x| (x, 0.5 * x)).collect();
    pts[6].1 += 200.0;
    let x = block_centre(8);
    assert!((predict(&pts, x, 0.0) - 0.5 * x).abs() < 1.0);
    assert_eq!(predict(&[], x, 7.0), 7.0);
    assert_eq!(predict(&[(10.0, 3.0)], x, 7.0), 3.0);
  }

  #[test]
  fn peak_interpolation_is_exact_on_its_own_model() {
    // 三角形峰：峰在中心点右边 0.3 个步长处（幅度 1 − |x − 0.3|）。
    let tri = |x: f64| 1.0 - (x - 0.3).abs();
    assert!((triangle_peak(tri(-1.0), tri(0.0), tri(1.0)) - 0.3).abs() < 1e-12);
    assert!((triangle_peak(tri(1.0), tri(0.0), tri(-1.0)) + 0.3).abs() < 1e-12);
    // 抛物线峰：峰在左边 0.2 个步长处。
    let par = |x: f64| 1.0 - (x + 0.2).powi(2);
    assert!((parabola_peak(par(-1.0), par(0.0), par(1.0)) + 0.2).abs() < 1e-12);
    // 平的：不动。
    assert_eq!(triangle_peak(1.0, 1.0, 1.0), 0.0);
    assert_eq!(parabola_peak(1.0, 1.0, 1.0), 0.0);
  }

  #[test]
  fn fixed_drift_is_the_plain_grid() {
    let g = grid();
    let d = Drift::fixed(&g, 900.0, 10.0);
    assert_eq!(d.start_at(0.0), 10.0);
    assert_eq!(d.start_at(3.0), 10.0 + 3.0 * g.sym);
    assert_eq!(d.base_at(100.0), 900.0);
    assert_eq!(d.clock_ppm(), 0.0);
    assert_eq!(d.drift_hz_per_s(), 0.0);
  }

  #[test]
  fn a_curve_pushing_the_start_before_the_recording_drops_whole_symbols() {
    let g = grid();
    // 开头的曲线值是 −700 样本（> 一个符号 512）：前两个符号够不着，其余整体前移。
    let mut drift = Drift::fixed(&g, 900.0, 0.0);
    drift.timing = Curve {
      coef: vec![-700.0],
      centre: 0.0,
      half: 1.0,
      x_min: 0.0,
      x_max: 100.0,
    };
    assert!(drift.start_at(0.0) < 0.0);
    assert!(drift.drop_unreachable());
    assert_eq!(drift.first, 2.0);
    assert!((drift.start_at(0.0) - (2.0 * g.sym - 700.0)).abs() < 1e-9);
    // 新序号 5 对应名义序号 7。
    assert!((drift.start_at(5.0) - (7.0 * g.sym - 700.0)).abs() < 1e-9);
    let samples = vec![0.0f32; (g.sym * 120.0) as usize];
    assert!(!symbol_bins_tracked(&samples, &g, &drift).is_empty());
  }

  #[test]
  fn a_curve_that_would_drop_too_many_symbols_is_malformed() {
    let g = grid();
    // 开头比录音起点早 1000 万个样本（两万个符号，远超上限）：不是真实的漂移，不该丢这么多再继续。
    let mut drift = Drift::fixed(&g, 900.0, 0.0);
    drift.timing = Curve {
      coef: vec![-1.0e7],
      centre: 0.0,
      half: 1.0,
      x_min: 0.0,
      x_max: 100.0,
    };
    assert!(!drift.drop_unreachable());
  }

  #[test]
  fn silence_and_short_recordings_are_not_tracked() {
    let g = grid();
    let silence = vec![0.0f32; 8000 * 30];
    let bins = symbol_bins_tracked(&silence, &g, &Drift::fixed(&g, 900.0, 0.0));
    assert!(track(&silence, &g, 900.0, 0.0, &bins).is_none());
    // 不足 `MIN_BLOCKS` 块有信号。
    assert!(track(&silence, &g, 900.0, 0.0, &[]).is_none());
  }

  #[test]
  fn bins_tracked_without_a_correction_is_the_plain_grid_bit_for_bit() {
    let g = grid();
    let silence = vec![0.0f32; 8000 * 30];
    let (bins, drift) = bins_tracked(&silence, &g, 900.0, 0.0);
    assert_eq!(bins, symbol_bins(&silence, &g, 900.0, 0.0));
    assert_eq!(drift.clock_ppm(), 0.0);
    assert_eq!(drift.drift_hz_per_s(), 0.0);
    assert_eq!(drift.start_at(7.0), 7.0 * g.sym);
    assert_eq!(drift.base_at(7.0), 900.0);
  }
}
