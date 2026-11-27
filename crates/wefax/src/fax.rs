//! 真实的短波气象传真（WMO / ITU-T T.4 附录里的 radiofax）：连续的调频副载波，没有逐行同步。
//!
//! 与 [`crate::fm`] 那套「自拟口径」不同，这里按真实发射的结构解：
//!
//! - 副载波 1500 Hz = 黑、2300 Hz = 白（USB 接收，中心 1900 Hz，频偏 ±400 Hz）；
//! - 行速率 60 / 90 / 100 / 120 / 180 / 240 LPM，120 LPM 最常见；
//! - 合作指数 IOC 576（每行约 1810 像素）或 IOC 288（约 905 像素）；
//! - 开头依次是起始音（IOC 576 为 300 Hz、IOC 288 为 675 Hz）、**相位线**（约 30 s，每行是
//!   一条黑线加行首一个 5% 的白脉冲）、图像；图像里**没有**同步脉冲，行与行靠发收两端各自
//!   的时钟自由运行——时钟差几百 ppm 就会让图斜过来（斜率）。
//!
//! # 做法
//!
//! 1. **解调**：先混频到基带、低通并抽取，再取相邻样本的相位差，得到每个样本的瞬时频率。
//!    每个像素只有 2–3 个样本，所以必须用平滑的解调，不能像 [`crate::fm`] 那样逐像素拟合。
//! 2. **相位线**：找「黑背景里每隔一行出现一次的短白脉冲」。脉冲的前沿就是行首，脉冲间隔
//!    的最小二乘斜率给出发端的真实行周期（修掉斜率），最后一个脉冲之后是图像。
//! 3. **没有相位线**（例如截取的片段）：行周期取标称值，再用自相关在 ±1% 内微调；行首位置
//!    没有依据，任取 0，图像左右可能错开（页面可以把它转回来）。
//! 4. **黑白电平**：有相位线就以黑线与白脉冲的实测频率为准；没有就看整段频率分布的
//!    1% / 99% 分位——落在标称 1500 / 2300 Hz 附近就信标称，否则（接收机偏移或带宽被削窄）
//!    按分位拉伸。
//!
//! # 没有建模的
//!
//! - 起始音与停止音（不用来触发；停止音 450 Hz 在图像末尾会表现为黑条）；
//! - LSB 接收（黑白会倒过来）；
//! - 彩色传真（起始音 200 Hz 的那一族）；
//! - 噪声很大时的抗干扰（相位差解调在低信噪比下有尖峰，没有做中值滤波）。

use std::f64::consts::{PI, TAU};

use crate::{Error, WefaxImage};

/// 黑电平的副载波频率（Hz）。
pub const BLACK_HZ: f64 = 1500.0;
/// 白电平的副载波频率（Hz）。
pub const WHITE_HZ: f64 = 2300.0;
/// 副载波中心（Hz）。
pub const CENTER_HZ: f64 = 1900.0;
/// 可选的行速率（每分钟行数）。
pub const LPM_CHOICES: [u32; 6] = [60, 90, 100, 120, 180, 240];

/// 基带低通截止（Hz）：中心 ±400 Hz 的频偏再加像素速率的边带。
const LOWPASS_HZ: f64 = 700.0;
/// 解调后序列的目标采样率下限（Hz）：每个像素至少约 1.5 个样本。
const TARGET_RATE: f64 = 5500.0;
/// 相位线脉冲（白）的时长范围，占行周期的比例。
const PULSE_FRACTION: std::ops::RangeInclusive<f64> = 0.01..=0.12;
/// 脉冲前后各要有这么长（占行周期）的黑底。
const BLACK_MARGIN_FRACTION: f64 = 0.25;
/// 至少连着这么多条相位线才采信……
const MIN_PHASING_LINES: usize = 6;
/// ……并且至少持续这么久（秒）。真实相位线是 20–30 s；黑底白字的图头（每行同一处有一道竖笔画）
/// 也会形成一串等间距的白脉冲，但只有几行，靠时长把它们分开（240 LPM 的 14 行才 3.5 s）。
const MIN_PHASING_SECONDS: f64 = 8.0;
/// 相邻脉冲最多隔几个行周期（中间漏检的行数 + 1）。
const MAX_MISSED_STEP: f64 = 3.0;
/// 相邻脉冲的间隔相对标称行周期允许的偏差（每个行周期）。
const PULSE_SPACING_TOLERANCE: f64 = 0.015;
/// 没有相位线时，自相关微调行周期的范围（相对标称）。
const AUTOCORR_RANGE: f64 = 0.01;
/// 包络变异系数的上限（中位数）：纯噪声约 0.52，调频副载波远小于它。
const MAX_ENVELOPE_VARIATION: f64 = 0.3;
/// 精修行周期时最多用几行的滞后。
const MAX_LAG_LINES: usize = 16;
/// 1% 分位不高于「黑 + 它」、99% 分位不低于「白 − 它」，就直接用标称（没有失谐的录音）。
/// 只看「是否盖住了标称范围」、不管超出多少：硬边处的解调过冲与噪声毛刺会把两端各推出去几十 Hz，
/// 那不是电平。范围放大会把几十 Hz 的失谐当成标称，白平衡偏掉 10% 以上。
const NOMINAL_SNAP_HZ: f64 = 30.0;
/// 按分位拉伸至少要有这么大的跨度（Hz）：一整页白纸上有一点灰，分位跨度很小，拉伸会把它
/// 放大成满幅黑白，这时宁可信标称。
const MIN_STRETCH_SPAN_HZ: f64 = 250.0;

/// 合作指数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ioc {
  /// IOC 576（气象图最常见）。
  I576,
  /// IOC 288。
  I288,
}

impl Ioc {
  /// 每行像素数：`π × IOC`。
  #[must_use]
  pub fn width(self) -> usize {
    (PI * self.value()).round() as usize
  }

