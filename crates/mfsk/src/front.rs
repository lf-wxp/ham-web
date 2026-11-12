//! 离线文件专用的接收前端：符号窗口几何、中心频率检测、符号对齐、信号区间与质量门限。
//!
//! MFSK 与 THOR 共用（两者都是「每个符号一个音调、音调在一排等间隔的频点上」）：区别只在
//! 波特率、音调间隔与音调数，由 [`Grid`] 承载。fldigi 在这一段用滑动 FFT 逐样本跟踪；
//! 这里在整段录音上搜一次，时钟漂移与频漂由随后的 [`crate::track`] 另行跟踪。

use crate::tone_magnitudes;
use ham_web_dsp::tone_power;

/// 「最强音调 / 平均幅度」的比值（以下简称峰均比）在多宽的滑动窗口里取平均来划定信号区间。
const SMOOTH_SYMBOLS: usize = 8;

/// 滑动平均峰均比至少多大才算「这里有信号」，用来找信号的起止。
///
/// 一个干净的符号只有一个音调有能量：16 个音调时比值接近 16；纯噪声里 16 个瑞利分布幅度的
/// 最大值约为平均值的 1.8 倍（单个符号的波动约 ±0.25，8 个符号取平均后只剩约 ±0.09）。
/// MFSK16 在音调箱信噪比 +8 dB 时仍能译出，此时单符号峰均比约 2.9 —— 门限必须低于它，
/// 又要比噪声高出 6 个标准差以上，取 2.4。
const REGION_RATIO: f64 = 2.4;

/// 整个信号区间的平均峰均比下限（只有区间划定后才看）：比起逐符号计数，平均值对低信噪比稳得多。
const MIN_MEAN_RATIO: f64 = 2.6;

/// 区间至少多少个符号。更短的统计没有意义，也解不出一个完整字符。
const MIN_SYMBOLS: usize = 32;

/// 估计中心频率 / 符号对齐时最多看多少个最响的符号窗口。
const MAX_PROBE_WINDOWS: usize = 96;

/// 中心频率搜索的计算预算（Goertzel 乘加次数的量级），预算内尽量多看窗口。
const PROBE_BUDGET: usize = 60_000_000;

/// 符号窗口几何：窗口恰为一个符号时长；音调间隔必须是波特率的整数倍（MFSK 等于波特率，
/// THOR 是 1 / 2 / 4 倍），这样各音调在一个窗口里互相正交。
pub struct Grid {
  pub rate: u32,
  /// 波特率（Hz）= Goertzel 窗口的频率分辨力：音调只在恰好整数倍的位置上互相正交。
  pub baud: f64,
  /// 一个符号的样本数（小数，符号起点按它累加，不因取整累积偏移）。
  pub sym: f64,
  /// 取整后的 Goertzel 窗口长度。
  pub win: usize,
  /// 音调间隔（Hz）。
  pub spacing: f64,
  pub ntones: usize,
}

impl Grid {
  /// `None`：采样率为 0、参数非有限，或一个符号只有几个样本（Goertzel 没有频率分辨力，
  /// 也容易被畸形输入拖进无意义的计算）。
  #[must_use]
  pub fn new(rate: u32, baud: f64, spacing: f64, ntones: usize) -> Option<Self> {
    if rate == 0 || !baud.is_finite() || baud <= 0.0 || !spacing.is_finite() || spacing <= 0.0 {
      return None;
    }
    let sym = f64::from(rate) / baud;
    let win = sym.round() as usize;
    (win >= 16 && ntones >= 2).then_some(Self {
      rate,
      baud,
      sym,
      win,
      spacing,
      ntones,
    })
  }

  /// 信号带必须整个落在 Nyquist 以下（留一个音调间隔的余量，否则混叠后只会报误导性的无信号）。
  #[must_use]
  pub fn fits(&self, base: f64) -> bool {
    base >= 0.0 && base + (self.ntones as f64 + 1.0) * self.spacing < f64::from(self.rate) / 2.0
  }
}

