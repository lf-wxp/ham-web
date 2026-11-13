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

/// [`polish_base`] 默认在检测结果上下各核对多少个音调。
pub const POLISH_REACH: i32 = 2;

/// 长交织变体（64L / 128L）用的核对范围。
///
/// 它们的发端冲刷长达 1250 / 2500 个符号、数据位被交织开稀疏地摊在其中，整段录音只用到 5 个音调
/// （0 / 1 / 3 / 7 / 15），而且 90% 以上的符号是零位对应的最低音调。粗检测的窗口还没对齐、证据又稀疏，
/// 实测会偏 5–7 个音调（空闲音落在梳的最低一齿上，被左邻位的泄漏罚分，梳往下挪几个音调反而得分更高）。
/// 对齐之后每个窗口只含一个音调，核对的区分度高得多，范围放宽到装得下这种偏差。别对所有模式都放这么宽：
/// 有频漂的录音里梳本来就会在相邻音调间摇摆，范围一宽，核对就会被带到错误的梳上
/// （`tracks_linear_frequency_drift` 实测过）；按「每种最强音调权重相同」重新加权的做法也试过，
/// 同样会把有频漂的录音带偏，已放弃。
pub const POLISH_REACH_LONG: i32 = 8;

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
fn pick_windows(samples: &[f32], grid: &Grid, count: usize, jitter: bool) -> Vec<f64> {
  let live = live_indices(samples, grid);
  if live.is_empty() {
    return Vec::new();
  }
  let take = count.min(live.len());
  (0..take)
    .map(|k| {
      let frac = if jitter {
        unit_jitter(k as u64, take as u64, live.len() as u64)
      } else {
        0.0
      };
      let cell = (k as f64 + frac) * live.len() as f64 / take as f64;
      live[(cell as usize).min(live.len() - 1)] as f64 * grid.sym
    })
    .collect()
}

/// 名义网格（起点 0）上「有信号」的符号窗口序号，升序。
fn live_indices(samples: &[f32], grid: &Grid) -> Vec<usize> {
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
  energies
    .iter()
    .filter(|e| e.1 >= floor && e.1 > 0.0)
    .map(|e| e.0)
    .collect()
}

/// 等间距版（每格取格首）：符号对齐用 —— 要的是沿时间均匀，不怕与报文周期同相。
pub fn signal_windows(samples: &[f32], grid: &Grid, count: usize) -> Vec<f64> {
  pick_windows(samples, grid, count, false)
}

/// 中心频率探测用的窗口：每格内取一个**抖动**位置，而不是格首。
///
/// 报文严格周期重复时（信标、测试序列、复读机），等间距抽样会与周期混叠，窗口永远落在同一批符号上，
/// 某些音调从没被抽到。梳评分看不到的边缘音调等于没有证据，梳就可能偏一个音调
/// （MFSK128 把同一句报文连续重复 100 遍时实测过：自动检测选成 1375 而不是 1500 Hz，整段乱码）。
/// 抖动仍保持沿时间均匀，且对同一段录音是确定的（哈希只依赖格序号、格数与有信号窗口数），结果可复现。
/// 符号对齐（[`find_offset`]）**不用**它：对齐看的是整窗能量，与哪个音调无关，不受混叠影响，
/// 而换一批窗口会让大频漂录音的对齐点跟着挪一点，没有必要冒这个险。
pub fn probe_windows(samples: &[f32], grid: &Grid, count: usize) -> Vec<f64> {
  pick_windows(samples, grid, count, true)
}

