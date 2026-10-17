//! TDR（时域反射）故障定位：对 S11 扫频做逆 FFT，映射到距离域。
//!
//! 原理：传输线上距参考面 `d` 处的一处失配，其反射系数随频率的相位是
//! `Γ(f) = Γ0 · e^(−j·4π·f·d/v)`（`v = vf·c` 是线上相速，`4π` 是往返双程）。
//! 对扫频采样做逆 FFT，|回波| 在 `t = 2d/v` 处出现峰 —— 把时间轴换成距离轴，
//! 峰的位置就是失配点离参考面的距离。这正是 NanoVNA 的「TDR」功能做的事。
//!
//! 两个由数据本身决定的量：
//! - **距离分辨率** ≈ `v/(2·BW)`（瑞利口径，`BW = f_stop − f_start`；加 Hann 窗后
//!   主瓣更宽、实际分辨力略差）；
//! - **最大不模糊距离** = `v/(2·Δf)`（`Δf` 是频点间隔；超出会绕回来，界面须提示）。
//!
//! 加 Hann 窗压低强反射的旁瓣（−31 dB 首旁瓣），避免把旁瓣当成第二个反射点；
//! 幅度归一化到最强反射为 0 dB，峰检测只在 −30 dB 以上、且彼此至少隔一个
//! 分辨率的局部极大里选，最多报 [`MAX_PEAKS`] 个（超出时保留最强的）。
//!
//! 「等间隔」的判定容差按间隔的 1% 取（`GAP_TOLERANCE_REL`），而不是浮点精度：频率文本常被
//! 量化到 1kHz，用 `1e-6` 级容差会把等间隔的真实文件判成不可用。
//!
//! # 已知边界
//!
//! 参考面本身（`d ≈ 0`，接插件 / 校准面）的失配落在距离轴的**第 0 个 bin**。它参与 0 dB 归一化，
//! 但按定义不会被报成「反射点」（否则每个文件都会多出一条 0 m 的必然结论）。代价是：近端
//! 强失配会把远端弱反射的相对幅度整体压低，可能落到 −30 dB 门限以下而漏报 —— 读数时先看
//! 0 m 附近是否本身就很大。

use crate::fft::fft_in_place;
use crate::touchstone::Sweep;
use crate::waveguide::C;

/// TDR 计算失败的原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TdrError {
  /// 频点不足（至少需要 3 个点才能有扫频带宽与频点间隔）。
  TooFewPoints,
  /// 频点太多（上限 [`MAX_POINTS`]）：IFFT 与逐点曲线都在主线程上跑，超大文件会卡死页面。
  TooManyPoints,
  /// 频点间隔不均匀（从第 `index` 个点起，1 起）：IFFT 只取一个 Δf，不均匀时距离轴是错的。
  NotUniform(usize),
  /// 速率因子非法（要求 `0 < vf ≤ 1`）：否则整条距离轴都是 NaN，看上去像「没有反射」。
  BadVelocityFactor,
}

/// 一次 TDR 接受的最大频点数。
///
/// 计算全在主线程：`n_fft` 跟着点数走（见 [`tdr`] 的注释），距离与幅度各是一条 `Vec<f64>`，
/// 1 万点已经要 16384 点的 FFT 与近 500 KB 的曲线。VNA 导出动辄十万点，不设上限的话
/// 一个几 MB 的 `.s1p` 就能把标签页冻住 —— 上限取得比常见导出（几万点）宽，仍留足余量。
pub const MAX_POINTS: usize = 65_536;

/// 一个检测到的反射点。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TdrPeak {
  /// 距参考面的距离（m）。
  pub distance_m: f64,
  /// 相对最强反射的幅度（dB，最强为 0）。
  pub magnitude_db: f64,
}

/// 一次 TDR 结果。
#[derive(Debug, Clone, PartialEq)]
pub struct Tdr {
  /// 距离轴（m，从 0 到略小于 [`Tdr::max_distance_m`]）。
  pub distances_m: Vec<f64>,
  /// 归一化回波幅度（dB，最强为 0；理想匹配时全是 `-∞`）。
  pub magnitude_db: Vec<f64>,
  /// 距离分辨率（m，瑞利口径）。
  pub resolution_m: f64,
  /// 最大不模糊距离（m）。
  pub max_distance_m: f64,
  /// 检测到的反射点（按距离升序，最多 [`MAX_PEAKS`] 个 —— 超出时保留最强的）。
  pub peaks: Vec<TdrPeak>,
}