/// 一段录音里**有信号**的符号窗口中，沿时间均匀抽出的 `count` 个窗口起点（样本位置，升序）。
///
/// 「有信号」= 窗口能量不低于最响 10% 窗口的一半。不能直接取能量最高的 `count` 个：
/// 恒包络信号（MFSK 就是）每个窗口的能量几乎一样，差别只来自音调频率与窗口的整周期关系，
/// 按能量排序等于偏向某几个音调 —— 中心频率检测会因此被带偏好几个音调（真实录音上实测过）。
pub fn signal_windows(samples: &[f32], grid: &Grid, count: usize) -> Vec<f64> {
  let n = ((samples.len() as f64) / grid.sym).floor() as usize;
  let energies: Vec<(usize, f64)> = (0..n)
    .filter_map(|i| {
      let start = (i as f64 * grid.sym).round() as usize;
      let w = samples.get(start..start + grid.win)?;
      Some((i, w.iter().map(|&x| f64::from(x) * f64::from(x)).sum()))
    })
    .collect();
  if energies.is_empty() {
    return Vec::new();
  }
  let mut sorted: Vec<f64> = energies.iter().map(|e| e.1).collect();
  sorted.sort_by(f64::total_cmp);
  let floor = sorted[sorted.len() * 9 / 10] * 0.5;
  let live: Vec<usize> = energies
    .iter()
    .filter(|e| e.1 >= floor && e.1 > 0.0)
    .map(|e| e.0)
    .collect();
  let take = count.min(live.len());
  (0..take)
    .map(|k| live[k * live.len() / take] as f64 * grid.sym)
    .collect()
}

/// 梳评分：`windows` 里每个窗口「梳内最强音调能量 − 梳两侧各一个邻位的最强能量」之和。
///
/// `power(w, k)` 取第 `w` 个窗口在梳起点往上第 `k` 个音调位（`k = -1` 与 `k = ntones` 是两个邻位）的功率。
fn comb_score(nwin: usize, nt: usize, power: &dyn Fn(usize, i32) -> f64) -> f64 {
  (0..nwin)
    .map(|w| {
      let comb = (0..nt as i32).map(|k| power(w, k)).fold(0.0, f64::max);
      comb - power(w, -1).max(power(w, nt as i32))
    })
    .sum()
}