/// `[0, 1)` 内的确定性伪随机数（SplitMix64 的终结步骤）：只依赖三个整数输入，不用全局状态。
fn unit_jitter(a: u64, b: u64, c: u64) -> f64 {
  let mut z = a
    .wrapping_mul(0x9E37_79B9_7F4A_7C15)
    .wrapping_add(b.wrapping_mul(0xBF58_476D_1CE4_E5B9))
    .wrapping_add(c.wrapping_mul(0x94D0_49BB_1331_11EB));
  z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
  z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
  z ^= z >> 31;
  // 取高 53 位，恰好填满 f64 尾数。
  (z >> 11) as f64 / (1u64 << 53) as f64
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
/// （[`probe_windows`]）。录音里没有足够的符号窗口返回 `None`。
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
  let wa = probe_windows(samples, grid, coarse_windows);
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
  let wb = probe_windows(samples, grid, more);
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
  let wc = probe_windows(samples, grid, fine_windows);
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

/// 载波判定：某个音调位的「底」至少是各音调位底的中位数的这么多倍。
const CARRIER_VS_FLOOR: f64 = 3.0;

/// 载波判定：它的底**至多**是「去掉它自己之后每个符号的最强音调幅度」中位数的这个倍数。
///
/// 去掉它自己之后剩下的最强音调就是数据音调（信号幅度的估计）；载波不比信号强时减载波能把被它淹掉的信号找回来，
/// 载波比信号强时**不减**：幅度域的减法会在载波位上留下 `√(2·载波·噪声)` 量级的交叉项噪声，载波越强越大，
/// 把信号淹掉，反而不如不减（实测载波幅度 2 倍于信号、噪声小时，不减 6/6、减了 0/6）。强载波交给接收机里
/// fldigi 沿用的窄带干扰软删除（`Receiver::soft_decode` 的 `cwi`）去处理。
/// 不能拿「每个符号的最强音调幅度」当分母：载波比信号强时那个最强音调就是载波自己，比值恒约等于 1，
/// 与「载波刚好等于信号」分不开。
const CARRIER_MAX_VS_SIGNAL: f64 = 1.0;

/// 载波判定：它的底至少是「每个符号最强音调幅度」中位数的这个比例 —— 干净的合成信号里
/// 各音调位的底都近乎 0，比较的分母也近乎 0，不加这一条会把任何一点点泄漏都当成载波。
const CARRIER_VS_PEAK: f64 = 0.15;

/// 至少这么多个符号才做载波检测（统计量要有意义）。
const CARRIER_MIN_SYMBOLS: usize = 32;

/// 一个音调位被判为最强的符号占比达到这个值，就算「它压住了所有数据」。
const CARRIER_DOMINANT: f64 = 0.995;

/// 「底」要靠至少这么多个「它没被判为最强」的符号来估。
const CARRIER_MIN_OFF_SAMPLES: usize = 8;

/// 窄带载波（CWI）抑制：把在整段录音里始终占着某个音调位的恒定载波从各符号的音调幅度里减掉，返回被判为载波的音调位个数。
///
/// 载波的特征是**不管发的是哪个音调，它那个位上都有同样的幅度**；数据音调只在轮到它的时候才有。
/// 所以每个音调位取「它没被判为最强的那些符号」里幅度的中位数当它的底 —— 数据音调位上是噪声底，载波位上是载波幅度。
/// 这样能把载波与**空闲音**分开：64L / 128L（以及任何前导 / 冲刷占大头的短报文）里最低音调占九成以上的符号，
/// 但剩下的那些符号里它是噪声底，不是载波。
/// 载波比信号还强时它在**每个**符号里都是最强，「没被判为最强」的符号集是空的；此时它压住了全部数据
/// （占比 ≥ [`CARRIER_DOMINANT`]），改取它整体幅度的低分位数 —— 但这种载波不减（见 [`CARRIER_MAX_VS_SIGNAL`]），
/// 这一支只是让判定不漏掉「比信号弱但把数据压住」的临界情形。
///
/// 载波与数据音调同位时两者相加，幅度大致是 `√(载波² + 信号²)`（相位随机），所以在**功率**域里减掉载波功率：
/// `m' = √max(0, m² − floor²)`，信号的那份留得下来。
///
/// 这比 fldigi 的做法强：fldigi 逐符号在线判定，只能「这个音调最近一直最强就当干扰、换次强的」，
/// 载波与信号等功率时它也解不出来（差分对拍见 `crates/thor/examples/noise_sweep.rs` 与 `crates/mfsk/reference/fldigi_web/README.md`）；离线有整段录音，
/// 恒定载波的幅度可以直接估出来再减掉。
///
/// **局限**：只处理载波不比信号强的情形（见 [`CARRIER_MAX_VS_SIGNAL`]）；载波比信号强得多、同时噪声又大时
/// 解不出来（fldigi 同样解不出）。自动检测中心频率那一步不做这个处理，有载波时请手填中心频率。
pub fn suppress_carriers(bins: &mut [f64], nt: usize) -> usize {
  subtract_carriers(bins, nt, &detect_carriers(bins, nt))
}

/// 把比信号还强的恒定载波所在的音调位整个抹成 0（不是减）：[`suppress_carriers`] 不碰它们（见 [`CARRIER_MAX_VS_SIGNAL`]），
/// 但硬判决的接收机（THOR）需要这一步 —— 载波在每个符号里都是最强，不抹掉，判出来的就是一串恒定音调。
/// 抹掉的代价是数据恰好落在这个音调位上的符号（约 1/18）丢了，交给前向纠错。返回抹掉的音调位个数。
pub fn erase_strong_carriers(bins: &mut [f64], nt: usize) -> usize {
  erase_carriers(bins, nt, &detect_carriers(bins, nt))
}

/// 一次检测、两种处理：先减掉不比信号强的载波，再抹掉比信号强的。MFSK / THOR 收端都是这样连着调用的，
/// 合成一个入口只扫一遍各音调位的统计量（`detect_carriers` 每个候选载波还要重算一次「去掉它之后的信号幅度」）。
/// 两类载波互不相交、处理互不影响，结果与分别调用 [`suppress_carriers`] / [`erase_strong_carriers`] 逐位相同。
/// 返回 `(减掉的个数, 抹掉的个数)`。
pub fn clean_carriers(bins: &mut [f64], nt: usize) -> (usize, usize) {
  let carriers = detect_carriers(bins, nt);
  (
    subtract_carriers(bins, nt, &carriers),
    erase_carriers(bins, nt, &carriers),
  )
}

fn subtract_carriers(bins: &mut [f64], nt: usize, carriers: &[Carrier]) -> usize {
  let mut subtracted = 0;
  for c in carriers.iter().filter(|c| !c.stronger_than_signal) {
    subtracted += 1;
    for m in bins.chunks_exact_mut(nt).map(|b| &mut b[c.tone]) {
      *m = (*m * *m - c.floor * c.floor).max(0.0).sqrt();
    }
  }
  subtracted
}

fn erase_carriers(bins: &mut [f64], nt: usize, carriers: &[Carrier]) -> usize {
  let mut erased = 0;
  for c in carriers.iter().filter(|c| c.stronger_than_signal) {
    erased += 1;
    for m in bins.chunks_exact_mut(nt).map(|b| &mut b[c.tone]) {
      *m = 0.0;
    }
  }
  erased
}

/// 判出来的一个载波。
struct Carrier {
  tone: usize,
  /// 它在各符号里的幅度（「底」）。
  floor: f64,
  /// 它比数据音调还强。
  stronger_than_signal: bool,
}

/// 找出恒定载波：判据见 [`suppress_carriers`] 的文档。
fn detect_carriers(bins: &[f64], nt: usize) -> Vec<Carrier> {
  let windows = bins.len() / nt;
  if nt < 2 || windows < CARRIER_MIN_SYMBOLS {
    return Vec::new();
  }
  // 空切片没有分位数：返回 `None` 让调用方走「这批数据没有载波」那一支，而不是在 usize 下溢后越界。
  let quantile = |mut v: Vec<f64>, q: f64| -> Option<f64> {
    v.sort_by(f64::total_cmp);
    let at = ((v.len().checked_sub(1)?) as f64 * q) as usize;
    v.get(at).copied()
  };
  let argmax: Vec<usize> = bins
    .chunks_exact(nt)
    .map(|b| {
      b.iter()
        .enumerate()
        .fold(
          (0, f64::NEG_INFINITY),
          |m, (i, &x)| if x > m.1 { (i, x) } else { m },
        )
        .0
    })
    .collect();
  let floors: Vec<Option<f64>> = (0..nt)
    .map(|k| {
      let off: Vec<f64> = (0..windows)
        .filter(|&w| argmax[w] != k)
        .map(|w| bins[w * nt + k])
        .collect();
      if off.len() >= CARRIER_MIN_OFF_SAMPLES {
        return quantile(off, 0.5);
      }
      let on = windows - off.len();
      (on as f64 >= CARRIER_DOMINANT * windows as f64)
        .then(|| {
          quantile(
            (0..windows).map(|w| bins[w * nt + k]).collect(),
            CARRIER_DOMINANT_PERCENTILE,
          )
        })
        .flatten()
    })
    .collect();
  let known: Vec<f64> = floors.iter().flatten().copied().collect();
  let Some(reference) = quantile(known, 0.5) else {
    return Vec::new();
  };
  let Some(peak) = quantile(
    bins
      .chunks_exact(nt)
      .map(|b| b.iter().copied().fold(0.0, f64::max))
      .collect(),
    0.5,
  ) else {
    return Vec::new();
  };
  // 去掉第 `k` 个音调位之后，每个符号最强音调幅度的中位数。
  let signal_without = |k: usize| -> Option<f64> {
    quantile(
      bins
        .chunks_exact(nt)
        .map(|b| {
          b.iter()
            .enumerate()
            .filter(|&(j, _)| j != k)
            .fold(0.0, |m, (_, &x)| f64::max(m, x))
        })
        .collect(),
      0.5,
    )
  };
  floors
    .iter()
    .enumerate()
    .filter_map(|(tone, floor)| {
      let floor = (*floor)?;
      if floor <= CARRIER_VS_FLOOR * reference.max(1e-12) || floor <= CARRIER_VS_PEAK * peak {
        return None;
      }
      Some(Carrier {
        tone,
        floor,
        stronger_than_signal: floor > CARRIER_MAX_VS_SIGNAL * signal_without(tone)?,
      })
    })
    .collect()
}

/// 压住全部数据的载波取整体幅度的这个分位数当底（个别符号里相位相消会把它压低，最小值不稳）。
const CARRIER_DOMINANT_PERCENTILE: f64 = 0.10;

/// 对齐之后再核对一遍中心频率：只在 `base` 上下各 2 个音调间隔内挑。
///
/// [`detect_base`] 取的窗口还没对齐（起点是名义位置），常常跨在两个符号上，证据被冲淡 ——
/// 噪声稍大一点，「梳」就可能差一个音调（实测过：THOR8 加一点点噪声就偏了）。符号对齐之后，每个窗口里
/// 只有一个音调，评分的区分度高得多，所以把候选梳在**对齐窗口**上重新评一次分。
#[must_use]
pub fn polish_base(samples: &[f32], grid: &Grid, base: f64, offset: f64, reach: i32) -> f64 {
  let (nt, sp, rate) = (grid.ntones, grid.spacing, grid.rate);
  let count = (PROBE_BUDGET / (5 * (nt + 2) * grid.win)).clamp(16, 192);
  let slices: Vec<&[f32]> = probe_windows(samples, grid, count)
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
  // 0 放最前：并列时留原值；再按离原值由近到远排。
  for k in (0..=reach).flat_map(|d| if d == 0 { vec![0] } else { vec![-d, d] }) {
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

/// 前导 / 冲刷里同一个音调至少连续这么多个符号才算「空闲音」（MFSK8 的前导只有 14 个符号；随机数据里
/// 连续 10 个符号落在同一个音调的概率可以忽略）。
const IDLE_MIN_RUN: usize = 10;

/// 在发射的开头 / 结尾各看这么多个有信号的符号。
const IDLE_SPAN: usize = 64;

/// 空闲音在检测结果上下各找这么多个音调。
const IDLE_REACH: i32 = 12;

/// 用 MFSK 的**前导 / 冲刷**把梳定下来：返回「空闲音比 `base` 高几个音调」，没有可信的空闲音就是 `None`。
///
/// 为什么需要它：[`comb_score`] 只惩罚「梳外邻位比梳内更强」，梳整体往低处挪几个音调时，只要被挪出去的高音调
/// 在报文里没出现过就**没有任何惩罚**；短报文只用到少数几个音调，检测因此常偏低 2–3 个音调
/// （MFSK8 / MFSK4 的 `CQ DE BG1AAA K ` 在 35 个中心频率里错 6 / 5 个，`CQ ` 全错），整段乱码。
/// 而 MFSK 的发射总是以**最低音调**（符号 0）的前导开头、以冲刷收尾，是一段恒定音，位置不依赖报文内容。
/// 所以在发射开头的前 [`IDLE_SPAN`] 个符号里找「连续 [`IDLE_MIN_RUN`] 个以上同一个最强音调」，它就是音调 0
/// （`reverse` 时是最高音调）。
///
/// 只认「起点在最前面几个符号之内、且没占满整个观察窗」的恒定段：恒定载波会让每个窗口的最强音调都是它，
/// 占满整个窗口，不算。开头找不到再看结尾；两头给出的结果不一致就不信。
/// 录音从发射中途开始、又没有冲刷时没有这个锚，调用方沿用 [`polish_base`]。
#[must_use]
pub fn idle_tone_shift(
  samples: &[f32],
  grid: &Grid,
  base: f64,
  offset: f64,
  reverse: bool,
) -> Option<i32> {
  let live = live_indices(samples, grid);
  if live.len() < 2 * IDLE_MIN_RUN {
    return None;
  }
  let nyq = f64::from(grid.rate) / 2.0;
  let (nt, sp) = (grid.ntones as i32, grid.spacing);
  let ks: Vec<i32> = (-IDLE_REACH..nt + IDLE_REACH)
    .filter(|&k| {
      let f = base + f64::from(k) * sp;
      f > 0.0 && f < nyq
    })
    .collect();
  let strongest = |i: usize| -> Option<i32> {
    let start = (offset + i as f64 * grid.sym).round() as usize;
    let w = samples.get(start..start + grid.win)?;
    ks.iter()
      .map(|&k| (k, tone_power(w, grid.rate, base + f64::from(k) * sp)))
      .max_by(|a, b| a.1.total_cmp(&b.1))
      .map(|(k, _)| k)
  };
  let head: Vec<Option<i32>> = live.iter().take(IDLE_SPAN).map(|&i| strongest(i)).collect();
  let tail: Vec<Option<i32>> = live
    .iter()
    .rev()
    .take(IDLE_SPAN)
    .map(|&i| strongest(i))
    .collect();
  // 从观察窗的一端数起的恒定段：起点在最前面 4 个符号内、长度够、但没占满整个窗口。
  let run = |seq: &[Option<i32>]| -> Option<i32> {
    for start in 0..4.min(seq.len()) {
      let Some(k) = seq[start] else {
        continue;
      };
      let len = seq[start..].iter().take_while(|&&x| x == Some(k)).count();
      if len >= IDLE_MIN_RUN && start + len < seq.len() {
        return Some(k);
      }
    }
    None
  };
  let shift = |k: i32| if reverse { k - (nt - 1) } else { k };
  match (run(&head), run(&tail)) {
    (Some(a), Some(b)) if a != b => None,
    (Some(k), _) | (None, Some(k)) => Some(shift(k)),
    (None, None) => None,
  }
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

#[cfg(test)]
mod tests {
  use super::*;

  /// 恒包络的单音录音：每个符号窗口能量几乎相同，所以全部都算「有信号」。
  fn steady_tone(grid: &Grid, symbols: usize) -> Vec<f32> {
    let n = (symbols as f64 * grid.sym) as usize + grid.win;
    (0..n)
      .map(|i| (2.0 * std::f64::consts::PI * 1000.0 * i as f64 / f64::from(grid.rate)).sin() as f32)
      .collect()
  }

  fn grid() -> Grid {
    Grid::new(8000, 31.25, 31.25, 16).expect("合法几何")
  }

  /// 每个符号一个音调（相位连续）：`tones[i]` 是第 `i` 个符号相对 `low` 的音调序号。
  fn keyed(grid: &Grid, low: f64, tones: &[usize]) -> Vec<f32> {
    let mut phase = 0.0f64;
    let mut out = Vec::new();
    for &t in tones {
      let step = std::f64::consts::TAU * (low + t as f64 * grid.spacing) / f64::from(grid.rate);
      for _ in 0..grid.win {
        out.push(phase.sin() as f32);
        phase += step;
      }
    }
    out
  }

  /// 前导 / 冲刷：开头与结尾各一段恒定音，中间是只用到中段几个音调的「报文」。
  fn framed(idle: usize, data: &[usize]) -> Vec<usize> {
    let mut v = vec![idle; 24];
    v.extend_from_slice(data);
    v.extend(vec![idle; 24]);
    v
  }

  #[test]
  fn the_idle_tone_pins_the_comb_even_when_the_message_uses_few_tones() {
    let g = grid();
    // 真实梳的最低音调 1000 Hz；检测给出的起点偏低 3 个音调（评分对这种偏差没有惩罚）。
    let low = 1000.0;
    let audio = keyed(
      &g,
      low,
      &framed(0, &[5, 6, 7, 6, 5, 8, 7, 6, 9, 5, 6, 7, 8, 6, 5, 7, 6, 8]),
    );
    let detected = low - 3.0 * g.spacing;
    assert_eq!(idle_tone_shift(&audio, &g, detected, 0.0, false), Some(3));
    // 边带反转：空闲音在梳的最高一齿（第 15 齿），起点同样偏低 3 个音调时应往上挪 3。
    let flipped_base = low - 18.0 * g.spacing;
    assert_eq!(
      idle_tone_shift(&audio, &g, flipped_base, 0.0, true),
      Some(3)
    );
  }

  #[test]
  fn no_idle_tone_no_anchor() {
    let g = grid();
    // 从发射中途开始：开头没有恒定段；最后一段又太短。
    let mid: Vec<usize> = (0..60).map(|i| (i * 7 + i / 3) % 16).collect();
    assert_eq!(
      idle_tone_shift(&keyed(&g, 1000.0, &mid), &g, 1000.0, 0.0, false),
      None
    );
    // 恒定载波占满整个观察窗：不是前导。
    assert_eq!(
      idle_tone_shift(&keyed(&g, 1000.0, &[4; 80]), &g, 1000.0, 0.0, false),
      None
    );
    // 太短的录音。
    assert_eq!(
      idle_tone_shift(&keyed(&g, 1000.0, &[0; 12]), &g, 1000.0, 0.0, false),
      None
    );
  }

  #[test]
  fn probe_windows_are_deterministic() {
    let g = grid();
    let audio = steady_tone(&g, 100);
    assert_eq!(
      probe_windows(&audio, &g, 10),
      probe_windows(&audio, &g, 10),
      "同一段录音两次探测必须取到同一批窗口"
    );
  }

  /// 每个窗口落在自己那一格里（沿时间仍然均匀、升序），而且至少有一个不在格首 ——
  /// 否则「抖动」就是空话，与报文周期混叠的问题原样保留。
  #[test]
  fn probe_windows_stay_in_their_cell_and_actually_move() {
    let g = grid();
    let audio = steady_tone(&g, 100);
    let live = 100.0;
    let count = 10;
    let probe = probe_windows(&audio, &g, count);
    let even = signal_windows(&audio, &g, count);
    assert_eq!(probe.len(), count);
    assert_eq!(even.len(), count);
    for (k, &start) in probe.iter().enumerate() {
      let sym = (start / g.sym).round();
      let lo = (k as f64 * live / count as f64).floor();
      let hi = ((k + 1) as f64 * live / count as f64).ceil();
      assert!(
        (lo..hi).contains(&sym),
        "第 {k} 个窗口在符号 {sym}，应在 [{lo}, {hi})"
      );
    }
    assert!(probe.windows(2).all(|w| w[0] <= w[1]), "应升序：{probe:?}");
    assert_ne!(probe, even, "抖动没有生效：与等间距抽样完全相同");
  }

  /// 要的窗口数不少于有信号的窗口数时，每格只剩一个窗口可选，抖动不能越界或丢窗口：
  /// 结果必须与等间距版一致（下标上界的钳位在这里生效）。
  #[test]
  fn probe_windows_with_few_live_windows_match_the_even_pick() {
    let g = grid();
    let audio = steady_tone(&g, 12);
    let probe = probe_windows(&audio, &g, 500);
    assert!(!probe.is_empty());
    assert_eq!(probe, signal_windows(&audio, &g, 500));
    let limit = audio.len() as f64;
    assert!(probe.iter().all(|&s| s >= 0.0 && s + g.win as f64 <= limit));
  }

  /// 固定种子的 `[0, 1)` 伪随机数。
  fn rand(state: &mut u32) -> f64 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    f64::from(*state >> 8) / 16_777_216.0
  }

  /// 每个符号一个数据音调（`picks[w]`）幅度 1，其余是 0.05 量级的噪声底。
  fn synthetic_bins(nt: usize, picks: &[usize]) -> Vec<f64> {
    let mut seed = 7;
    let mut bins = Vec::with_capacity(picks.len() * nt);
    for &pick in picks {
      for k in 0..nt {
        let noise = 0.05 * rand(&mut seed);
        bins.push(if k == pick { 1.0 } else { noise });
      }
    }
    bins
  }

  #[test]
  fn clean_bins_are_left_bit_for_bit() {
    let mut seed = 99;
    let picks: Vec<usize> = (0..400)
      .map(|_| (rand(&mut seed) * 16.0) as usize)
      .collect();
    let mut bins = synthetic_bins(16, &picks);
    let before = bins.clone();
    assert_eq!(suppress_carriers(&mut bins, 16), 0);
    assert_eq!(bins, before, "没有载波时不许碰任何一个数");
  }

  /// 64L / 128L 的最低音调占九成以上的符号：它是数据（空闲音），不是载波，不能被减掉。
  #[test]
  fn an_idle_tone_that_dominates_is_not_a_carrier() {
    let mut seed = 5;
    let picks: Vec<usize> = (0..600)
      .map(|_| {
        if rand(&mut seed) < 0.92 {
          0
        } else {
          (rand(&mut seed) * 16.0) as usize
        }
      })
      .collect();
    let mut bins = synthetic_bins(16, &picks);
    let before = bins.clone();
    assert_eq!(suppress_carriers(&mut bins, 16), 0);
    assert_eq!(bins, before);
  }

  /// 一个比数据略小的恒定载波（幅度 0.8）占着第 5 个音调位：被找出来，减掉之后那一位回到噪声底，数据位保持。
  #[test]
  fn a_constant_carrier_is_found_and_removed() {
    let mut seed = 3;
    let nt = 16;
    let picks: Vec<usize> = (0..400)
      .map(|_| (rand(&mut seed) * 16.0) as usize)
      .collect();
    let mut bins = synthetic_bins(nt, &picks);
    for w in 0..400 {
      // 载波与数据音调同位时幅度相加后相位随机，这里取功率相加的期望值。
      let k = 5;
      let m = bins[w * nt + k];
      bins[w * nt + k] = (m * m + 0.64).sqrt();
    }
    assert_eq!(suppress_carriers(&mut bins, nt), 1);
    for w in 0..400 {
      let at5 = bins[w * nt + 5];
      if picks[w] == 5 {
        assert!(at5 > 0.9, "数据落在载波位上时它的那份要留下来：{at5}");
      } else {
        assert!(at5 < 0.2, "载波位上没有数据时应回到噪声底：{at5}");
      }
    }
  }

  /// 载波比数据还强（每个符号里它都是最强）：「没被判为最强」的符号集是空的，要靠整体幅度的低分位数估；
  /// 这种载波**不减**（幅度域的减法会留下交叉项噪声），而是整个音调位抹成 0。
  #[test]
  fn a_carrier_stronger_than_the_data_is_erased_not_subtracted() {
    let mut seed = 11;
    let nt = 16;
    let picks: Vec<usize> = (0..400)
      .map(|_| (rand(&mut seed) * 16.0) as usize)
      .collect();
    let mut bins = synthetic_bins(nt, &picks);
    for w in 0..400 {
      let m = bins[w * nt + 9];
      bins[w * nt + 9] = (m * m + 4.0).sqrt(); // 载波幅度 2，是数据的两倍
    }
    let before = bins.clone();
    assert_eq!(suppress_carriers(&mut bins, nt), 0, "太强的载波不减");
    assert_eq!(bins, before);
    assert_eq!(erase_strong_carriers(&mut bins, nt), 1);
    for w in 0..400 {
      assert_eq!(bins[w * nt + 9], 0.0);
      for k in (0..nt).filter(|&k| k != 9) {
        assert_eq!(bins[w * nt + k], before[w * nt + k], "别的音调位不许动");
      }
    }
  }

  #[test]
  fn clean_bins_are_never_erased_either() {
    let mut seed = 21;
    let picks: Vec<usize> = (0..400)
      .map(|_| (rand(&mut seed) * 16.0) as usize)
      .collect();
    let mut bins = synthetic_bins(16, &picks);
    let before = bins.clone();
    assert_eq!(erase_strong_carriers(&mut bins, 16), 0);
    assert_eq!(bins, before);
  }

  #[test]
  fn too_few_symbols_are_never_touched() {
    let mut bins = synthetic_bins(16, &[3; 10]);
    for w in 0..10 {
      bins[w * 16 + 9] = 5.0;
    }
    let before = bins.clone();
    assert_eq!(suppress_carriers(&mut bins, 16), 0);
    assert_eq!(bins, before);
  }

  #[test]
  fn silence_has_no_windows() {
    let g = grid();
    assert!(probe_windows(&vec![0.0; 8000], &g, 8).is_empty());
    assert!(probe_windows(&[], &g, 8).is_empty());
  }
}