  fn value(self) -> f64 {
    match self {
      Self::I576 => 576.0,
      Self::I288 => 288.0,
    }
  }

  /// 给协议与页面用的稳定名字。
  #[must_use]
  pub fn code(self) -> &'static str {
    match self {
      Self::I576 => "576",
      Self::I288 => "288",
    }
  }

  /// 反过来：`"576"` / `"288"`。
  #[must_use]
  pub fn from_code(code: &str) -> Option<Self> {
    match code {
      "576" => Some(Self::I576),
      "288" => Some(Self::I288),
      _ => None,
    }
  }
}

/// 解码参数：发射台的行速率与合作指数（电台的广播时刻表上都有）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Params {
  pub lpm: u32,
  pub ioc: Ioc,
}

impl Default for Params {
  fn default() -> Self {
    Self {
      lpm: 120,
      ioc: Ioc::I576,
    }
  }
}

/// 黑白电平从哪来。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelSource {
  /// 相位线的黑线与白脉冲实测。
  Phasing,
  /// 整段频率分布与标称 1500 / 2300 Hz 吻合，按标称。
  Nominal,
  /// 整段频率分布的 1% / 99% 分位拉伸（接收机偏移或带宽被削窄）。
  Stretched,
}

/// 解码结果与诊断信息。
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
  pub image: WefaxImage,
  /// 实测的行速率（LPM）；与标称的差就是收发时钟差。
  pub lpm_measured: f64,
  /// 找到的相位线条数；没找到为 `None`（行首位置没有依据）。
  pub phasing_lines: Option<usize>,
  /// 实际采用的黑电平与白电平（Hz）。
  pub black_hz: f64,
  pub white_hz: f64,
  pub levels: LevelSource,
}

/// 解调后的瞬时频率序列。
pub(crate) struct Baseband {
  /// 每个样本的瞬时频率（Hz）。
  pub hz: Vec<f32>,
  /// 序列的采样率（Hz）。
  pub rate: f64,
  /// 低通后基带的包络（|I + jQ|）：调频副载波的包络近似恒定，噪声的包络是起伏的瑞利分布。
  pub envelope: Vec<f32>,
}

/// 混频 → 低通 + 抽取 → 相位差，得到瞬时频率序列。
pub(crate) fn demodulate(samples: &[f32], rate: f64) -> Baseband {
  let decim = ((rate / TARGET_RATE).round() as usize).max(1);
  let out_rate = rate / decim as f64;
  let taps = ((3.0 * rate / LOWPASS_HZ) as usize) | 1;
  let half = taps / 2;
  let kernel = lowpass(rate, taps);

  // 混频：用旋转相量递推，省掉逐样本的 sin / cos；每 1024 个样本把模长拉回 1。
  let step = TAU * CENTER_HZ / rate;
  let (sin_w, cos_w) = step.sin_cos();
  let (mut re, mut im) = (1.0_f64, 0.0_f64);
  let mut i_mix = Vec::with_capacity(samples.len());
  let mut q_mix = Vec::with_capacity(samples.len());
  for (n, &x) in samples.iter().enumerate() {
    let x = f64::from(x);
    i_mix.push((x * re) as f32);
    q_mix.push((-x * im) as f32);
    let (nr, ni) = (re * cos_w - im * sin_w, re * sin_w + im * cos_w);
    (re, im) = (nr, ni);
    if n % 1024 == 1023 {
      let norm = re.hypot(im);
      re /= norm;
      im /= norm;
    }
  }

  let outputs = samples.len() / decim;
  let filter = |s: &[f32], at: usize| -> f64 {
    let centre = at * decim;
    let lo = centre.saturating_sub(half);
    let hi = (centre + half + 1).min(s.len());
    let mut acc = 0.0_f64;
    for (j, &v) in s[lo..hi].iter().enumerate() {
      // `lo` 在左边界被截断时，卷积核也要从对应位置起。
      let k = j + lo + half - centre;
      acc += f64::from(kernel[k]) * f64::from(v);
    }
    acc
  };
  let mut hz = Vec::with_capacity(outputs);
  let mut envelope = Vec::with_capacity(outputs);
  let (mut prev_i, mut prev_q) = (0.0_f64, 0.0_f64);
  for m in 0..outputs {
    let (i_f, q_f) = (filter(&i_mix, m), filter(&q_mix, m));
    let dot = i_f * prev_i + q_f * prev_q;
    let cross = q_f * prev_i - i_f * prev_q;
    let value = if m == 0 {
      CENTER_HZ
    } else {
      CENTER_HZ + cross.atan2(dot) * out_rate / TAU
    };
    hz.push(value as f32);
    envelope.push(i_f.hypot(q_f) as f32);
    (prev_i, prev_q) = (i_f, q_f);
  }
  Baseband {
    hz,
    rate: out_rate,
    envelope,
  }
}

/// 包络是否近似恒定：约 8 ms 一窗的（标准差 / 均值），取各窗的中位数。
///
/// 调频副载波的包络恒定（只在频率跳变处有短暂的凹陷），中位数很小；带限噪声的包络是瑞利分布，
/// 变异系数约 0.52。「瞬时频率落在 1200–2600 Hz」这条判不出噪声——低通之后的带限噪声，其
/// 相位差也集中在中心附近。
fn envelope_variation(envelope: &[f32], rate: f64) -> f64 {
  let window = ((0.008 * rate) as usize).max(8);
  let mut cvs: Vec<f64> = envelope
    .chunks_exact(window)
    .filter_map(|w| {
      let mean = w.iter().map(|&v| f64::from(v)).sum::<f64>() / window as f64;
      if mean <= 1e-9 {
        return None;
      }
      let var = w
        .iter()
        .map(|&v| (f64::from(v) - mean).powi(2))
        .sum::<f64>()
        / window as f64;
      Some(var.sqrt() / mean)
    })
    .collect();
  if cvs.is_empty() {
    return f64::INFINITY;
  }
  cvs.sort_by(f64::total_cmp);
  cvs[cvs.len() / 2]
}