/// 估计信号**最低音调**的频率（Hz）。
///
/// 三步，每步只在上一步缩小的范围里花算力：
/// 1. 少量窗口（≤ 16）在**一个窗口分辨力（= 波特率）**为步长的粗网格上扫过整个 0–Nyquist，找「音调梳」
///    的大致位置。步长必须取波特率而不是音调间隔：THOR 的音调间隔是波特率的 2 / 4 倍，音调若恰好落在
///    两个相隔一个间隔的网格点正中，到两边都差整整一个分辨力 —— 矩形窗在那里是**零点**，什么也测不到
///    （合成信号加一点点噪声就会选错，实测过）；
/// 2. 更多窗口（≤ 96）只在第 1 步结果上下各 4 个音调内重新评分 —— 窗口少时某些音调恰好没出现过，
///    梳的边缘会差几个音调，窗口多了才分得开；
/// 3. 在最优点附近以 1/8 音调间隔细扫。
///
/// 评分 = 梳内最强音调能量 − 梳两侧邻位的最强能量（[`comb_score`]）：梳错一个音调时，
/// 那个音调被发送的窗口会得分骤降。窗口取自「有信号」的符号并沿时间均匀抽取
/// （[`signal_windows`]）。录音里没有足够的符号窗口返回 `None`。
#[must_use]
pub fn detect_base(samples: &[f32], grid: &Grid) -> Option<f64> {
  let rate = grid.rate;
  let nyq = f64::from(rate) / 2.0;
  let (nt, sp) = (grid.ntones, grid.spacing);
  // 一个音调间隔是几个网格步长（MFSK = 1，THOR = 1 / 2 / 4）。
  let ds = (sp / grid.baud).round().max(1.0) as usize;
  // 粗网格：第 j 点在 `(j + 1) × baud`，直到 Nyquist；梳的第 k 个音调落在 `j0 + k × ds`。
  let j_max = (nyq / grid.baud).floor() as usize;
  if j_max < (nt + 3) * ds {
    return None;
  }
  let slice = |s: f64| {
    let start = s.round() as usize;
    &samples[start..start + grid.win]
  };
  // 梳起点的取值范围：左邻位 `j0 - ds` 与右邻位 `j0 + nt × ds` 都要在网格内。
  let j_hi = j_max - nt * ds - 1;

  // 第 1 步：整个频带，少量窗口。
  let coarse_windows = (PROBE_BUDGET / (j_max * grid.win)).clamp(4, 16);
  let wa = signal_windows(samples, grid, coarse_windows);
  if wa.len() < 4 {
    return None;
  }
  let ea: Vec<Vec<f64>> = wa
    .iter()
    .map(|&s| {
      let w = slice(s);
      (0..j_max)
        .map(|j| tone_power(w, rate, (j + 1) as f64 * grid.baud))
        .collect()
    })
    .collect();
  let tooth = |j0: usize, k: i32| (j0 as i32 + k * ds as i32) as usize;
  let (mut j_a, mut best) = (ds, f64::NEG_INFINITY);
  for j0 in ds..=j_hi {
    let score = comb_score(ea.len(), nt, &|w, k| ea[w][tooth(j0, k)]);
    if score > best {
      (j_a, best) = (j0, score);
    }
  }

  // 第 2 步：在第 1 步结果附近（上下各 4 个音调），用更多窗口重新评分。
  let lo = j_a.saturating_sub(4 * ds).max(ds);
  let hi = (j_a + 4 * ds).min(j_hi).max(lo);
  let first = lo - ds; // 需要的最低网格点
  let last = tooth(hi, nt as i32); // 需要的最高网格点
  let span = last - first + 1;
  let more = (PROBE_BUDGET / (span * grid.win)).clamp(8, MAX_PROBE_WINDOWS);
  let wb = signal_windows(samples, grid, more);
  let eb: Vec<Vec<f64>> = wb
    .iter()
    .map(|&s| {
      let w = slice(s);
      (first..=last)
        .map(|j| tone_power(w, rate, (j + 1) as f64 * grid.baud))
        .collect()
    })
    .collect();
  let (mut j_b, mut best) = (lo, f64::NEG_INFINITY);
  for j0 in lo..=hi {
    let score = comb_score(eb.len(), nt, &|w, k| eb[w][tooth(j0, k) - first]);
    if score > best {
      (j_b, best) = (j0, score);
    }
  }

  // 第 3 步：细扫 ±1.5 个音调间隔，1/8 间隔。窗口数再按预算收缩。
  let fine_windows = (PROBE_BUDGET / (25 * (nt + 2) * grid.win)).clamp(8, 48);
  let wc = signal_windows(samples, grid, fine_windows);
  let slices: Vec<&[f32]> = wc.iter().map(|&s| slice(s)).collect();
  let centre = (j_b + 1) as f64 * grid.baud;
  let mut refined = (f64::NEG_INFINITY, centre);
  for step in -12i32..=12 {
    let f0 = centre + f64::from(step) * sp / 8.0;
    if f0 - sp <= 0.0 || f0 + (nt as f64 + 1.0) * sp >= nyq {
      continue;
    }
    let score = comb_score(slices.len(), nt, &|w, k| {
      tone_power(slices[w], rate, f0 + f64::from(k) * sp)
    });
    if score > refined.0 {
      refined = (score, f0);
    }
  }
  refined.0.is_finite().then_some(refined.1)
}

/// 对齐之后再核对一遍中心频率：只在 `base` 上下各 2 个音调间隔内挑。
///
/// [`detect_base`] 取的窗口还没对齐（起点是名义位置），常常跨在两个符号上，证据被冲淡 ——
/// 噪声稍大一点，「梳」就可能差一个音调（实测过：THOR8 加一点点噪声就偏了）。符号对齐之后，每个窗口里
/// 只有一个音调，评分的区分度高得多，所以把候选梳在**对齐窗口**上重新评一次分。
#[must_use]
pub fn polish_base(samples: &[f32], grid: &Grid, base: f64, offset: f64) -> f64 {
  let (nt, sp, rate) = (grid.ntones, grid.spacing, grid.rate);
  let count = (PROBE_BUDGET / (5 * (nt + 2) * grid.win)).clamp(16, 192);
  let slices: Vec<&[f32]> = signal_windows(samples, grid, count)
    .into_iter()
    .filter_map(|s| {
      let start = (s + offset).round() as usize;
      samples.get(start..start + grid.win)
    })
    .collect();
  if slices.len() < 8 {
    return base;
  }
  let nyq = f64::from(rate) / 2.0;
  let mut best = (f64::NEG_INFINITY, base);
  // 0 放最前：并列时留原值。
  for k in [0i32, -1, 1, -2, 2] {
    let f0 = base + f64::from(k) * sp;
    if f0 - sp <= 0.0 || f0 + (nt as f64 + 1.0) * sp >= nyq {
      continue;
    }
    let score = comb_score(slices.len(), nt, &|w, j| {
      tone_power(slices[w], rate, f0 + f64::from(j) * sp)
    });
    if score > best.0 {
      best = (score, f0);
    }
  }
  best.1
}