/// 峰值检测门限（相对最强反射的 dB）。界面上的门限虚线也用这个值，避免两处各写一份。
pub const PEAK_THRESHOLD_DB: f64 = -30.0;

/// 一次 TDR 最多报出的反射点数。
///
/// 峰值列表要逐条渲染成 DOM（列表项与图上的标记）。长电缆多处失配、再叠上参考面回波的旁瓣时，
/// 上面那套贪心选取理论上能挑出上百个「过门限」的峰：列表读不完，大文件的 DOM 也会膨胀。
/// 只留最强的 [`MAX_PEAKS`] 个 —— 其余本来就在门限附近，读数意义有限。
pub const MAX_PEAKS: usize = 16;

/// 频点间隔允许的相对偏差。
///
/// 不按浮点精度定容差：文本导出的频率常只保留 6–9 位有效数字（900MHz 处量化步长可达 1kHz），
/// 用 `1e-6` 级的容差会把**等间隔的真实文件**判成不均匀、TDR 直接不可用。1% 的间隔偏差只会让
/// 距离轴跟着偏 1%，对故障定位没有影响。
const GAP_TOLERANCE_REL: f64 = 0.01;

/// 逆 FFT：`IFFT(x) = conj(FFT(conj(x))) / n`，复用 [`fft_in_place`]。
fn ifft(re: &mut [f32], im: &mut [f32]) {
  for v in im.iter_mut() {
    *v = -*v;
  }
  fft_in_place(re, im);
  for v in im.iter_mut() {
    *v = -*v;
  }
  let scale = 1.0 / re.len() as f32;
  for v in re.iter_mut() {
    *v *= scale;
  }
  for v in im.iter_mut() {
    *v *= scale;
  }
}

/// Hann 窗（`n` 个采样里的第 `i` 个，0 起）。
fn hann(i: usize, n: usize) -> f64 {
  let x = i as f64 / (n - 1).max(1) as f64;
  0.5 - 0.5 * (2.0 * std::f64::consts::PI * x).cos()
}

/// 在归一化 dB 曲线上找反射峰。
///
/// 先取「严格高于左邻、不低于右邻」的局部极大（高于门限），再按幅度从强到弱
/// 贪心挑选，保证相邻峰至少隔一个 `min_sep_m`（分辨率）—— 这能滤掉强峰的旁瓣。
///
/// 从第 1 个 bin 起扫：第 0 个 bin 是参考面本身（`d = 0`）的回波，永远存在且已被归一化成
/// 0 dB，把它当「反射点」只会给每个文件加一条无信息量的结论（见模块文档「已知边界」）。
fn find_peaks(distances_m: &[f64], magnitude_db: &[f64], min_sep_m: f64) -> Vec<TdrPeak> {
  let n = magnitude_db.len();
  let mut cand: Vec<(usize, f64)> = Vec::new();
  for i in 1..n.saturating_sub(1) {
    let m = magnitude_db[i];
    if m > magnitude_db[i - 1] && m >= magnitude_db[i + 1] && m > PEAK_THRESHOLD_DB {
      cand.push((i, m));
    }
  }
  cand.sort_by(|a, b| b.1.total_cmp(&a.1));
  let mut picked: Vec<TdrPeak> = Vec::new();
  for (i, m) in cand {
    let d = distances_m[i];
    if picked.iter().all(|p| (p.distance_m - d).abs() >= min_sep_m) {
      picked.push(TdrPeak {
        distance_m: d,
        magnitude_db: m,
      });
    }
  }
  // 先截断再排序：`cand` 是幅度降序（stable），所以留下的正是最强的那些，而不是最近的。
  picked.truncate(MAX_PEAKS);
  picked.sort_by(|a, b| a.distance_m.total_cmp(&b.distance_m));
  picked
}