/// 汉明窗 sinc 低通，直流增益归一。
fn lowpass(rate: f64, taps: usize) -> Vec<f32> {
  let mid = (taps - 1) as f64 / 2.0;
  let mut h: Vec<f64> = (0..taps)
    .map(|i| {
      let n = i as f64 - mid;
      let sinc = if n == 0.0 {
        2.0 * LOWPASS_HZ / rate
      } else {
        (TAU * LOWPASS_HZ / rate * n).sin() / (PI * n)
      };
      let window = 0.54 - 0.46 * (TAU * i as f64 / (taps - 1) as f64).cos();
      sinc * window
    })
    .collect();
  let sum: f64 = h.iter().sum();
  h.iter_mut().for_each(|v| *v /= sum);
  h.into_iter().map(|v| v as f32).collect()
}

/// 相位线：一串等间距的白脉冲。
struct Phasing {
  /// 第一个脉冲的前沿（序列下标，带小数）。
  first: f64,
  /// 最小二乘的脉冲间隔（序列样本数）。
  period: f64,
  /// 脉冲条数。
  lines: usize,
}

/// 找相位线：黑底里每隔一个行周期出现一次的短白脉冲，连着至少 [`MIN_PHASING_LINES`] 条。
///
/// 「黑」「白」的分界按整段分布估出的黑白电平定（中点 ± 20% 的跨度），不写死 1700 / 2100 Hz：
/// 接收机偏移或带宽被削窄的录音，白只到 2050 Hz，写死的阈值会把整段相位线看漏。
fn find_phasing(
  hz: &[f32],
  rate: f64,
  nominal_period: f64,
  black: f64,
  white: f64,
) -> Option<Phasing> {
  let (mid, span) = (0.5 * (black + white), white - black);
  // 白的阈值在中点之上 20% 跨度处：实测取中点（0%）或更低时，边沿处解调的纹波让脉冲
  // 前沿抖动，行周期的最小二乘斜率变差（合成样本里 60 LPM 量成 59.993，偏了 0.011%）；
  // 20% 以上稳定。
  let white_above = (mid + 0.2 * span) as f32;
  let black_below = (mid - 0.2 * span) as f32;
  let is_white = |f: f32| f > white_above;
  let is_black = |f: f32| f < black_below;
  let margin = (BLACK_MARGIN_FRACTION * nominal_period) as usize;
  let black_share = |a: usize, b: usize| -> f64 {
    let seg = &hz[a.min(hz.len())..b.min(hz.len())];
    if seg.is_empty() {
      return 0.0;
    }
    seg.iter().filter(|&&f| is_black(f)).count() as f64 / seg.len() as f64
  };

  // 白脉冲候选：前后都有足够的黑。
  let mut starts: Vec<f64> = Vec::new();
  let mut i = 0;
  while i < hz.len() {
    if !is_white(hz[i]) {
      i += 1;
      continue;
    }
    let s = i;
    while i < hz.len() && is_white(hz[i]) {
      i += 1;
    }
    let len = (i - s) as f64;
    if PULSE_FRACTION.contains(&(len / nominal_period))
      && s >= margin
      && black_share(s - margin, s) > 0.85
      && black_share(i, i + margin) > 0.85
    {
      // 取整的偏差是个常数（不超过一个样本），对行周期的斜率没有影响；
      // 在边沿处做亚样本内插反而被解调纹波带得抖动，斜率变差。
      starts.push(s as f64);
    }
  }

  // 连着的链：相邻脉冲的间隔是一到三个行周期（噪声里会漏掉个别脉冲）。取跨度最长的。
  // 每个元素是（行序号，位置）；行序号随间隔累加，漏掉的行也占着序号。
  let mut best: Vec<(usize, f64)> = Vec::new();
  let mut cur: Vec<(usize, f64)> = Vec::new();
  let span_of = |c: &[(usize, f64)]| c.last().map_or(0, |l| l.0 - c[0].0 + 1);
  for &s in &starts {
    let step = cur.last().and_then(|&(idx, pos)| {
      let ratio = (s - pos) / nominal_period;
      let n = ratio.round();
      ((1.0..=MAX_MISSED_STEP).contains(&n) && (ratio - n).abs() <= PULSE_SPACING_TOLERANCE * n)
        .then_some(idx + n as usize)
    });
    match step {
      Some(idx) => cur.push((idx, s)),
      None => {
        if span_of(&cur) > span_of(&best) {
          best = std::mem::take(&mut cur);
        }
        cur = vec![(0, s)];
      }
    }
  }
  if span_of(&cur) > span_of(&best) {
    best = cur;
  }
  let lines = span_of(&best);
  let min_lines =
    MIN_PHASING_LINES.max((MIN_PHASING_SECONDS * rate / nominal_period).ceil() as usize);
  // 脉冲本身也不能太稀：至少一半的行要真的检出，否则只是巧合排在一起的几个白点。
  if lines < min_lines || best.len() * 2 < lines {
    return None;
  }

  // 最小二乘：位置 = first + 行序号 × period。
  let n = best.len() as f64;
  let (sx, sy, sxx, sxy) = best.iter().fold((0.0, 0.0, 0.0, 0.0), |a, &(idx, pos)| {
    let x = idx as f64;
    (a.0 + x, a.1 + pos, a.2 + x * x, a.3 + x * pos)
  });
  let period = (n * sxy - sx * sy) / (n * sxx - sx * sx);
  let first = (sy - period * sx) / n;
  Some(Phasing {
    first,
    period,
    lines,
  })
}