/// 符号对齐：返回 `[0, sym)` 内使「每个窗口最强音调能量之和」最大的起点偏移（样本，小数）。
///
/// 对齐正确时一个符号的能量全落在一个音调上；错位的窗口跨两个符号、能量分散。
/// 两级搜索：先以 `sym / 16` 粗扫，再在最优点两侧各 3 个 `sym / 64` 细扫。
pub fn find_offset(samples: &[f32], grid: &Grid, base: f64) -> f64 {
  let windows = signal_windows(samples, grid, 64);
  let score = |offset: f64| -> f64 {
    windows
      .iter()
      .filter_map(|&s| {
        let start = (s + offset).round() as usize;
        let w = samples.get(start..start + grid.win)?;
        Some(
          (0..grid.ntones)
            .map(|k| tone_power(w, grid.rate, base + k as f64 * grid.spacing))
            .fold(0.0, f64::max),
        )
      })
      .sum()
  };
  let mut best = (f64::NEG_INFINITY, 0.0f64);
  for i in 0..16 {
    let o = grid.sym * f64::from(i) / 16.0;
    let s = score(o);
    if s > best.0 {
      best = (s, o);
    }
  }
  let centre = best.1;
  for d in -3i32..=3 {
    if d == 0 {
      continue;
    }
    let o = (centre + grid.sym * f64::from(d) / 64.0).rem_euclid(grid.sym);
    let s = score(o);
    if s > best.0 {
      best = (s, o);
    }
  }
  best.1
}

/// 每个符号各音调的幅度，按符号顺序平铺（`ntones` 个一组）。
///
/// 第 `n` 个符号的窗口起点是 `offset + n × grid.sym`（小数位置，逐符号累加，不因取整累积偏移）。
#[must_use]
pub fn symbol_bins(samples: &[f32], grid: &Grid, base: f64, offset: f64) -> Vec<f64> {
  let mut bins = Vec::new();
  let mut buf = vec![0.0f64; grid.ntones];
  let mut n = 0usize;
  while tone_magnitudes(
    samples,
    grid.rate,
    offset + n as f64 * grid.sym,
    grid.win,
    base,
    grid.spacing,
    &mut buf,
  ) {
    bins.extend_from_slice(&buf);
    n += 1;
  }
  bins
}

/// 每个符号的峰均比（静音的平均值为 0，比值无意义，记 0）。
#[must_use]
pub fn peak_ratios(bins: &[f64], ntones: usize) -> Vec<f64> {
  bins
    .chunks_exact(ntones)
    .map(|b| {
      let avg = b.iter().sum::<f64>() / ntones as f64;
      let max = b.iter().copied().fold(0.0, f64::max);
      if avg > 0.0 { max / avg } else { 0.0 }
    })
    .collect()
}

/// 质量门限：划出信号区间（首尾符号下标，含），并要求区间够长、平均峰均比够高。
///
/// 噪声与静音返回 `None`；没有这道门限时，Viterbi 会在噪声里拼出看似正常的乱码。
#[must_use]
pub fn gate(ratios: &[f64]) -> Option<(usize, usize)> {
  let (first, last) = signal_region(ratios)?;
  let span = last - first + 1;
  let mean = ratios[first..=last].iter().sum::<f64>() / span as f64;
  (span >= MIN_SYMBOLS && mean >= MIN_MEAN_RATIO).then_some((first, last))
}

/// 信号区间：滑动平均峰均比达到 [`REGION_RATIO`] 的第一个与最后一个符号。
fn signal_region(ratios: &[f64]) -> Option<(usize, usize)> {
  if ratios.len() < SMOOTH_SYMBOLS {
    return None;
  }
  let half = SMOOTH_SYMBOLS / 2;
  let smooth = |i: usize| {
    let lo = i.saturating_sub(half);
    let hi = (i + half).min(ratios.len());
    ratios[lo..hi].iter().sum::<f64>() / (hi - lo) as f64
  };
  let on: Vec<bool> = (0..ratios.len())
    .map(|i| smooth(i) >= REGION_RATIO)
    .collect();
  Some((on.iter().position(|&x| x)?, on.iter().rposition(|&x| x)?))
}