/// 绘图用的一个采样点。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotPoint {
  /// 距参考面的距离（m）。
  pub distance_m: f64,
  /// 归一化回波幅度（dB，有限值）。
  pub magnitude_db: f64,
  /// 是否要与上一个点断开（折线的新起点）：首点，以及前一段全是 `-∞` 时为 `true`。
  pub segment_start: bool,
}

impl Tdr {
  /// 抽稀成绘图用的点列，最多约 `2 × columns` 个点。
  ///
  /// 距离轴的点数跟着 `n_fft` 走（1 万点文件就有 16384 个 bin），而画布只有几百像素宽：
  /// 逐点画既浪费属性字符串空间，也让每次输入都重建一份几百 KB 的 `d`。这里按列分桶，
  /// 每桶同时保留**谷与峰**，强反射的尖顶不会被抽稀抹平。
  ///
  /// 幅度为 `-∞` 的 bin（理想匹配时全轴都是）会被丢掉；中间出现空洞时下一段以
  /// `segment_start = true` 起笔，避免在图上凭空连出一条跨越空洞的直线。
  #[must_use]
  pub fn plot_points(&self, columns: usize) -> Vec<PlotPoint> {
    let n = self.distances_m.len();
    if n == 0 {
      return Vec::new();
    }
    let columns = columns.max(1);
    let bucket = n.div_ceil(columns).max(1);
    let mut out = Vec::with_capacity(columns * 2);
    let mut previous_had_points = false;
    for start in (0..n).step_by(bucket) {
      let end = (start + bucket).min(n);
      let mut low: Option<(f64, f64)> = None;
      let mut high: Option<(f64, f64)> = None;
      for i in start..end {
        let db = self.magnitude_db[i];
        if !db.is_finite() {
          continue;
        }
        let point = (self.distances_m[i], db);
        if low.is_none_or(|(_, v)| db < v) {
          low = Some(point);
        }
        if high.is_none_or(|(_, v)| db > v) {
          high = Some(point);
        }
      }
      let Some(low) = low else {
        // 整桶都是 -∞：下一段要重新 Move。
        previous_had_points = false;
        continue;
      };
      // 谷与峰按距离先后次序输出，折线才不会自己打结。
      let high = high.unwrap_or(low);
      let (first, second) = if low.0 < high.0 {
        (low, high)
      } else {
        (high, low)
      };
      out.push(PlotPoint {
        distance_m: first.0,
        magnitude_db: first.1,
        segment_start: !previous_had_points,
      });
      if second.0 != first.0 {
        out.push(PlotPoint {
          distance_m: second.0,
          magnitude_db: second.1,
          segment_start: false,
        });
      }
      previous_had_points = true;
    }
    out
  }
}