/// 没有相位线时用自相关在标称周期附近微调行周期。
///
/// 先用单行滞后粗定，再用「若干行」的滞后精修：滞后放大 m 倍、峰位的精度就提高 m 倍，
/// 斜率才压得下去（1200 行的图，0.01% 的周期误差就是 12% 行宽的斜）。
fn refine_period(hz: &[f32], rate: f64, nominal_period: f64) -> f64 {
  // 先降到约 1.4 kHz，自相关只需要粗网格，抛物线内插给出亚网格精度。
  let group = ((rate / 1400.0).floor() as usize).max(1);
  let coarse: Vec<f32> = hz
    .chunks_exact(group)
    .map(|c| c.iter().sum::<f32>() / group as f32)
    .collect();
  let mean = coarse.iter().sum::<f32>() / coarse.len().max(1) as f32;
  let g: Vec<f32> = coarse.iter().map(|v| v - mean).collect();
  let nominal = nominal_period / group as f64;
  let corr = |lag: usize| -> f64 {
    g[..g.len() - lag]
      .iter()
      .zip(&g[lag..])
      .map(|(a, b)| f64::from(*a) * f64::from(*b))
      .sum()
  };
  // 在 `[centre − half, centre + half]` 里找相关最大的滞后，抛物线内插；峰在边上返回 None。
  let peak = |centre: f64, half: f64| -> Option<f64> {
    let (lo, hi) = (
      (centre - half).floor() as usize,
      (centre + half).ceil() as usize,
    );
    if lo < 2 || g.len() < hi + 2 {
      return None;
    }
    let best = (lo..=hi).max_by(|&a, &b| corr(a).total_cmp(&corr(b)))?;
    if best <= lo || best >= hi {
      return None;
    }
    let (y0, y1, y2) = (corr(best - 1), corr(best), corr(best + 1));
    let denom = y0 - 2.0 * y1 + y2;
    let offset = if denom.abs() < f64::MIN_POSITIVE {
      0.0
    } else {
      0.5 * (y0 - y2) / denom
    };
    Some(best as f64 + offset)
  };
  // 至少要有三个行周期才谈得上相关。
  if g.len() < 4 * nominal as usize {
    return nominal_period;
  }
  let Some(coarse_period) = peak(nominal, nominal * AUTOCORR_RANGE + 1.0) else {
    return nominal_period;
  };
  // 精修：m 行的滞后，搜索窗只开到粗估误差（±1.5 个网格）放大 m 倍。
  let lines_available = (g.len() as f64 / coarse_period) as usize;
  let m = (lines_available / 4).clamp(1, MAX_LAG_LINES);
  let fine = if m > 1 {
    let half = 1.5 * m as f64;
    peak(coarse_period * m as f64, half).map(|lag| lag / m as f64)
  } else {
    None
  };
  fine.unwrap_or(coarse_period) * group as f64
}

/// 整段频率分布给出的黑白电平：`(黑, 白, 来源)`。
///
/// 看 1% / 99% 分位（10 Hz 一格，只数 1000–2800 Hz 内的）：盖住标称 1500 / 2300 Hz 就信标称，
/// 否则（接收机失谐或带宽被削窄）按分位拉伸。
fn distribution_levels(hz: &[f32]) -> (f64, f64, LevelSource) {
  let mut hist = [0usize; 180];
  let mut total = 0usize;
  for &f in hz {
    let bin = ((f - 1000.0) / 10.0).floor();
    if (0.0..180.0).contains(&bin) {
      hist[bin as usize] += 1;
      total += 1;
    }
  }
  if total == 0 {
    return (BLACK_HZ, WHITE_HZ, LevelSource::Nominal);
  }
  let percentile = |share: f64| -> f64 {
    let target = (total as f64 * share) as usize;
    let mut acc = 0usize;
    for (i, &c) in hist.iter().enumerate() {
      acc += c;
      if acc > target {
        return 1000.0 + i as f64 * 10.0 + 5.0;
      }
    }
    2800.0
  };
  let (low, high) = (percentile(0.01), percentile(0.99));
  if low <= BLACK_HZ + NOMINAL_SNAP_HZ && high >= WHITE_HZ - NOMINAL_SNAP_HZ {
    (BLACK_HZ, WHITE_HZ, LevelSource::Nominal)
  } else if high - low >= MIN_STRETCH_SPAN_HZ {
    (low, high, LevelSource::Stretched)
  } else {
    (BLACK_HZ, WHITE_HZ, LevelSource::Nominal)
  }
}

/// 相位线给出的黑白电平（黑线与白脉冲的实测中位数）；脉冲与黑线差不到 150 Hz 就不信。
fn phasing_levels(hz: &[f32], p: &Phasing) -> Option<(f64, f64)> {
  let (mut blacks, mut whites) = (Vec::new(), Vec::new());
  for k in 0..p.lines {
    let start = p.first + k as f64 * p.period;
    // 脉冲占 5%：取前沿之后 1.5%–3% 当白；前沿之后 30%–80% 当黑。
    let seg = |a: f64, b: f64| -> Vec<f32> {
      let (a, b) = (
        (start + a * p.period) as isize,
        (start + b * p.period) as isize,
      );
      (a.max(0) as usize..(b.max(0) as usize).min(hz.len()))
        .map(|i| hz[i])
        .collect()
    };
    whites.extend(seg(0.015, 0.03));
    blacks.extend(seg(0.30, 0.80));
  }
  let (b, w) = (median(&mut blacks)?, median(&mut whites)?);
  (w - b > 150.0).then_some((b, w))
}

fn median(v: &mut [f32]) -> Option<f64> {
  if v.is_empty() {
    return None;
  }
  v.sort_by(f32::total_cmp);
  Some(f64::from(v[v.len() / 2]))
}

/// `[a, b)`（序列下标，带小数）上的平均频率；区间里没有整数点时在中点线性内插。
fn average(hz: &[f32], a: f64, b: f64) -> f32 {
  let (lo, hi) = (
    a.ceil().max(0.0) as usize,
    (b.ceil().max(0.0) as usize).min(hz.len()),
  );
  if hi > lo {
    return hz[lo..hi].iter().sum::<f32>() / (hi - lo) as f32;
  }
  let mid = 0.5 * (a + b);
  let i = (mid.floor().max(0.0) as usize).min(hz.len().saturating_sub(1));
  let j = (i + 1).min(hz.len() - 1);
  let frac = (mid - i as f64).clamp(0.0, 1.0) as f32;
  hz[i] * (1.0 - frac) + hz[j] * frac
}

/// 解码一段真实传真录音（归一化单声道样本与采样率）。
pub fn decode_samples(samples: &[f32], sample_rate: u32, params: Params) -> Result<Decoded, Error> {
  if sample_rate < 4000 {
    // 副载波最高约 2.4 kHz，低于 4 kHz 采样就混叠了。
    return Err(Error::UnsupportedFormat);
  }
  let rate = f64::from(sample_rate);
  let nominal_period_s = 60.0 / f64::from(params.lpm);
  if (samples.len() as f64) < 4.0 * nominal_period_s * rate {
    return Err(Error::TooShort);
  }
  let base = demodulate(samples, rate);
  let hz = &base.hz;
  // 不是调频副载波（噪声、别的模式）就没有可解的东西：瞬时频率多数要落在工作带里，包络
  // 还要近似恒定——光看频率判不出带限噪声（低通之后它的相位差也集中在中心附近）。
  let in_band = hz
    .iter()
    .filter(|&&f| (1200.0..=2600.0).contains(&f))
    .count();
  let variation = envelope_variation(&base.envelope, base.rate);
  if in_band * 2 < hz.len() || variation > MAX_ENVELOPE_VARIATION {
    return Err(Error::NoSync);
  }

  let nominal_period = nominal_period_s * base.rate;
  let (dist_black, dist_white, dist_source) = distribution_levels(hz);
  let phasing = find_phasing(hz, base.rate, nominal_period, dist_black, dist_white);
  let (period, origin, phasing_lines) = match &phasing {
    Some(p) => (p.period, p.first + p.lines as f64 * p.period, Some(p.lines)),
    None => (refine_period(hz, base.rate, nominal_period), 0.0, None),
  };
  let (black, white, source) = match phasing.as_ref().and_then(|p| phasing_levels(hz, p)) {
    Some((b, w)) => (b, w, LevelSource::Phasing),
    None => (dist_black, dist_white, dist_source),
  };

  let lines = ((hz.len() as f64 - origin) / period).floor();
  if lines < 1.0 {
    return Err(Error::TooShort);
  }
  let lines = lines as usize;
  let width = params.ioc.width();
  let mut gray = Vec::with_capacity(lines * width);
  for line in 0..lines {
    for p in 0..width {
      let a = origin + (line as f64 + p as f64 / width as f64) * period;
      let b = origin + (line as f64 + (p + 1) as f64 / width as f64) * period;
      let level = ((white - f64::from(average(hz, a, b))) / (white - black)).clamp(0.0, 1.0);
      gray.push((level * 255.0).round() as u8);
    }
  }
  Ok(Decoded {
    image: WefaxImage {
      width,
      height: lines,
      gray,
    },
    lpm_measured: 60.0 / (period / base.rate),
    phasing_lines,
    black_hz: black,
    white_hz: white,
    levels: source,
  })
}

/// [`modulate`] 的选项。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModulateOptions {
  pub params: Params,
  /// 输出采样率（Hz）。
  pub rate: u32,
  /// 相位线的条数（0 = 不发相位线，模拟截取的片段）。
  pub phasing_lines: usize,
  /// 发端时钟相对标称的比值：1.002 = 发端快 0.2%，收端看到行周期短 0.2%、图像斜过来。
  pub clock: f64,
  /// 整条副载波平移（Hz），模拟接收机调谐不准。
  pub shift_hz: f64,
  /// 黑 / 白电平的副载波频率（Hz）；标称 1500 / 2300，削窄带宽的接收机会更窄。
  pub black_hz: f64,
  pub white_hz: f64,
}

impl ModulateOptions {
  /// 标称参数：8 kHz、20 条相位线、时钟与调谐都准。
  #[must_use]
  pub fn new(params: Params, rate: u32) -> Self {
    Self {
      params,
      rate,
      phasing_lines: 20,
      clock: 1.0,
      shift_hz: 0.0,
      black_hz: BLACK_HZ,
      white_hz: WHITE_HZ,
    }
  }
}

/// 合成一段真实格式的传真：起始音、相位线、图像，各行连续、没有逐行同步。
///
/// 图像逐像素取常频（相位连续）。只用于合成对拍与样本生成。
pub fn modulate(image: &WefaxImage, options: &ModulateOptions) -> Result<Vec<f32>, Error> {
  let ModulateOptions {
    params,
    rate,
    phasing_lines,
    clock,
    shift_hz,
    black_hz,
    white_hz,
  } = *options;
  if image.width != params.ioc.width() || image.gray.len() != image.width * image.height {
    return Err(Error::InvalidImage);
  }
  let rate = f64::from(rate);
  let line_s = 60.0 / f64::from(params.lpm) / clock;
  let mut out: Vec<f32> = Vec::new();
  let mut phase = 0.0_f64;
  // 逐段发：时长累计到全局时间轴再取整，避免每段各自取整带来行周期漂移。
  let mut emitted = 0usize;
  let mut clock_s = 0.0_f64;
  let mut tone = |hz: f64, seconds: f64, out: &mut Vec<f32>| {
    clock_s += seconds;
    let end = (clock_s * rate).round() as usize;
    let step = TAU * (hz + shift_hz) / rate;
    while emitted < end {
      out.push((0.8 * phase.sin()) as f32);
      phase = (phase + step) % TAU;
      emitted += 1;
    }
  };
  // 起始音（IOC 576 为 300 Hz，IOC 288 为 675 Hz）：只发 1 s，不参与解码。
  let start_tone = if params.ioc == Ioc::I576 {
    300.0
  } else {
    675.0
  };
  tone(start_tone, 1.0, &mut out);
  for _ in 0..phasing_lines {
    tone(white_hz, 0.05 * line_s, &mut out);
    tone(black_hz, 0.95 * line_s, &mut out);
  }
  let pixel_s = line_s / image.width as f64;
  for row in 0..image.height {
    for col in 0..image.width {
      let level = f64::from(image.gray[row * image.width + col]) / 255.0;
      tone(white_hz - (white_hz - black_hz) * level, pixel_s, &mut out);
    }
  }
  tone(450.0, 0.5, &mut out);
  Ok(out)
}