/// 对一次扫频做 TDR。
///
/// `velocity_factor` 是传输线的速率因子（`0 < vf ≤ 1`，如聚乙烯同轴 0.66、发泡 0.82、
/// 双线 0.88）；越界返回 [`TdrError::BadVelocityFactor`] —— 界面也会校验，但内核不能靠调用方。
pub fn tdr(sweep: &Sweep, velocity_factor: f64) -> Result<Tdr, TdrError> {
  let pts = &sweep.points;
  if pts.len() < 3 {
    return Err(TdrError::TooFewPoints);
  }
  if pts.len() > MAX_POINTS {
    return Err(TdrError::TooManyPoints);
  }
  // NaN 与任何值比较都是 false，`!(…)` 的写法把 NaN 一起挡在门外。
  if !(velocity_factor.is_finite() && velocity_factor > 0.0 && velocity_factor <= 1.0) {
    return Err(TdrError::BadVelocityFactor);
  }
  let n = pts.len();
  // 参考间隔取全部 gap 的**中位数**，而不是首个 gap：文本导出的频率常被量化，首个 gap 本身可能
  // 就是抖动的那一个；中位数对单个异常频点也稳（它不会跟着坏点跑）。
  let mut gaps: Vec<f64> = pts.windows(2).map(|w| w[1].f_hz - w[0].f_hz).collect();
  gaps.sort_by(f64::total_cmp);
  let df = gaps[gaps.len() / 2];
  // 显式查有限性：频率里有 NaN 时中位数也是 NaN，而 `<= 0.0` 对 NaN 恒为 false。
  if !df.is_finite() || df <= 0.0 {
    return Err(TdrError::NotUniform(1));
  }
  let tol = df * GAP_TOLERANCE_REL;
  for i in 1..n {
    if (pts[i].f_hz - pts[i - 1].f_hz - df).abs() > tol {
      // 报「偏离等间隔网格的第一个点」（1 起），与 `touchstone::ParseError` 的行号口径一致。
      return Err(TdrError::NotUniform(i + 1));
    }
  }

  // 只保下界不封上界：上界一封，`n > 8192` 的文件会让下面的填充循环越过缓冲区
  //（1 万点的 VNA 导出很常见）。FFT 长度取 2 的幂只为零填到整数 bin。
  let n_fft = n.next_power_of_two().max(1024);
  // Γ 采样 × Hann 窗，零填到 n_fft（填零 = 频域插值，距离轴更平滑）。
  let mut re = vec![0f32; n_fft];
  let mut im = vec![0f32; n_fft];
  for i in 0..n {
    let w = hann(i, n) as f32;
    re[i] = pts[i].re as f32 * w;
    im[i] = pts[i].im as f32 * w;
  }
  ifft(&mut re, &mut im);

  let v = velocity_factor * C;
  let bw = pts[n - 1].f_hz - pts[0].f_hz;
  let resolution_m = v / (2.0 * bw);
  let max_distance_m = v / (2.0 * df);

  let mut magnitude_db = vec![f64::NEG_INFINITY; n_fft];
  let mut distances_m = vec![0.0; n_fft];
  let peak_mag = re
    .iter()
    .zip(&im)
    .map(|(&a, &b)| (a as f64).hypot(b as f64))
    .fold(0.0f64, f64::max);
  for i in 0..n_fft {
    distances_m[i] = i as f64 / n_fft as f64 * max_distance_m;
    if peak_mag > 0.0 {
      let m = (re[i] as f64).hypot(im[i] as f64);
      magnitude_db[i] = 20.0 * (m / peak_mag).log10();
    }
  }
  let peaks = find_peaks(&distances_m, &magnitude_db, resolution_m);

  Ok(Tdr {
    distances_m,
    magnitude_db,
    resolution_m,
    max_distance_m,
    peaks,
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::touchstone::S11Point;

  /// 由若干 `(距离 m, |Γ0|)` 合成一份扫频：`Γ(f) = Σ Γ0·e^(−j·4π·f·d/v)`，忽略多次反射。
  ///
  /// 这是「已知失配模型」——先拿它反推，断言峰值距离回到输入，再信真实数据。
  fn synth(vf: f64, f0_hz: f64, f1_hz: f64, n: usize, refls: &[(f64, f64)]) -> Sweep {
    let v = vf * C;
    let df = (f1_hz - f0_hz) / (n - 1) as f64;
    let points = (0..n)
      .map(|i| {
        let f = f0_hz + i as f64 * df;
        let (mut re, mut im) = (0.0, 0.0);
        for &(d, g0) in refls {
          let phase = -4.0 * std::f64::consts::PI * f * d / v;
          re += g0 * phase.cos();
          im += g0 * phase.sin();
        }
        S11Point { f_hz: f, re, im }
      })
      .collect();
    Sweep { z0: 50.0, points }
  }

  /// 最强峰的距离。
  fn strongest(peaks: &[TdrPeak]) -> Option<f64> {
    peaks
      .iter()
      .max_by(|a, b| a.magnitude_db.total_cmp(&b.magnitude_db))
      .map(|p| p.distance_m)
  }

  #[test]
  fn single_reflection_lands_at_its_distance() {
    // 10 m 处 Γ0 = 1/3（100 Ω 负载挂在 50 Ω 线上），1–900 MHz、512 点。
    let s = synth(0.66, 1e6, 900e6, 512, &[(10.0, 1.0 / 3.0)]);
    let t = tdr(&s, 0.66).unwrap();
    let d = strongest(&t.peaks).expect("应至少有一个反射峰");
    assert!((d - 10.0).abs() < 0.5, "最强峰在 {d} m，应为 10 m");
    // 最强峰归一化后是 0 dB。
    let max_db = t
      .peaks
      .iter()
      .map(|p| p.magnitude_db)
      .fold(f64::NEG_INFINITY, f64::max);
    assert!((max_db - 0.0).abs() < 1e-6);
  }

  #[test]
  fn two_reflections_yield_two_peaks() {
    // 5 m 处 Γ=0.2、20 m 处 Γ=0.3：两峰相距 15 m，远大于分辨率。
    let s = synth(0.66, 1e6, 900e6, 512, &[(5.0, 0.2), (20.0, 0.3)]);
    let t = tdr(&s, 0.66).unwrap();
    assert!(t.peaks.len() >= 2, "峰值应为两个，得到 {:?}", t.peaks);
    let ds: Vec<f64> = t.peaks.iter().map(|p| p.distance_m).collect();
    assert!(ds.iter().any(|d| (*d - 5.0).abs() < 0.5), "{ds:?}");
    assert!(ds.iter().any(|d| (*d - 20.0).abs() < 0.5), "{ds:?}");
  }

  #[test]
  fn matched_line_has_no_peaks() {
    let s = synth(0.66, 1e6, 900e6, 512, &[]);
    let t = tdr(&s, 0.66).unwrap();
    assert!(t.peaks.is_empty(), "理想匹配不应有反射峰");
    assert!(t.magnitude_db.iter().all(|m| *m == f64::NEG_INFINITY));
    // 分辨率与最大距离仍按数据算出。
    let df = 899e6 / 511.0;
    let v = 0.66 * C;
    assert!((t.max_distance_m - v / (2.0 * df)).abs() < 1e-9);
    assert!((t.resolution_m - v / (2.0 * 899e6)).abs() < 1e-9);
  }

  #[test]
  fn rejects_non_uniform_or_too_few_points() {
    assert_eq!(
      tdr(
        &Sweep {
          z0: 50.0,
          points: vec![]
        },
        0.66
      ),
      Err(TdrError::TooFewPoints)
    );
    let mut s = synth(0.66, 1e6, 10e6, 4, &[(3.0, 0.1)]);
    // 打乱第 3 个点的频率（下标 2），间隔不再均匀；报点号按 1 起。
    // 扰动量要给到间隔的百分之几：几百 Hz 在 3MHz 的间隔上只有 0.004%，本来就该接受（见
    // `quantised_frequencies_are_still_uniform_enough`）。
    s.points[2].f_hz += 123_000.0;
    assert_eq!(tdr(&s, 0.66), Err(TdrError::NotUniform(3)));
  }

  #[test]
  fn quantised_frequencies_are_still_uniform_enough() {
    // 文本导出的频率常被量化到 1kHz（900MHz 处 6 位有效数字）：间隔抖动最多 1kHz，
    // 而按浮点精度定容差（1e-6·df ≈ 1.8Hz）会把这种等间隔文件判成不可用。
    let mut s = synth(0.66, 1e6, 900e6, 512, &[(10.0, 1.0 / 3.0)]);
    for p in &mut s.points {
      p.f_hz = (p.f_hz / 1000.0).round() * 1000.0;
    }
    let t = tdr(&s, 0.66).expect("量化到 1kHz 的等间隔扫频应可用");
    let d = strongest(&t.peaks).expect("应至少有一个反射峰");
    assert!((d - 10.0).abs() < 0.5, "最强峰在 {d} m，应为 10 m");
  }

  #[test]
  fn peak_list_is_capped_at_the_strongest() {
    // 直接构造幅度曲线（绕开 FFT）：每 10 个 bin 一个 0dB 峰、其余 −40dB，共 40 个峰，
    // 彼此间隔远大于 `min_sep_m`。列表只该留下 `MAX_PEAKS` 条，避免 DOM 随文件点数膨胀。
    let n = 400;
    let distances: Vec<f64> = (0..n).map(|i| i as f64 * 0.1).collect();
    let magnitudes: Vec<f64> = (0..n)
      .map(|i| if i % 10 == 5 { 0.0 } else { -40.0 })
      .collect();
    let peaks = find_peaks(&distances, &magnitudes, 0.05);
    assert_eq!(peaks.len(), MAX_PEAKS);
    // 报告顺序仍是按距离升序。
    assert!(peaks.windows(2).all(|w| w[0].distance_m < w[1].distance_m));
  }

  #[test]
  fn distance_axis_spans_the_unambiguous_range() {
    let s = synth(0.66, 1e6, 900e6, 512, &[(1.0, 0.5)]);
    let t = tdr(&s, 0.66).unwrap();
    assert!(t.distances_m[0] == 0.0);
    let last = *t.distances_m.last().unwrap();
    assert!(last < t.max_distance_m && (t.max_distance_m - last) < t.max_distance_m / 1024.0);
  }

  #[test]
  fn many_points_do_not_overflow_the_fft_buffer() {
    // 1 万点的导出文件很常见：FFT 长度必须跟着点数走，不能在 8192 处封顶
    //（封顶后缓冲区装不下 `n` 个采样，填充循环越界 panic）。
    let s = synth(0.66, 1e6, 900e6, 10_000, &[(10.0, 1.0 / 3.0)]);
    let t = tdr(&s, 0.66).unwrap();
    assert!(t.distances_m.len() >= 10_000);
    let d = strongest(&t.peaks).expect("应至少有一个反射峰");
    assert!((d - 10.0).abs() < 0.5, "最强峰在 {d} m，应为 10 m");
  }

  #[test]
  fn too_many_points_are_rejected_instead_of_hanging_the_page() {
    let s = synth(0.66, 1e6, 900e6, MAX_POINTS + 1, &[(10.0, 1.0 / 3.0)]);
    assert_eq!(tdr(&s, 0.66), Err(TdrError::TooManyPoints));
    // 正好等于上限仍然要算（边界不该被误杀）。
    let s = synth(0.66, 1e6, 900e6, MAX_POINTS, &[(10.0, 1.0 / 3.0)]);
    assert!(tdr(&s, 0.66).is_ok());
  }

  #[test]
  fn illegal_velocity_factor_is_rejected() {
    let s = synth(0.66, 1e6, 900e6, 64, &[(10.0, 0.3)]);
    for vf in [0.0, -0.5, f64::NAN, f64::INFINITY, 1.5] {
      assert_eq!(
        tdr(&s, vf),
        Err(TdrError::BadVelocityFactor),
        "vf = {vf} 应被拒绝"
      );
    }
  }

  #[test]
  fn plot_points_decimate_but_keep_the_peak() {
    let s = synth(0.66, 1e6, 900e6, 2048, &[(10.0, 1.0 / 3.0)]);
    let t = tdr(&s, 0.66).unwrap();
    let points = t.plot_points(128);
    // 每列最多两个点（谷 + 峰）。
    assert!(points.len() <= 256, "抽稀后还有 {} 点", points.len());
    assert!(!points.is_empty());
    // 抽稀不能把最强反射抹平：最大幅度必须仍是 0 dB，位置仍在 10 m。
    let top = points
      .iter()
      .max_by(|a, b| a.magnitude_db.total_cmp(&b.magnitude_db))
      .expect("应有采样点");
    assert!((top.magnitude_db - 0.0).abs() < 1e-9, "{top:?}");
    assert!((top.distance_m - 10.0).abs() < 0.5, "{top:?}");
    // 只有第一个点是新折线的起点（空桶没有，所以中途不该出现断点）。
    assert!(points[0].segment_start);
    assert!(points[1..].iter().all(|p| !p.segment_start));
    // 所有幅度都是有限值：`-∞` 的 bin 不该进点列。
    assert!(points.iter().all(|p| p.magnitude_db.is_finite()));
  }

  #[test]
  fn plot_points_of_a_matched_line_are_empty() {
    let s = synth(0.66, 1e6, 900e6, 512, &[]);
    let t = tdr(&s, 0.66).unwrap();
    assert!(t.plot_points(128).is_empty());
  }
}