/// 解码一段真实传真的 WAV 字节。
pub fn decode(bytes: &[u8], params: Params) -> Result<Decoded, Error> {
  let (samples, rate) = ham_web_wav::parse(bytes)?;
  decode_samples(&samples, rate, params)
}

#[cfg(test)]
mod tests {
  use super::*;

  const IMAGE_LINES: usize = 40;

  /// 一幅有渐变、粗黑块、细竖线、白块的图：既有平滑区也有硬边与 3 像素宽的细线。
  fn chart(ioc: Ioc, height: usize) -> WefaxImage {
    let width = ioc.width();
    let mut gray = Vec::with_capacity(width * height);
    for row in 0..height {
      for col in 0..width {
        let x = col as f64 / width as f64;
        // 粗黑块与每 150 列一条的 3 像素细竖线都是黑。
        let v = if ((8..16).contains(&row) && (0.3..0.5).contains(&x)) || col % 150 < 3 {
          255
        } else if (22..28).contains(&row) && (0.6..0.8).contains(&x) {
          0 // 白块
        } else {
          (x * 170.0) as u8 + 20 // 渐变
        };
        gray.push(v);
      }
    }
    WefaxImage {
      width,
      height,
      gray,
    }
  }

  fn mean_abs_diff(a: &[u8], b: &[u8]) -> f64 {
    a.iter()
      .zip(b)
      .map(|(x, y)| (f64::from(*x) - f64::from(*y)).abs())
      .sum::<f64>()
      / a.len() as f64
  }

  /// 把 `image` 每行循环右移 `by` 个像素（行首位置没有依据时，解出的图与原图差这么个循环位移）。
  fn rotate(image: &WefaxImage, by: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(image.gray.len());
    for row in image.gray.chunks(image.width) {
      out.extend(row[by..].iter().chain(&row[..by]));
    }
    out
  }

  /// 解出的图与原图在**最佳（行偏移，循环位移）**下的平均误差。
  ///
  /// 没有相位线时，行首位置没有依据（循环位移），开头的起始音还会占掉几行（行偏移）。
  /// 位移先按 8 像素粗扫、再在最佳点附近逐像素细扫。
  fn best_alignment(decoded: &WefaxImage, original: &WefaxImage) -> (f64, usize, usize) {
    let mut best = (f64::MAX, 0, 0);
    for skip in 0..=3usize {
      let lines = decoded.height.saturating_sub(skip).min(original.height);
      if lines < 10 {
        continue;
      }
      let a = &decoded.gray[skip * decoded.width..(skip + lines) * decoded.width];
      let orig = WefaxImage {
        width: original.width,
        height: lines,
        gray: original.gray[..lines * original.width].to_vec(),
      };
      let err_at = |by: usize| mean_abs_diff(a, &rotate(&orig, by % orig.width));
      let coarse = (0..orig.width)
        .step_by(8)
        .map(|by| (err_at(by), by))
        .min_by(|x, y| x.0.total_cmp(&y.0))
        .unwrap();
      let fine = (coarse.1 + orig.width - 8..=coarse.1 + orig.width + 8)
        .map(|by| (err_at(by), by % orig.width))
        .min_by(|x, y| x.0.total_cmp(&y.0))
        .unwrap();
      if fine.0 < best.0 {
        best = (fine.0, skip, fine.1);
      }
    }
    best
  }

  /// 区域 `[rows) × [cols)` 的平均灰度。
  fn region_mean(
    img: &WefaxImage,
    rows: std::ops::Range<usize>,
    cols: std::ops::Range<usize>,
  ) -> f64 {
    let mut sum = 0.0;
    let mut n = 0.0;
    for r in rows {
      for c in cols.clone() {
        sum += f64::from(img.gray[r * img.width + c]);
        n += 1.0;
      }
    }
    sum / n
  }

  fn opts(rate: u32) -> ModulateOptions {
    ModulateOptions::new(Params::default(), rate)
  }

  #[test]
  fn round_trip_with_phasing_at_each_sample_rate() {
    // 8 kHz 时每个像素只有 2.2 个样本，误差比 11 / 44 kHz 大一倍，但图还是对的。
    for (rate, limit) in [(8000u32, 8.0), (11_025, 4.0), (44_100, 4.0)] {
      let img = chart(Ioc::I576, IMAGE_LINES);
      let audio = modulate(&img, &opts(rate)).unwrap();
      let d = decode_samples(&audio, rate, Params::default()).unwrap();
      assert_eq!(
        (d.image.width, d.image.height),
        (1810, IMAGE_LINES),
        "{rate} Hz"
      );
      assert_eq!(d.phasing_lines, Some(20), "{rate} Hz");
      assert_eq!(d.levels, LevelSource::Phasing, "{rate} Hz");
      assert!(
        (d.lpm_measured - 120.0).abs() < 0.012,
        "{rate} Hz：{}",
        d.lpm_measured
      );
      assert!((d.black_hz - BLACK_HZ).abs() < 3.0 && (d.white_hz - WHITE_HZ).abs() < 3.0);
      let err = mean_abs_diff(&d.image.gray, &img.gray);
      assert!(err < limit, "{rate} Hz：平均误差 {err:.2}");
      // 粗黑块与白块的内部要分得开（边缘会被解调抹开几个像素，所以只看内部）。
      assert!(
        region_mean(&d.image, 9..15, 600..880) > 245.0,
        "{rate} Hz 黑块"
      );
      assert!(
        region_mean(&d.image, 23..27, 1130..1420) < 10.0,
        "{rate} Hz 白块"
      );
    }
  }

  #[test]
  fn clock_error_is_measured_and_the_slant_removed() {
    // 发端时钟快 / 慢 0.2%：不修的话每行错开 3 个像素，40 行就是 120 个像素的斜。
    // 有相位线时用脉冲位置的最小二乘斜率，没有时用多行滞后的自相关；两条路都要把行周期量准。
    for clock in [1.002, 0.998] {
      for phasing in [20usize, 0] {
        let img = chart(Ioc::I576, IMAGE_LINES);
        let mut o = opts(11_025);
        o.clock = clock;
        o.phasing_lines = phasing;
        let d = decode_samples(&modulate(&img, &o).unwrap(), 11_025, Params::default()).unwrap();
        let truth = 120.0 * clock;
        assert!(
          (d.lpm_measured - truth).abs() < 0.012,
          "时钟 {clock}、相位线 {phasing}：量到 {:.4}，真值 {truth:.4}",
          d.lpm_measured
        );
        assert_eq!(d.phasing_lines.is_some(), phasing > 0);
        let (err, _, _) = best_alignment(&d.image, &img);
        assert!(
          err < 4.0,
          "时钟 {clock}、相位线 {phasing}：平均误差 {err:.2}"
        );
      }
    }
  }

  #[test]
  fn a_mistuned_receiver_is_calibrated_from_the_phasing_or_the_distribution() {
    // 接收机失谐 ±80 Hz：整条副载波平移，黑白电平跟着移。
    for shift in [80.0, -80.0] {
      let img = chart(Ioc::I576, IMAGE_LINES);
      let mut o = opts(11_025);
      o.shift_hz = shift;
      // 有相位线：电平取自相位线。行首位置靠脉冲前沿外推，误差在几个像素内（≈ 0.3% 行宽）。
      let d = decode_samples(&modulate(&img, &o).unwrap(), 11_025, Params::default()).unwrap();
      assert_eq!(d.levels, LevelSource::Phasing);
      assert!(
        (d.black_hz - (BLACK_HZ + shift)).abs() < 3.0,
        "黑 {}",
        d.black_hz
      );
      assert!(
        (d.white_hz - (WHITE_HZ + shift)).abs() < 3.0,
        "白 {}",
        d.white_hz
      );
      let (err, skip, by) = best_alignment(&d.image, &img);
      let off = by.min(img.width - by);
      assert_eq!(skip, 0);
      assert!(off <= 6, "行首位置偏了 {off} 个像素");
      assert!(err < 4.0, "失谐 {shift} Hz、有相位线：平均误差 {err:.2}");
      // 没有相位线：电平取自整段分布的分位（误差略大，分位不是精确的黑白）。
      o.phasing_lines = 0;
      let d = decode_samples(&modulate(&img, &o).unwrap(), 11_025, Params::default()).unwrap();
      assert_eq!(d.levels, LevelSource::Stretched);
      assert!(
        (d.black_hz - (BLACK_HZ + shift)).abs() < 15.0,
        "黑 {}",
        d.black_hz
      );
      assert!(
        (d.white_hz - (WHITE_HZ + shift)).abs() < 15.0,
        "白 {}",
        d.white_hz
      );
      let (err, _, _) = best_alignment(&d.image, &img);
      assert!(err < 5.0, "失谐 {shift} Hz、无相位线：平均误差 {err:.2}");
    }
  }

  #[test]
  fn a_narrow_bandwidth_recording_is_stretched_to_full_range() {
    // 真实样本 `dwd_chart_8k.wav` 就是这样：黑 1725 Hz、白 2095 Hz，只有标称 800 Hz 跨度的一半不到。
    let img = chart(Ioc::I576, IMAGE_LINES);
    let mut o = opts(11_025);
    o.black_hz = 1725.0;
    o.white_hz = 2095.0;
    for phasing in [20usize, 0] {
      o.phasing_lines = phasing;
      let d = decode_samples(&modulate(&img, &o).unwrap(), 11_025, Params::default()).unwrap();
      let expected = if phasing > 0 {
        LevelSource::Phasing
      } else {
        LevelSource::Stretched
      };
      assert_eq!(d.levels, expected);
      let (err, _, _) = best_alignment(&d.image, &img);
      assert!(err < 5.0, "相位线 {phasing}：平均误差 {err:.2}");
    }
  }

  #[test]
  fn other_ioc_and_line_speeds() {
    for (ioc, lpm) in [
      (Ioc::I288, 120u32),
      (Ioc::I576, 60),
      (Ioc::I576, 90),
      (Ioc::I576, 100),
      (Ioc::I576, 180),
      (Ioc::I576, 240),
    ] {
      let params = Params { lpm, ioc };
      let img = chart(ioc, IMAGE_LINES);
      let mut o = ModulateOptions::new(params, 11_025);
      // 相位线要持续够 8 s 才采信：按行速率折算条数。
      o.phasing_lines = (10.0 * f64::from(lpm) / 60.0).ceil() as usize;
      let d = decode_samples(&modulate(&img, &o).unwrap(), 11_025, params).unwrap();
      assert_eq!(d.image.width, ioc.width(), "{lpm} LPM IOC {}", ioc.code());
      // 末尾 0.5 s 的停止音在快速率下会多占几行（240 LPM 是 2 行）。
      assert!(
        (IMAGE_LINES..=IMAGE_LINES + 3).contains(&d.image.height),
        "{lpm} LPM：{}",
        d.image.height
      );
      assert!(
        (d.lpm_measured - f64::from(lpm)).abs() < f64::from(lpm) * 1e-4,
        "{lpm} LPM：{}",
        d.lpm_measured
      );
      let err = mean_abs_diff(&d.image.gray[..img.gray.len()], &img.gray);
      assert!(err < 8.0, "{lpm} LPM IOC {}：平均误差 {err:.2}", ioc.code());
    }
    assert_eq!((Ioc::I576.width(), Ioc::I288.width()), (1810, 905));
    assert_eq!(Ioc::from_code("288"), Some(Ioc::I288));
    assert_eq!(Ioc::from_code("500"), None);
  }

  #[test]
  fn a_short_black_header_with_a_stroke_is_not_taken_for_phasing() {
    // 黑底白字的图头在每行同一处有一道竖笔画，形成一串等间距的白脉冲——和相位线长得一样。
    // 只能靠时长分开：真实相位线是 20–30 s，这里 14 行 120 LPM 才 7 s。
    let mut img = chart(Ioc::I576, IMAGE_LINES);
    for row in 0..14 {
      for col in 0..img.width {
        img.gray[row * img.width + col] = if (900..906).contains(&col) { 0 } else { 255 };
      }
    }
    let mut o = opts(11_025);
    o.phasing_lines = 0;
    let d = decode_samples(&modulate(&img, &o).unwrap(), 11_025, Params::default()).unwrap();
    assert_eq!(d.phasing_lines, None);
  }

  #[test]
  fn a_missed_phasing_pulse_does_not_break_the_chain() {
    // 噪声里个别脉冲会漏检。把第 10 条相位线的白脉冲抹掉（换成紧随其后的黑音）：
    // 链仍要连起来、条数按行序号算满 20，图像起点才不会落进相位线里。
    let img = chart(Ioc::I576, IMAGE_LINES);
    let mut audio = modulate(&img, &opts(11_025)).unwrap();
    let rate = 11_025.0_f64;
    let start = ((1.0 + 10.0 * 0.5) * rate) as usize;
    let len = (0.05 * 0.5 * rate) as usize;
    let black: Vec<f32> = audio[start + len..start + 2 * len].to_vec();
    audio[start..start + len].copy_from_slice(&black);
    let d = decode_samples(&audio, 11_025, Params::default()).unwrap();
    assert_eq!(d.phasing_lines, Some(20));
    assert_eq!(d.image.height, IMAGE_LINES);
    assert!(mean_abs_diff(&d.image.gray, &img.gray) < 4.0);
  }

  #[test]
  fn a_noisy_but_present_signal_still_decodes() {
    // 信噪比约 10 dB 的白噪声叠在副载波上：相位差解调会出尖峰，图变毛，但行周期、相位线与
    // 大块的黑白仍然对。要的是「噪声里有信号时不被当成噪声拒掉」。
    let img = chart(Ioc::I576, IMAGE_LINES);
    let mut audio = modulate(&img, &opts(11_025)).unwrap();
    let mut state = 99u64;
    for v in &mut audio {
      state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
      // 均匀分布 ±0.45 的噪声，功率 ≈ 0.0675；载波幅度 0.8，功率 0.32 → 约 6.8 dB。
      *v += ((state >> 33) as f32 / (1u64 << 31) as f32 * 2.0 - 1.0) * 0.45;
    }
    let d = decode_samples(&audio, 11_025, Params::default()).unwrap();
    // 噪声里个别脉冲会漏检（行序号照样累加，所以只差一两条）。
    assert!(
      matches!(d.phasing_lines, Some(n) if n >= 18),
      "{:?}",
      d.phasing_lines
    );
    assert!((d.lpm_measured - 120.0).abs() < 0.02, "{}", d.lpm_measured);
    assert!(region_mean(&d.image, 9..15, 600..880) > 200.0, "黑块");
    assert!(region_mean(&d.image, 23..27, 1130..1420) < 60.0, "白块");
  }

  #[test]
  fn noise_silence_and_bad_input_are_rejected() {
    let params = Params::default();
    assert_eq!(
      decode_samples(&vec![0.0; 80_000], 8000, params).err(),
      Some(Error::NoSync)
    );
    // 白噪声的相位差均匀铺在整个带宽里，只有四分之一落进 1200–2600 Hz。
    let mut state = 12345u64;
    let noise: Vec<f32> = (0..80_000)
      .map(|_| {
        state = state
          .wrapping_mul(6364136223846793005)
          .wrapping_add(1442695040888963407);
        ((state >> 33) as f32 / (1u64 << 31) as f32) * 2.0 - 1.0
      })
      .collect();
    assert_eq!(
      decode_samples(&noise, 8000, params).err(),
      Some(Error::NoSync)
    );
    assert_eq!(
      decode_samples(&[0.1; 100], 8000, params).err(),
      Some(Error::TooShort)
    );
    assert_eq!(
      decode_samples(&noise, 3000, params).err(),
      Some(Error::UnsupportedFormat)
    );
    assert_eq!(
      decode_samples(&noise, 0, params).err(),
      Some(Error::UnsupportedFormat)
    );
  }

  #[test]
  fn modulate_validates_the_image() {
    let mut img = chart(Ioc::I576, 4);
    img.gray.pop();
    assert_eq!(modulate(&img, &opts(8000)).err(), Some(Error::InvalidImage));
    let wrong_width = chart(Ioc::I288, 4);
    assert_eq!(
      modulate(&wrong_width, &opts(8000)).err(),
      Some(Error::InvalidImage)
    );
  }

  #[test]
  fn decode_reads_a_wav_file() {
    let img = chart(Ioc::I576, IMAGE_LINES);
    let wav = ham_web_wav::encode(&modulate(&img, &opts(8000)).unwrap(), 8000);
    let d = decode(&wav, Params::default()).unwrap();
    assert_eq!(d.image.height, IMAGE_LINES);
  }
}
