//! 波形实验室：调制波形与频谱、星座图与眼图、滤波器响应曲线、香农容量。
//!
//! 面向教学演示，全部函数为纯计算、无状态，页面侧只负责画图与交互。
//!
//! 时间轴采用**归一化基准**：1 个时间单位 = 1 个调制周期。这样横轴直接读作
//! 「第几个调制周期」，频谱横轴直接读作「偏离载波多少个 fm」，不需要在界面上
//! 换算到 Hz。频谱复用 [`crate::fft`]，滤波器响应按 Butterworth 极点解析求值，
//! 幅频 / 相频 / 群延迟三条曲线共用同一次频点扫描（相位在扫描中逐步解卷绕，
//! 群延迟由解卷绕后的相位作数值微分得到 —— 直接对主值相位求导会在 ±π 跳变处
//! 产生假尖峰）。
//!
//! 精度说明：频谱与希尔伯特变换复用 [`crate::fft`] 的 **f32** 实现（内存与速度都更省），
//! 输入输出仍是 f64，但频谱底噪受 f32 限制（约 −60 dB 量级的弱边带会失真）。
//! 教学演示够用；若将来要定量分析更弱的杂散，需要先给 FFT 加 f64 版本。

use std::f64::consts::TAU;

use crate::cx::Cx;
use crate::fft::fft_in_place;
use crate::filter_design::FilterKind;

// ───────────────────────────── 波形与频谱 ─────────────────────────────

/// 调制方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modulation {
  /// 调幅（载波 + 双边带）。
  Am,
  /// 双边带抑制载波（DSB-SC）。
  Dsb,
  /// 上边带（USB）。
  Usb,
  /// 下边带（LSB）。
  Lsb,
  /// 调频（FM）。
  Fm,
}

/// 波形生成参数（时间轴单位：1 个调制周期）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaveSpec {
  /// 调制方式。
  pub kind: Modulation,
  /// 载波与调制频率之比 fc/fm，取整数可让频谱落在整数 bin 上。
  pub carrier_ratio: f64,
  /// 调制指数：AM / DSB 为调制度 m（0–1 为正常调幅），FM 为 β = Δf/fm。
  pub index: f64,
  /// 渲染的调制周期数。
  pub periods: f64,
  /// 采样点数，须为 2 的幂（频谱用 FFT）。
  pub samples: usize,
}

/// 波形参数是否满足全部前置条件。
///
/// 时间轴以「1 个调制周期 = 1」归一化：`periods` 非正会让步长为 0 或负（波形退化成常数
/// 或倒放），`samples < 2` 连半个周期都画不出来，非 2 的幂则过不了希尔伯特变换 / FFT。
///
/// `carrier_ratio` / `index` 只要求**有限**：它们是自由度，取大取小都画得出来；但 `NaN`
/// 会让整条波形（以及后面的频谱）全是 `NaN` —— 曲线静默空白，比报错更难查。本函数是
/// 「非法值退化为空结果」这条约定的唯一门口，漏一项就等于没兜。
fn spec_is_usable(spec: &WaveSpec) -> bool {
  spec.periods.is_finite()
    && spec.periods > 0.0
    && spec.carrier_ratio.is_finite()
    && spec.index.is_finite()
    && spec.samples >= 2
    && spec.samples.is_power_of_two()
}

/// 调制波形（幅度归一化到约 ±1）。
///
/// 单边带用 FFT 希尔伯特变换（解析信号）实现，因此换用更复杂的调制信号时
/// 依然是标准的单边带，而不是把单音掩藏成「相位差 90° 的巧合」。
///
/// 参数不满足前置条件（见 [`spec_is_usable`]）时返回**空波形**，页面什么都不画。
/// 刻意不 `panic!`：本函数会被编进 wasm、由 UI 直接调用，而 [`WaveSpec`] 的字段是公开的，
/// 任何新调用点都可能喂进非法值 —— wasm 里 panic 等于整站白屏。返回空结果也满足
/// 「别静默产出一条看着正常、其实没意义的曲线」这条原则：空就是空。
#[must_use]
pub fn waveform(spec: &WaveSpec) -> Vec<f64> {
  let n = spec.samples;
  if !spec_is_usable(spec) {
    return Vec::new();
  }
  let step = spec.periods / n as f64;
  // 调制信号（音频）：单音。
  let msg: Vec<f64> = (0..n).map(|i| (TAU * i as f64 * step).cos()).collect();
  let carrier_phase = |i: usize| TAU * spec.carrier_ratio * i as f64 * step;

  match spec.kind {
    Modulation::Am => (0..n)
      .map(|i| (1.0 + spec.index * msg[i]) * carrier_phase(i).cos())
      .collect(),
    Modulation::Dsb => (0..n)
      .map(|i| spec.index * msg[i] * carrier_phase(i).cos())
      .collect(),
    Modulation::Fm => {
      // 瞬时相位 = ωc·t + β·sin(ωm·t)；用 sin 积分而不是把频率逐点相加，
      // 免得数值积分累积漂移。
      (0..n)
        .map(|i| {
          let t = i as f64 * step;
          (TAU * spec.carrier_ratio * t + spec.index * (TAU * t).sin()).cos()
        })
        .collect()
    }
    Modulation::Usb | Modulation::Lsb => {
      let hilbert = hilbert(&msg);
      let sign = if spec.kind == Modulation::Usb {
        -1.0
      } else {
        1.0
      };
      (0..n)
        .map(|i| {
          let ph = carrier_phase(i);
          spec.index * (msg[i] * ph.cos() + sign * hilbert[i] * ph.sin())
        })
        .collect()
    }
  }
}

/// 希尔伯特变换（返回解析信号的虚部）。
///
/// 用 FFT 实现：正频率 ×2、负频率置 0、DC 与 Nyquist 保持。输入长度须为 2 的幂；
/// 长度不合法时原样返回（不 panic）—— 它是 [`waveform`] 的内部实现，调用方已先校验。
fn hilbert(x: &[f64]) -> Vec<f64> {
  let n = x.len();
  if n < 2 || !n.is_power_of_two() {
    return x.to_vec();
  }
  let mut re: Vec<f32> = x.iter().map(|&v| v as f32).collect();
  let mut im = vec![0f32; n];
  fft_in_place(&mut re, &mut im);
  for i in 0..n {
    let gain = match i {
      0 => 1.0,
      _ if i == n / 2 => 1.0,
      _ if i < n / 2 => 2.0,
      _ => 0.0,
    };
    re[i] *= gain;
    im[i] *= gain;
  }
  ifft_in_place(&mut re, &mut im);
  im.iter().map(|&v| f64::from(v)).collect()
}

/// 就地逆 FFT，用恒等式 IFFT(X) = conj(FFT(conj(X))) / N 复用正向变换。
fn ifft_in_place(re: &mut [f32], im: &mut [f32]) {
  for v in im.iter_mut() {
    *v = -*v;
  }
  fft_in_place(re, im);
  let n = re.len() as f32;
  for (r, i) in re.iter_mut().zip(im.iter_mut()) {
    *r /= n;
    *i = -*i / n;
  }
}

/// 单边幅度谱。
#[derive(Debug, Clone, PartialEq)]
pub struct Spectrum {
  /// 各 bin 的频率，单位为**调制频率 fm 的倍数**（相对基带，未加载波）。
  pub freqs_fm: Vec<f64>,
  /// 各 bin 的幅度（dB，峰值归一化到 0 dB）。
  pub mag_db: Vec<f64>,
}

/// 计算单边幅度谱。
///
/// `periods` 为样本覆盖的调制周期数（与 [`WaveSpec::periods`] 一致），
/// 因此第 i 个 bin 对应 `i / periods` 个 fm。
///
/// 前置条件（样本数 ≥ 2 且为 2 的幂、`periods` 为正的有限值）不满足时返回**空谱**而不是
/// panic，理由同 [`waveform`]。频率轴是 `i / periods`：非正的 `periods` 会让第 0 个 bin
/// 变成 0/0、其余变成 Inf，整条曲线（连峰值归一化）随之失效 —— 返回空谱比画出一条
/// 静默失效的曲线更好发现。
#[must_use]
pub fn spectrum(samples: &[f64], periods: f64) -> Spectrum {
  let n = samples.len();
  if !(periods.is_finite() && periods > 0.0) || n < 2 || !n.is_power_of_two() {
    return Spectrum {
      freqs_fm: Vec::new(),
      mag_db: Vec::new(),
    };
  }
  let mut re: Vec<f32> = samples
    .iter()
    .enumerate()
    .map(|(i, &v)| {
      // Hann 窗：抑制截断泄漏，便于观察弱边带。
      let w = 0.5 - 0.5 * (TAU * i as f64 / n as f64).cos();
      (v * w) as f32
    })
    .collect();
  let mut im = vec![0f32; n];
  fft_in_place(&mut re, &mut im);

  let bins = n / 2;
  let mut freqs_fm = Vec::with_capacity(bins);
  let mut mags = Vec::with_capacity(bins);
  for i in 0..bins {
    freqs_fm.push(i as f64 / periods);
    mags.push(re[i].hypot(im[i]) as f64);
  }
  let peak = mags.iter().copied().fold(0.0f64, f64::max).max(1e-12);
  let mag_db = mags
    .iter()
    .map(|&m| (20.0 * (m / peak).max(1e-9).log10()).max(-120.0))
    .collect();
  Spectrum { freqs_fm, mag_db }
}

/// 调幅功率分配。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AmPower {
  /// 载波功率（归一化为 1）。
  pub carrier: f64,
  /// 单个边带功率。
  pub sideband_each: f64,
  /// 双边带合计功率。
  pub sideband_total: f64,
  /// 总功率。
  pub total: f64,
  /// 边带占总功率的比例（即发射效率上限）。
  pub efficiency: f64,
}

/// 调制度为 `m` 时的 AM 功率分配（载波归一化为 1）。
///
/// 载波分量幅度 1、每个边带幅度 m/2，故边带功率各为 m²/4；
/// `m = 1`（100% 调制）时总功率 1.5、边带占比 1/3 —— 这就是 AM 效率上限。
#[must_use]
pub fn am_power(m: f64) -> AmPower {
  let each = m * m / 4.0;
  let total = 1.0 + 2.0 * each;
  AmPower {
    carrier: 1.0,
    sideband_each: each,
    sideband_total: 2.0 * each,
    total,
    efficiency: 2.0 * each / total,
  }
}

/// 卡森带宽（Hz）：`BW ≈ 2(Δf + fm)`。
#[must_use]
pub fn carson_bandwidth_hz(peak_dev_hz: f64, mod_hz: f64) -> f64 {
  2.0 * (peak_dev_hz.abs() + mod_hz.abs())
}

// ───────────────────────────── 星座图与眼图 ─────────────────────────────

/// 数字调制方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
  /// 二相相移键控。
  Bpsk,
  /// 四相相移键控。
  Qpsk,
  /// 16 正交幅度调制。
  Qam16,
}

impl Scheme {
  /// 每符号比特数。
  #[must_use]
  pub fn bits_per_symbol(self) -> u32 {
    match self {
      Self::Bpsk => 1,
      Self::Qpsk => 2,
      Self::Qam16 => 4,
    }
  }

  /// 理想星座点（平均符号功率归一化为 1）。
  #[must_use]
  pub fn constellation(self) -> Vec<(f64, f64)> {
    match self {
      Self::Bpsk => vec![(-1.0, 0.0), (1.0, 0.0)],
      Self::Qpsk => {
        // 平均功率：(1+1)/2 = 1，除以 √2 归一化。
        let s = std::f64::consts::FRAC_1_SQRT_2;
        vec![(-s, -s), (-s, s), (s, -s), (s, s)]
      }
      Self::Qam16 => {
        // 每轴取值 ±1、±3，平均功率 (1+1+9+9)/4 = 5，每轴 5 → 总 10，除以 √10。
        let s = 1.0 / 10.0f64.sqrt();
        let mut pts = Vec::with_capacity(16);
        for &a in &[-3.0, -1.0, 1.0, 3.0] {
          for &b in &[-3.0, -1.0, 1.0, 3.0] {
            pts.push((a * s, b * s));
          }
        }
        pts
      }
    }
  }
}

/// 确定性伪随机数（xorshift64*），避免为演示引入 rand 依赖。
struct Rng(u64);

impl Rng {
  fn new(seed: u64) -> Self {
    Self(seed | 1)
  }

  fn next_u64(&mut self) -> u64 {
    let mut x = self.0;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    self.0 = x;
    x.wrapping_mul(0x2545_F491_4F6C_DD1D)
  }

  /// [0, 1) 均匀分布。
  fn uniform(&mut self) -> f64 {
    (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
  }

  /// 标准正态分布（Box-Muller）。
  fn gauss(&mut self) -> f64 {
    let u1 = self.uniform().max(1e-12);
    let u2 = self.uniform();
    (-2.0 * u1.ln()).sqrt() * (TAU * u2).cos()
  }
}

/// 加噪星座点云（每符号平均功率归一化为 1，`snr_db` 为 Es/N0）。
///
/// 每个分量的噪声标准差为 `√(1/(2·10^(snr/10)))`：符号功率 1 摊到 IQ 两路，
/// 于是每路的信号功率是 1/2，噪声功率是 N0/2。
#[must_use]
pub fn scatter(scheme: Scheme, snr_db: f64, points: usize, seed: u64) -> Vec<(f64, f64)> {
  let ideal = scheme.constellation();
  let sigma = (1.0 / (2.0 * 10f64.powf(snr_db / 10.0))).sqrt();
  let mut rng = Rng::new(seed);
  (0..points)
    .map(|_| {
      let idx = (rng.uniform() * ideal.len() as f64) as usize;
      let (sre, sim) = ideal[idx.min(ideal.len() - 1)];
      (sre + sigma * rng.gauss(), sim + sigma * rng.gauss())
    })
    .collect()
}

/// 升余弦脉冲（`t` 以符号周期为单位）。
///
/// `h(t) = sinc(t)·cos(πβt)/(1-(2βt)²)`，在 `t = ±1/(2β)` 处取极限 `(π/4)·sinc(t)`，
/// 否则会算出 0/0。
#[must_use]
pub fn raised_cosine(t: f64, beta: f64) -> f64 {
  if t.abs() < 1e-9 {
    return 1.0;
  }
  let sinc = (std::f64::consts::PI * t).sin() / (std::f64::consts::PI * t);
  let denom = 1.0 - (2.0 * beta * t) * (2.0 * beta * t);
  if denom.abs() < 1e-9 {
    std::f64::consts::FRAC_PI_4 * sinc
  } else {
    sinc * (std::f64::consts::PI * beta * t).cos() / denom
  }
}

/// 眼图轨迹：双极性 NRZ 经升余弦成形后的合成波形切片。
///
/// 返回 `traces` 条轨迹，每条 `2·sps + 1` 个采样点（覆盖 2 个符号周期，
/// 中间时刻即最佳判决点）。`span` 为成形滤波器单侧截断的符号数。
#[must_use]
pub fn eye_traces(beta: f64, sps: usize, span: usize, traces: usize, seed: u64) -> Vec<Vec<f64>> {
  let sps = sps.max(4);
  let span = span.max(1);
  let total_symbols = traces + 2 * span + 2;
  let mut rng = Rng::new(seed);
  let symbols: Vec<f64> = (0..total_symbols)
    .map(|_| if rng.uniform() < 0.5 { -1.0 } else { 1.0 })
    .collect();

  let len = total_symbols * sps;
  let mut out = vec![0.0f64; len];
  // 只累加该符号被成形滤波器覆盖的窗口（±span 个符号，即 ±span·sps 个采样点），
  // 而不是每个符号都扫整条 `out`：复杂度从 O(符号数²·sps) 降到 O(符号数·span·sps)。
  let half = span * sps;
  for (k, &a) in symbols.iter().enumerate() {
    let center = k * sps;
    let lo = center.saturating_sub(half);
    let hi = (center + half).min(len - 1);
    for (n, v) in out.iter_mut().enumerate().take(hi + 1).skip(lo) {
      let t = (n as f64 - center as f64) / sps as f64;
      *v += a * raised_cosine(t, beta);
    }
  }

  let window = 2 * sps + 1;
  (0..traces)
    .map(|i| {
      let start = span * sps + i * sps;
      out[start..start + window].to_vec()
    })
    .collect()
}

// ───────────────────────────── 滤波器响应 ─────────────────────────────

/// 滤波器响应类型（原型的等波纹方式）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterResponse {
  /// 最平坦幅度（Butterworth）：通带与阻带都单调。
  Butterworth,
  /// 通带等波纹（Chebyshev I）：过渡带更陡，代价是通带内的波纹与更差的相位线性。
  Chebyshev1,
  /// 阻带等波纹（Chebyshev II / 逆 Chebyshev）：通带最平坦、阻带有波纹与有限传输零点。
  Chebyshev2,
}

/// 滤波器响应规格。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterSpec {
  /// 频率变换类型。
  pub kind: FilterKind,
  /// 原型类型。
  pub response: FilterResponse,
  /// 阶数（1–5）；带通 / 带阻的实际阶数为 2n。
  pub order: usize,
  /// 波纹参数（dB）：Chebyshev I 为通带波纹，Chebyshev II 为阻带最小衰减；
  /// Butterworth 忽略此字段。
  pub ripple_db: f64,
  /// 低通 / 高通为截止频率，带通 / 带阻为中心频率（Hz）。
  pub fc_hz: f64,
  /// 带宽（Hz），仅带通 / 带阻使用。
  pub bw_hz: f64,
}

/// 单个频点的响应。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResponsePoint {
  /// 频率（Hz）。
  pub f_hz: f64,
  /// 幅频（dB）。
  pub mag_db: f64,
  /// 相频（度，已解卷绕）。
  pub phase_deg: f64,
  /// 群延迟（秒）。
  pub group_delay_s: f64,
}

/// 归一化低通原型：`H(s) = gain · Π(s − z_k) / Π(s − p_k)`。
#[derive(Debug, Clone)]
struct Proto {
  poles: Vec<Cx>,
  zeros: Vec<Cx>,
  gain: f64,
}

impl Proto {
  /// 单极点 / 零点形式的求值。`Π(s − z)` 按 Horner 之外的最朴素连乘，
  /// 阶数 ≤ 10，数值上完全够用。
  fn eval(&self, s: Cx) -> Cx {
    let mut num = Cx::of(self.gain);
    for z in &self.zeros {
      num *= s - *z;
    }
    let mut den = Cx::ONE;
    for p in &self.poles {
      den *= s - *p;
    }
    num / den
  }
}

/// `Π(−x_k)`：极点 / 零点都成共轭对出现，因此乘积必为实数，直接取实部。
fn negated_product(xs: &[Cx]) -> f64 {
  let mut p = Cx::ONE;
  for x in xs {
    p *= Cx::of(0.0) - *x;
  }
  p.re
}

/// 归一化 Butterworth 低通原型的极点（全部落在左半平面）。
///
/// `p_k = exp(jπ(2k + n + 1)/(2n))`，`k = 0..n`。`n = 2` 时得到一对
/// `-√2/2 ± j√2/2`，对应 `1/(s² + √2s + 1)`；`n = 3` 时为 `-1`、`-0.5 ± j√3/2`。
fn butterworth_poles(order: usize) -> Vec<Cx> {
  let n = order.max(1) as f64;
  (0..order.max(1))
    .map(|k| {
      let ang = std::f64::consts::PI * (2.0 * k as f64 + n + 1.0) / (2.0 * n);
      Cx::new_polar(ang)
    })
    .collect()
}

/// Chebyshev I 的波纹系数 `ε`：`ε² = 10^(A/10) − 1`。
fn chebyshev_epsilon(ripple_db: f64) -> f64 {
  (10f64.powf(ripple_db.max(1e-6) / 10.0) - 1.0)
    .max(1e-12)
    .sqrt()
}

/// Chebyshev I（通带等波纹）的归一化低通原型极点。
///
/// `|H(jΩ)|² = 1/(1 + ε²Tₙ²(Ω))`，极点由 `s = j·cos(θ + j·a)` 展开得到：
/// `p_k = −sinh(a)·sin(θ_k) + j·cosh(a)·cos(θ_k)`，其中
/// `a = asinh(1/ε)/n`、`θ_k = π(2k+1)/(2n)`。波纹带边缘（Ω = 1）处衰减恰为
/// `ripple_db` —— 单元测试用解析式 `1/(1+ε²Tₙ²)` 逐点核对整条曲线。
fn chebyshev1_poles(order: usize, ripple_db: f64) -> Vec<Cx> {
  let n = order.max(1) as f64;
  let a = (1.0 / chebyshev_epsilon(ripple_db)).asinh() / n;
  (0..order.max(1))
    .map(|k| {
      let theta = std::f64::consts::PI * (2.0 * k as f64 + 1.0) / (2.0 * n);
      Cx {
        re: -a.sinh() * theta.sin(),
        im: a.cosh() * theta.cos(),
      }
    })
    .collect()
}

/// Chebyshev II（阻带等波纹）的归一化低通原型。
///
/// `|H(jΩ)|² = ε²Tₙ²(1/Ω) / (1 + ε²Tₙ²(1/Ω))`，其中 `ε = 1/√(10^(Aₛ/10) − 1)`
/// 由阻带最小衰减 `stopband_db = Aₛ` 定出（`Ω = 1` 即阻带边缘）。
///
/// - 极点 = Chebyshev I 极点的**倒数**：由 `|H₂|² = 1 − |H₁(j/Ω)|²` 及
///   `s → 1/s` 的替换直接得到（见下），因此仍落在左半平面；
/// - 零点在虚轴上 `±j/cos(θ_k)`，`θ_k = π(2k−1)/(2n)`：这就是 `Tₙ(1/Ω) = 0`
///   的解，即阻带里的有限传输零点。奇数阶时其中一个解落到无穷远，
///   于是只有 `n−1` 个有限零点（分子次数比 `n` 少一次）。
///
/// 直流增益为 1（通带最平坦）。
fn chebyshev2_proto(order: usize, stopband_db: f64) -> Proto {
  let n = order.max(1) as f64;
  let eps = 1.0 / chebyshev_epsilon(stopband_db).max(1e-12);
  // Chebyshev I 用同一个 ε 求出极点，再逐点取倒数。
  let a = (1.0 / eps).asinh() / n;
  let poles: Vec<Cx> = (0..order.max(1))
    .map(|k| {
      let theta = std::f64::consts::PI * (2.0 * k as f64 + 1.0) / (2.0 * n);
      let p = Cx {
        re: -a.sinh() * theta.sin(),
        im: a.cosh() * theta.cos(),
      };
      Cx::ONE / p
    })
    .collect();
  let mut zeros = Vec::new();
  for k in 1..order.max(1) {
    let theta = std::f64::consts::PI * (2.0 * k as f64 - 1.0) / (2.0 * n);
    let cos = theta.cos();
    if cos.abs() < 1e-12 {
      // 该零点在无穷远（奇数阶各有一对这样的解）。
      continue;
    }
    let om = 1.0 / cos;
    if om > 0.0 {
      zeros.push(Cx::j(om));
      zeros.push(Cx::j(-om));
    }
  }
  let gain = negated_product(&poles) / negated_product(&zeros);
  Proto { poles, zeros, gain }
}

/// 由规格构造归一化低通原型。
fn prototype(spec: &FilterSpec) -> Proto {
  match spec.response {
    FilterResponse::Butterworth => {
      let poles = butterworth_poles(spec.order);
      let gain = negated_product(&poles);
      Proto {
        poles,
        zeros: Vec::new(),
        gain,
      }
    }
    FilterResponse::Chebyshev1 => {
      let poles = chebyshev1_poles(spec.order, spec.ripple_db);
      // 偶数阶的直流增益落在波纹谷底：|H(0)| = 1/√(1+ε²)；奇数阶为 1。
      let dc = if spec.order.max(1).is_multiple_of(2) {
        let eps = chebyshev_epsilon(spec.ripple_db);
        1.0 / (1.0 + eps * eps).sqrt()
      } else {
        1.0
      };
      Proto {
        gain: dc * negated_product(&poles),
        poles,
        zeros: Vec::new(),
      }
    }
    FilterResponse::Chebyshev2 => chebyshev2_proto(spec.order, spec.ripple_db),
  }
}

/// 把实际频率映射回原型低通的复频率。
fn proto_freq(spec: &FilterSpec, f_hz: f64) -> Cx {
  let w = TAU * f_hz.max(1e-9);
  let s = Cx::j(w);
  let wc = TAU * spec.fc_hz.max(1e-9);
  match spec.kind {
    FilterKind::LowPass => s / Cx::of(wc),
    FilterKind::HighPass => Cx::of(wc) / s,
    FilterKind::BandPass | FilterKind::BandStop => {
      let num = s * s + Cx::of(wc * wc);
      let den = Cx::of(TAU * spec.bw_hz.max(1e-9)) * s;
      if spec.kind == FilterKind::BandPass {
        num / den
      } else if num.abs() <= f64::EPSILON * wc * wc {
        // 理想带阻在 f₀ 处有**落在虚轴上的精确传输零点**：num 恰为 0，
        // 直接相除得到 0/0 → NaN，会把整条相频 / 群延迟曲线毁掉。
        // 用一个相对极小量代替，只影响陷波最深处那几个点（那里 |H| 本就趋零）。
        den / (num + Cx::of(f64::EPSILON * wc * wc))
      } else {
        den / num
      }
    }
  }
}

/// 在 `f_hz` 处的复传递函数（单点用；批量扫描请复用 [`prototype`] 的结果）。
fn transfer(spec: &FilterSpec, f_hz: f64) -> Cx {
  prototype(spec).eval(proto_freq(spec, f_hz))
}

/// 单点响应：`(幅频 dB, 相位 弧度)`。
#[must_use]
pub fn response_at(spec: &FilterSpec, f_hz: f64) -> (f64, f64) {
  let h = transfer(spec, f_hz);
  (20.0 * h.abs().max(1e-12).log10(), h.arg())
}

/// 在 `[f_start, f_end]` 上均匀取 `points` 个频点，返回幅频 / 相频 / 群延迟。
///
/// 群延迟由解卷绕后的相位作中心差分求得（端点用单边差分）。
/// 扫描步长过粗时（相邻点相位跳变超过 π）解卷绕会失效，调用方应保证
/// 取样足够密 —— 对 1–5 阶滤波器，数百点即可。
#[must_use]
pub fn response_curve(
  spec: &FilterSpec,
  f_start: f64,
  f_end: f64,
  points: usize,
) -> Vec<ResponsePoint> {
  let (lo, hi) = curve_range(f_start, f_end);
  let points = points.max(3);
  let freqs: Vec<f64> = (0..points)
    .map(|i| lo + (hi - lo) * i as f64 / (points - 1) as f64)
    .collect();
  curve_at(spec, &freqs)
}

/// 与 [`response_curve`] 相同，但频点按**对数**均匀分布 —— Bode 图的常规取法：
/// 低通 / 高通的十倍频程与带通 / 带阻的窄带都能兼顾。
#[must_use]
pub fn response_curve_log(
  spec: &FilterSpec,
  f_start: f64,
  f_end: f64,
  points: usize,
) -> Vec<ResponsePoint> {
  let (lo, hi) = curve_range(f_start, f_end);
  let points = points.max(3);
  let (l0, l1) = (lo.log10(), hi.log10());
  let freqs: Vec<f64> = (0..points)
    .map(|i| 10f64.powf(l0 + (l1 - l0) * i as f64 / (points - 1) as f64))
    .collect();
  curve_at(spec, &freqs)
}

/// 频率扫描的起止：下界夹到正数，且保证上界**严格大于**下界。
///
/// 只写 `f_end.max(f_start + 1e-3)` 是不够的：`f_start` 为大负数时下界会先被夹到
/// `1e-6`，上界却仍按原始 `f_start` 算，于是上界小于下界，频点一路倒退。
fn curve_range(f_start: f64, f_end: f64) -> (f64, f64) {
  let lo = f_start.max(1e-6);
  (lo, f_end.max(lo + 1e-3))
}

/// 逐点求值（幅频 / 相频 / 群延迟），频点由调用方给出。
///
/// 幅频求值、相位解卷绕、群延迟中心差分只此一份：页面以前自己抄了一遍，那份没有
/// 单测，与这里一旦漂移就会变成「界面显示的曲线和 core 里验过的实现不是一回事」。
fn curve_at(spec: &FilterSpec, freqs: &[f64]) -> Vec<ResponsePoint> {
  // 原型只与规格有关、与频率无关：一次构造，几百个频点复用。
  let proto = prototype(spec);

  // 1) 幅频与主值相位。
  let raw: Vec<(f64, f64, f64)> = freqs
    .iter()
    .map(|&f| {
      let h = proto.eval(proto_freq(spec, f));
      (f, 20.0 * h.abs().max(1e-12).log10(), h.arg())
    })
    .collect();

  // 2) 相位解卷绕：相邻点取最接近的主值差，累加得到连续相位。
  let mut unwrapped = Vec::with_capacity(raw.len());
  let mut acc = 0.0f64;
  let mut prev: Option<f64> = None;
  for &(_, _, ph) in &raw {
    if let Some(p) = prev {
      let mut d = ph - p;
      while d > std::f64::consts::PI {
        d -= TAU;
      }
      while d < -std::f64::consts::PI {
        d += TAU;
      }
      acc += d;
    } else {
      acc = ph;
    }
    prev = Some(ph);
    unwrapped.push(acc);
  }

  // 3) 群延迟 τ = -dφ/dω（中心差分，端点用单边差分）。
  let points = raw.len();
  (0..points)
    .map(|i| {
      let (a, b) = if i == 0 {
        (0, 1)
      } else if i == points - 1 {
        (points - 2, points - 1)
      } else {
        (i - 1, i + 1)
      };
      let dw = TAU * (raw[b].0 - raw[a].0);
      let dphi = unwrapped[b] - unwrapped[a];
      ResponsePoint {
        f_hz: raw[i].0,
        mag_db: raw[i].1,
        phase_deg: unwrapped[i].to_degrees(),
        group_delay_s: if dw.abs() < 1e-12 { 0.0 } else { -dphi / dw },
      }
    })
    .collect()
}

// ───────────────────────────── 香农容量 ─────────────────────────────

/// 香农极限对应的 Eb/N0（dB）：频谱效率趋于 0 时 `ln 2 → −1.59 dB`。
pub const SHANNON_LIMIT_EBN0_DB: f64 = -1.59;

/// 香农信道容量 `C = B·log₂(1 + S/N)`（bit/s）。
#[must_use]
pub fn capacity_bps(bw_hz: f64, snr_db: f64) -> f64 {
  bw_hz.max(0.0) * (1.0 + 10f64.powf(snr_db / 10.0)).log2()
}

/// 频谱效率（bit/s/Hz）：`log₂(1 + S/N)`。
#[must_use]
pub fn spectral_efficiency(snr_db: f64) -> f64 {
  (1.0 + 10f64.powf(snr_db / 10.0)).log2()
}

/// 给定带宽与目标速率，反解所需 SNR（dB）。
#[must_use]
pub fn required_snr_db(bw_hz: f64, rate_bps: f64) -> f64 {
  if bw_hz <= 0.0 || rate_bps <= 0.0 {
    return f64::NEG_INFINITY;
  }
  let se = rate_bps / bw_hz;
  10.0 * (2f64.powf(se) - 1.0).log10()
}

/// 每比特信噪比 `Eb/N0`（dB）：`Eb/N0 = SNR − 10·lg(频谱效率)`。
///
/// 注意 `snr_db` 与 `se` 必须取自同一条工作点（`se = log₂(1+SNR)`），
/// 否则算出的不是 `Eb/N0`；频谱效率趋于 0 时结果收敛到
/// [`SHANNON_LIMIT_EBN0_DB`]。
#[must_use]
pub fn ebn0_db(snr_db: f64, se: f64) -> f64 {
  if se <= 0.0 {
    return SHANNON_LIMIT_EBN0_DB;
  }
  snr_db - 10.0 * se.log10()
}

#[cfg(test)]
mod tests {
  use super::*;

  const SPEC: WaveSpec = WaveSpec {
    kind: Modulation::Am,
    carrier_ratio: 20.0,
    index: 1.0,
    periods: 2.0,
    samples: 1024,
  };

  /// 频谱峰值的 bin 下标。
  fn peak_bin(s: &Spectrum) -> usize {
    s.mag_db
      .iter()
      .enumerate()
      .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
      .map(|(i, _)| i)
      .unwrap()
  }

  #[test]
  fn am_spectrum_has_carrier_and_two_sidebands() {
    let s = spectrum(&waveform(&SPEC), SPEC.periods);
    // 2 个调制周期 → bin 宽 0.5 fm，载波在 20 fm = bin 40，边带在 ±1 fm = bin 38 / 42。
    assert_eq!(peak_bin(&s), 40, "载波应落在 20 fm");
    assert!(
      s.mag_db[38] > -12.0 && s.mag_db[42] > -12.0,
      "两侧边带应显著"
    );
    // 100% 调制时边带幅度是载波的一半 → 相对载波约 -6 dB。
    assert!(
      (s.mag_db[42] - s.mag_db[40] + 6.0).abs() < 1.0,
      "{:?}",
      &s.mag_db[38..43]
    );
  }

  #[test]
  fn dsb_suppresses_carrier() {
    let spec = WaveSpec {
      kind: Modulation::Dsb,
      ..SPEC
    };
    let s = spectrum(&waveform(&spec), spec.periods);
    // 抑制载波：载波 bin 应比边带低 40 dB 以上。
    assert!(
      s.mag_db[42] - s.mag_db[40] > 40.0,
      "载波残留 {:?}",
      s.mag_db[40]
    );
  }

  #[test]
  fn single_sideband_keeps_only_one_side() {
    let usb = WaveSpec {
      kind: Modulation::Usb,
      ..SPEC
    };
    let s = spectrum(&waveform(&usb), usb.periods);
    let above = s.mag_db[42];
    let below = s.mag_db[38];
    assert!(
      above - below > 30.0,
      "USB 应只保留上边带：上 {above} 下 {below}"
    );

    let lsb = WaveSpec {
      kind: Modulation::Lsb,
      ..SPEC
    };
    let s = spectrum(&waveform(&lsb), lsb.periods);
    assert!(s.mag_db[38] - s.mag_db[42] > 30.0, "LSB 应只保留下边带");
  }

  #[test]
  fn fm_sidebands_follow_bessel_span() {
    let spec = WaveSpec {
      kind: Modulation::Fm,
      index: 2.0,
      ..SPEC
    };
    let s = spectrum(&waveform(&spec), spec.periods);
    // 载波两侧 ±2 fm 处应出现显著边带（β=2 时 J2 不可忽略）。
    assert!(s.mag_db[40 - 4] > -20.0 && s.mag_db[40 + 4] > -20.0);
    // 远离载波处（±10 fm = ±20 bin）应已很弱。
    assert!(s.mag_db[20] < -35.0, "{:?}", s.mag_db[20]);
  }

  #[test]
  fn hilbert_of_cosine_is_sine() {
    let n = 256;
    let x: Vec<f64> = (0..n)
      .map(|i| (TAU * 8.0 * i as f64 / n as f64).cos())
      .collect();
    let h = hilbert(&x);
    // cos 的希尔伯特变换是 sin；变换只在内部样本上准确（边界有吉布斯振铃）。
    for (i, &got) in h.iter().enumerate().take(n - 10).skip(10) {
      let want = (TAU * 8.0 * i as f64 / n as f64).sin();
      assert!((got - want).abs() < 0.02, "i={i} 得到 {got} 期望 {want}");
    }
  }

  /// 这三条公开函数会被 UI 直接调用（wasm 里 panic = 整站白屏），因此参数非法必须
  /// 退化成空结果而不是炸：`samples` 非 2 的幂 / 太小、`periods` 非正或非有限。
  #[test]
  fn invalid_inputs_degrade_to_empty_instead_of_panicking() {
    let spec = |samples: usize, periods: f64| WaveSpec {
      kind: Modulation::Am,
      carrier_ratio: 20.0,
      index: 1.0,
      periods,
      samples,
    };
    assert!(waveform(&spec(1024, 0.0)).is_empty(), "periods = 0");
    assert!(waveform(&spec(1024, -2.0)).is_empty(), "periods < 0");
    assert!(waveform(&spec(1024, f64::NAN)).is_empty(), "periods = NaN");
    assert!(waveform(&spec(1000, 2.0)).is_empty(), "样本数非 2 的幂");
    assert!(waveform(&spec(1, 2.0)).is_empty(), "样本数不足");
    // 单边带走希尔伯特变换，同样不能炸。
    let mut usb = spec(1000, 2.0);
    usb.kind = Modulation::Usb;
    assert!(waveform(&usb).is_empty());
    // hilbert 自身对非法长度原样返回。
    assert_eq!(hilbert(&[1.0, 2.0, 3.0]), vec![1.0, 2.0, 3.0]);

    for (samples, periods) in [
      (7usize, 2.0),
      (0, 2.0),
      (1, 2.0),
      (8, 0.0),
      (8, -1.0),
      (8, f64::NAN),
      (8, f64::INFINITY),
    ] {
      let s = spectrum(&vec![0.0; samples], periods);
      assert!(
        s.freqs_fm.is_empty() && s.mag_db.is_empty(),
        "samples = {samples}、periods = {periods} 应返回空谱"
      );
    }
    // 正常参数仍然出结果（别把「退化」写成了「永远空」）。
    assert_eq!(waveform(&spec(1024, 2.0)).len(), 1024);
    assert!(!spectrum(&vec![0.0; 1024], 2.0).mag_db.is_empty());
  }

  #[test]
  fn am_power_split_matches_textbook() {
    let p = am_power(1.0);
    assert!((p.sideband_each - 0.25).abs() < 1e-12);
    assert!((p.total - 1.5).abs() < 1e-12);
    assert!((p.efficiency - 1.0 / 3.0).abs() < 1e-12);
    let zero = am_power(0.0);
    assert!((zero.total - 1.0).abs() < 1e-12 && zero.efficiency.abs() < 1e-12);
  }

  #[test]
  fn carson_bandwidth_matches_formula() {
    assert!((carson_bandwidth_hz(3000.0, 1000.0) - 8000.0).abs() < 1e-9);
  }

  #[test]
  fn constellations_are_unit_average_power() {
    for scheme in [Scheme::Bpsk, Scheme::Qpsk, Scheme::Qam16] {
      let pts = scheme.constellation();
      assert_eq!(pts.len(), 1usize << scheme.bits_per_symbol());
      let power: f64 = pts.iter().map(|(a, b)| a * a + b * b).sum::<f64>() / pts.len() as f64;
      assert!((power - 1.0).abs() < 1e-9, "{scheme:?} 平均功率 {power}");
    }
  }

  #[test]
  fn high_snr_scatter_stays_near_constellation() {
    let pts = scatter(Scheme::Qam16, 40.0, 400, 7);
    let ideal = Scheme::Qam16.constellation();
    for (re, im) in pts {
      let d = ideal
        .iter()
        .map(|(a, b)| ((a - re).powi(2) + (b - im).powi(2)).sqrt())
        .fold(f64::INFINITY, f64::min);
      assert!(d < 0.05, "40 dB 下偏离理想点 {d}");
    }
  }

  #[test]
  fn raised_cosine_is_nyquist() {
    for beta in [0.0, 0.35, 0.5, 1.0] {
      assert!((raised_cosine(0.0, beta) - 1.0).abs() < 1e-12);
      for k in [1.0, 2.0, 3.0] {
        assert!(
          raised_cosine(k, beta).abs() < 1e-9,
          "β={beta} k={k} → {}（应满足无码间干扰）",
          raised_cosine(k, beta)
        );
      }
    }
  }

  #[test]
  fn eye_traces_shape_and_levels() {
    let traces = eye_traces(0.35, 16, 6, 40, 3);
    assert_eq!(traces.len(), 40);
    for tr in &traces {
      assert_eq!(tr.len(), 33);
      for &v in tr {
        // 升余弦脉冲的尾巴会让波形在判决点之间过冲：β=0.35 时最坏峰值失真接近 1.8
        // （Σ|h(t−kT)| 上界）。这里只用来抓住「成形归一化写错」这类量级错误
        // —— 那会给出 3、5 这种值，而不是 1.8。
        assert!(v.abs() < 2.0, "眼图幅度 {v} 超出预期");
      }
    }
    // 最佳判决点（每条轨迹中点，即第二个符号的采样时刻）应聚集在两个电平附近：
    // 升余弦满足无码间干扰，该时刻的值应恰为 ±1（仅剩截断尾巴的残差）。
    let mid: Vec<f64> = traces.iter().map(|t| t[16]).collect();
    assert!(
      mid.iter().all(|v| (v.abs() - 1.0).abs() < 0.15),
      "中点应张开：{mid:?}"
    );
    assert!(mid.iter().any(|v| *v > 0.0) && mid.iter().any(|v| *v < 0.0));
  }

  #[test]
  fn lowpass_cutoff_is_minus_3db() {
    let spec = FilterSpec {
      kind: FilterKind::LowPass,
      response: FilterResponse::Butterworth,
      order: 2,
      ripple_db: 0.0,
      fc_hz: 1000.0,
      bw_hz: 0.0,
    };
    // 截止频率处恰好 −3.0103 dB（Butterworth 定义）。
    let (at_fc, _) = response_at(&spec, 1000.0);
    assert!((at_fc + 3.0103).abs() < 1e-6, "fc 处 {at_fc}");
    // 二阶 → 每十倍频程 40 dB。
    let (at_10fc, _) = response_at(&spec, 10_000.0);
    assert!((at_10fc + 40.0).abs() < 0.01, "10fc 处 {at_10fc}");
    // 曲线应单调下降，且与单点求值一致。
    let pts = response_curve(&spec, 100.0, 10_000.0, 401);
    assert!(pts.windows(2).all(|w| w[0].mag_db >= w[1].mag_db - 1e-9));
    assert!(
      (pts[0].mag_db - response_at(&spec, pts[0].f_hz).0).abs() < 1e-9,
      "曲线与单点求值应一致"
    );
  }

  #[test]
  fn highpass_blocks_dc_and_passes_high() {
    let spec = FilterSpec {
      kind: FilterKind::HighPass,
      response: FilterResponse::Butterworth,
      order: 3,
      ripple_db: 0.0,
      fc_hz: 1000.0,
      bw_hz: 0.0,
    };
    let pts = response_curve(&spec, 10.0, 10000.0, 401);
    let last = pts.last().unwrap();
    assert!(last.mag_db > -0.1, "高频应直通：{}", last.mag_db);
    assert!(pts[0].mag_db < -60.0, "低频应阻断：{}", pts[0].mag_db);
  }

  #[test]
  fn bandpass_and_bandstop_are_duals() {
    let bp = FilterSpec {
      kind: FilterKind::BandPass,
      response: FilterResponse::Butterworth,
      order: 3,
      ripple_db: 0.0,
      fc_hz: 7_100_000.0,
      bw_hz: 500_000.0,
    };
    let bs = FilterSpec {
      kind: FilterKind::BandStop,
      ..bp
    };
    // 中心频率：带通直通、带阻深衰；偏离一个带宽：恰好相反。
    // （带阻中心取 1.000001 倍频，避开谐振点上必然出现的 0/0。）
    let center_bp = response_at(&bp, 7_100_000.0).0;
    let center_bs = response_at(&bs, 7_100_000.0 * 1.000_001).0;
    let off_bp = response_at(&bp, 7_100_000.0 * 2.0).0;
    let off_bs = response_at(&bs, 7_100_000.0 * 2.0).0;
    assert!(center_bp > -1e-6, "带通中心应导通：{center_bp}");
    assert!(center_bs < -30.0, "带阻中心应深衰：{center_bs}");
    assert!(off_bp < -20.0, "带通远端应阻断：{off_bp}");
    assert!(off_bs > -1.0, "带阻远端应导通：{off_bs}");
    // 中心处的群延迟：带阻在陷波处必然出现群延迟尖峰（物理上真实存在）。
    let curve = response_curve(&bs, 6_000_000.0, 8_000_000.0, 801);
    let peak = curve
      .iter()
      .map(|p| p.group_delay_s.abs())
      .fold(0.0f64, f64::max);
    assert!(curve.iter().all(|p| p.group_delay_s.is_finite()));
    assert!(peak > 1e-7, "陷波处群延迟应有尖峰，实际峰值 {peak}");
  }

  #[test]
  fn group_delay_at_dc_matches_butterworth() {
    // 二阶 Butterworth 低通在直流处的群延迟为 √2/ωc。
    let spec = FilterSpec {
      kind: FilterKind::LowPass,
      response: FilterResponse::Butterworth,
      order: 2,
      ripple_db: 0.0,
      fc_hz: 1000.0,
      bw_hz: 0.0,
    };
    let pts = response_curve(&spec, 1.0, 20.0, 200);
    let want = std::f64::consts::SQRT_2 / (TAU * 1000.0);
    assert!(
      (pts[0].group_delay_s - want).abs() / want < 0.01,
      "得到 {} 期望 {want}",
      pts[0].group_delay_s
    );
    assert!(pts[0].phase_deg.abs() < 1.0, "低频相位应为负小量");
  }

  /// 第一类 Chebyshev 多项式 `Tₙ(Ω)`（递推定义）。
  fn chebyshev_t(n: usize, omega: f64) -> f64 {
    let (mut t0, mut t1) = (1.0f64, omega);
    if n == 0 {
      return t0;
    }
    for _ in 2..=n {
      let t2 = 2.0 * omega * t1 - t0;
      t0 = t1;
      t1 = t2;
    }
    t1
  }

  /// 逐点核对：构造出来的传递函数必须与解析式 `|H(jΩ)|²` 完全一致。
  ///
  /// 这是三种原型最硬的判据 —— 极点 / 零点的位置、符号、直流增益归一化只要错
  /// 一处，曲线就会整体偏离，而不是「差一点点」。
  fn assert_matches_closed_form(spec: &FilterSpec, omega_max: f64) {
    let n = spec.order;
    let eps = chebyshev_epsilon(spec.ripple_db);
    for i in 0..=200 {
      // 对数刻度的 Ω 取样：同时覆盖通带（Ω≪1）与阻带（Ω≫1）。
      let omega = (omega_max.ln() * i as f64 / 200.0).exp();
      let analytic = match spec.response {
        FilterResponse::Butterworth => 1.0 / (1.0 + omega.powi(2 * n as i32)),
        FilterResponse::Chebyshev1 => {
          let t = chebyshev_t(n, omega);
          1.0 / (1.0 + eps * eps * t * t)
        }
        FilterResponse::Chebyshev2 => {
          let t = chebyshev_t(n, 1.0 / omega);
          let e = 1.0 / eps;
          e * e * t * t / (1.0 + e * e * t * t)
        }
      };
      let (mag_db, _) = response_at(spec, spec.fc_hz * omega);
      // `response_at` 把 |H| 下限夹在 1e-12（−240 dB）以避免 log(0)；期望值同样夹住。
      let want_db = (10.0 * analytic.max(1e-30).log10()).max(-240.0);
      assert!(
        (mag_db - want_db).abs() < 1e-6,
        "{:?} n={n} ripple={} Ω={omega}：得到 {mag_db} dB，解析式 {want_db} dB",
        spec.response,
        spec.ripple_db
      );
    }
  }

  #[test]
  fn all_three_prototypes_match_their_closed_forms() {
    for &response in &[
      FilterResponse::Butterworth,
      FilterResponse::Chebyshev1,
      FilterResponse::Chebyshev2,
    ] {
      for order in 1..=5usize {
        for &ripple_db in &[0.5, 1.0, 3.0, 40.0] {
          let spec = FilterSpec {
            kind: FilterKind::LowPass,
            response,
            order,
            ripple_db,
            fc_hz: 1000.0,
            bw_hz: 0.0,
          };
          assert_matches_closed_form(&spec, 1e3);
        }
      }
    }
  }

  #[test]
  fn chebyshev1_ripples_exactly_at_its_passband_edge() {
    // `Tₙ(1) = 1` 对任意阶数成立，因此波纹带边缘（Ω = 1）处一律是 −A dB。
    // 阶数的奇偶只影响**直流**增益：奇数阶的波纹从 0 dB 起，偶数阶从 −A dB 起。
    for &(order, ripple_db) in &[(3usize, 1.0), (5, 0.5), (4, 3.0), (2, 3.0)] {
      let spec = FilterSpec {
        kind: FilterKind::LowPass,
        response: FilterResponse::Chebyshev1,
        order,
        ripple_db,
        fc_hz: 1000.0,
        bw_hz: 0.0,
      };
      let at_edge = response_at(&spec, 1000.0).0;
      assert!(
        (at_edge + ripple_db).abs() < 1e-9,
        "{order} 阶 {ripple_db} dB：带边 {at_edge}，期望 {}",
        -ripple_db
      );
      let dc = response_at(&spec, 1e-3).0;
      let want_dc = if order.is_multiple_of(2) {
        -ripple_db
      } else {
        0.0
      };
      assert!(
        (dc - want_dc).abs() < 1e-6,
        "{order} 阶直流增益 {dc}，期望 {want_dc}"
      );
      // 通带内不应超过 0 dB（波纹只在 [−A, 0] 之间来回）。
      for i in 0..=50 {
        let f = 1000.0 * i as f64 / 50.0 + 1.0;
        assert!(response_at(&spec, f).0 <= 1e-9, "通带内出现正增益");
      }
    }
  }

  #[test]
  fn chebyshev1_is_steeper_than_butterworth_of_same_order() {
    // 同阶、同波纹带边缘：Chebyshev I 的过渡带更陡。
    let mk = |response| FilterSpec {
      kind: FilterKind::LowPass,
      response,
      order: 4,
      ripple_db: 0.5,
      fc_hz: 1000.0,
      bw_hz: 0.0,
    };
    let butter = response_at(&mk(FilterResponse::Butterworth), 2000.0).0;
    let cheby = response_at(&mk(FilterResponse::Chebyshev1), 2000.0).0;
    assert!(
      cheby < butter - 3.0,
      "Chebyshev {cheby} 应明显低于 {butter}"
    );
  }

  #[test]
  fn chebyshev2_has_finite_transmission_zeros_in_the_stopband() {
    let n = 4usize;
    let stopband_db = 40.0;
    let spec = FilterSpec {
      kind: FilterKind::LowPass,
      response: FilterResponse::Chebyshev2,
      order: n,
      ripple_db: stopband_db,
      fc_hz: 1000.0,
      bw_hz: 0.0,
    };
    let pi = std::f64::consts::PI;

    // 1) 通带最平坦：直流 0 dB。
    assert!(response_at(&spec, 1.0).0.abs() < 1e-9, "直流应 0 dB");

    // 2) 阻带等波纹：波纹峰值出现在 `Tₙ(1/Ω) = ±1` 处，即 `Ω = 1/|cos(mπ/n)|`
    //    （含边缘 Ω = 1），峰值一律等于设定的最小衰减。
    let mut peaks = vec![1.0f64];
    for m in 1..n {
      let c = (m as f64 * pi / n as f64).cos();
      if c.abs() > 1e-9 {
        let omega = 1.0 / c.abs();
        if omega >= 1.0 {
          peaks.push(omega);
        }
      }
    }
    for omega in peaks {
      let got = response_at(&spec, 1000.0 * omega).0;
      assert!(
        (got + stopband_db).abs() < 1e-6,
        "Ω={omega} 处波纹峰值 {got}，期望 {}",
        -stopband_db
      );
    }
    // 无穷远处（偶数阶）同样落在波纹峰值上。
    let far = response_at(&spec, 1e9).0;
    assert!((far + stopband_db).abs() < 1e-6, "高频极限 {far}");

    // 3) 有限传输零点：`Tₙ(1/Ω) = 0` 的解，落在虚轴上（Ω > 1），响应在此深陷。
    //    不能靠均匀扫描找零点（陷波极窄，网格永远采不到谷底），直接按解析位置取值。
    let mut zeros = Vec::new();
    for k in 1..=n {
      let c = ((2.0 * k as f64 - 1.0) * pi / (2.0 * n as f64)).cos();
      if c.abs() > 1e-9 {
        let omega = 1.0 / c.abs();
        if omega > 1.0 && !zeros.iter().any(|z: &f64| (z - omega).abs() < 1e-9) {
          zeros.push(omega);
        }
      }
    }
    assert_eq!(zeros.len(), n / 2, "4 阶应有两对共轭传输零点");
    for omega in zeros {
      let got = response_at(&spec, 1000.0 * omega * (1.0 + 1e-9)).0;
      assert!(got < -150.0, "Ω≈{omega} 应是传输零点，实际 {got} dB");
    }

    // 4) 零点附近的 0/0 不能漏成 NaN，整条曲线必须有限。
    let pts = response_curve(&spec, 1000.0, 4000.0, 3001);
    assert!(
      pts
        .iter()
        .all(|p| p.mag_db.is_finite() && p.phase_deg.is_finite() && p.group_delay_s.is_finite())
    );
  }

  #[test]
  fn shannon_capacity_and_inverse_agree() {
    assert!((capacity_bps(1000.0, 0.0) - 1000.0).abs() < 1e-6);
    assert!((spectral_efficiency(10.0) - 11f64.log2()).abs() < 1e-9);
    let rate = capacity_bps(1000.0, 10.0);
    assert!((required_snr_db(1000.0, rate) - 10.0).abs() < 1e-6);
    // 频谱效率 → 0 时 Eb/N0 收敛到香农极限 −1.59 dB。
    // 必须取同一条工作点上的 (SNR, se)：se = log₂(1+SNR)。
    let se = 1e-6;
    let snr = required_snr_db(1.0, se);
    assert!((ebn0_db(snr, se) - SHANNON_LIMIT_EBN0_DB).abs() < 0.01);
    // 0 dB SNR 的 1 bit/s/Hz 工作点：Eb/N0 应为 0 dB。
    assert!((ebn0_db(0.0, 1.0) - 0.0).abs() < 1e-9);
  }

  // ───────────────────────────── 边界与前置换条件 ─────────────────────────────

  /// 时间轴以「1 个调制周期 = 1」归一化：`periods` 非正会让步长为 0 或负。
  /// 这类非法参数**返回空结果**而不是 panic（详见 `invalid_inputs_degrade_to_empty_...`）。
  #[test]
  fn waveform_rejects_non_positive_periods() {
    let out = waveform(&WaveSpec {
      periods: 0.0,
      ..SPEC
    });
    assert!(out.is_empty());
  }

  #[test]
  fn waveform_rejects_a_single_sample() {
    let out = waveform(&WaveSpec { samples: 1, ..SPEC });
    assert!(out.is_empty());
  }

  /// `carrier_ratio` / `index` 为 `NaN` / `Inf` 时必须是**空波形**，而不是一整条 `NaN`。
  ///
  /// 这两项只受 `spec_is_usable` 把关，曾经漏掉：页面画出一条静默失效的曲线，
  /// 比什么都不画更难查（本模块的约定就是「非法值退化为空结果」）。
  #[test]
  fn waveform_rejects_non_finite_carrier_and_index() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
      assert!(
        waveform(&WaveSpec {
          carrier_ratio: bad,
          ..SPEC
        })
        .is_empty(),
        "carrier_ratio = {bad} 应退化为空波形"
      );
      assert!(
        waveform(&WaveSpec { index: bad, ..SPEC }).is_empty(),
        "index = {bad} 应退化为空波形"
      );
    }
    // 合法但极端的取值仍要算得出东西（它们只是自由度，不是错误）。
    assert!(
      !waveform(&WaveSpec {
        carrier_ratio: 256.0,
        ..SPEC
      })
      .is_empty()
    );
  }

  /// 频率轴是 `i / periods`：非正会让第 0 个 bin 变成 0/0、其余变成 Inf。
  #[test]
  fn spectrum_rejects_non_positive_periods() {
    let s = spectrum(&waveform(&SPEC), 0.0);
    assert!(s.freqs_fm.is_empty() && s.mag_db.is_empty());
  }

  /// DSB 无载波、SSB 只剩一条边带，两者都按 `index` 缩放。
  #[test]
  fn dsb_and_ssb_amplitudes_follow_the_modulation_index() {
    let peak = |v: &[f64]| v.iter().fold(0.0f64, |m, x| m.max(x.abs()));

    let dsb = waveform(&WaveSpec {
      kind: Modulation::Dsb,
      ..SPEC
    });
    // `index·cos(ωm t)·cos(ωc t)`：峰值就是 index（这里 index = 1）。
    let p = peak(&dsb);
    assert!(
      (p - SPEC.index).abs() < 0.05 * SPEC.index,
      "DSB 峰值 {p} 应约等于 index {}",
      SPEC.index
    );

    // 单边带：`cos(ωm t)cos(ωc t) ∓ sin(ωm t)sin(ωc t) = cos((ωc ± ωm)t)`，
    // 归一化单音因此峰值就是 index —— 若希尔伯特变换不对，这里会明显偏离。
    for kind in [Modulation::Usb, Modulation::Lsb] {
      let w = waveform(&WaveSpec { kind, ..SPEC });
      let p = peak(&w);
      assert!(
        (p - SPEC.index).abs() < 0.05 * SPEC.index,
        "{kind:?} 峰值 {p} 应约等于 index {}",
        SPEC.index
      );
      assert!(w.iter().all(|v| v.is_finite()));
    }
  }

  /// 眼图对退化尺寸的夹取：`sps < 4` / `span = 0` 不能求出空轨迹或越界。
  #[test]
  fn eye_traces_clamp_degenerate_sizes() {
    let traces = eye_traces(0.35, 1, 0, 3, 42);
    assert_eq!(traces.len(), 3);
    // sps 夹到 4、span 夹到 1 → 每条 2·4 + 1 个点。
    for t in &traces {
      assert_eq!(t.len(), 9);
      assert!(t.iter().all(|v| v.is_finite()));
    }
    // 0 条轨迹时不应 panic。
    assert!(eye_traces(0.35, 8, 2, 0, 1).is_empty());
  }

  /// 奇数阶 Chebyshev II 少一对有限传输零点（`c = cos((2k−1)π/2n)` 为 0 的那个
  /// 不落在 Ω > 1 上），且高频极限继续往下掉 —— 偶数阶止于波纹峰值（`Tₙ(0) = ±1`），
  /// 奇数阶 `Tₙ(0) = 0`，`|H| ≈ ε·Tₙ(1/Ω) → 0`，两者形状不同，别混用同一套断言。
  #[test]
  fn chebyshev2_odd_order_has_one_fewer_zero() {
    let n = 5;
    let stopband_db = 40.0;
    let spec = FilterSpec {
      kind: FilterKind::LowPass,
      response: FilterResponse::Chebyshev2,
      order: n,
      ripple_db: stopband_db,
      fc_hz: 1000.0,
      bw_hz: 0.0,
    };
    let pi = std::f64::consts::PI;

    assert!(response_at(&spec, 1.0).0.abs() < 1e-9, "直流应 0 dB");
    // `T₅(1e-9) ≈ 5e-9`，ε = 0.01 → `|H| ≈ 5e-11` → 约 −146 dB。
    let far = response_at(&spec, 1e9).0;
    assert!(far < -100.0, "奇数阶高频应继续深衰，实际 {far} dB");

    // `Tₙ(1/Ω) = 0` 的解里，`c = cos((2k−1)π/2n)` 为 0 的那个（奇数阶有且仅有一个）
    // 不落在 Ω > 1 上，因此有限零点数为 (n−1)/2。
    let mut zeros = Vec::new();
    for k in 1..=n {
      let c = ((2.0 * k as f64 - 1.0) * pi / (2.0 * n as f64)).cos();
      if c.abs() > 1e-9 {
        let omega = 1.0 / c.abs();
        if omega > 1.0 && !zeros.iter().any(|z: &f64| (z - omega).abs() < 1e-9) {
          zeros.push(omega);
        }
      }
    }
    assert_eq!(zeros.len(), (n - 1) / 2, "5 阶应有两对共轭传输零点");
    for omega in zeros {
      let got = response_at(&spec, 1000.0 * omega * (1.0 + 1e-9)).0;
      assert!(got < -150.0, "Ω≈{omega} 应是传输零点，实际 {got} dB");
    }
  }

  /// 反函数的退化输入：带宽 / 速率非正时无解（−∞）；`se ≤ 0` 时 Eb/N0 回到香农极限。
  #[test]
  fn shannon_inverse_handles_degenerate_inputs() {
    for (bw, rate) in [(0.0, 100.0), (-1.0, 100.0), (1000.0, 0.0), (1000.0, -5.0)] {
      assert_eq!(
        required_snr_db(bw, rate),
        f64::NEG_INFINITY,
        "bw={bw} rate={rate}"
      );
    }
    assert_eq!(ebn0_db(10.0, 0.0), SHANNON_LIMIT_EBN0_DB);
    assert_eq!(ebn0_db(10.0, -1.0), SHANNON_LIMIT_EBN0_DB);
    // 频谱效率 1 bit/s/Hz：lg 1 = 0，Eb/N0 就等于 SNR。
    assert!((ebn0_db(10.0, 1.0) - 10.0).abs() < 1e-9);
  }

  fn butterworth_lowpass(order: usize) -> FilterSpec {
    FilterSpec {
      kind: FilterKind::LowPass,
      response: FilterResponse::Butterworth,
      order,
      ripple_db: 0.0,
      fc_hz: 1000.0,
      bw_hz: 0.0,
    }
  }

  /// 对数扫描：频点严格递增、落在区间两端，且幅频与逐点 `response_at` 完全一致
  /// （页面用的就是这一份，不再自己抄一遍）。
  #[test]
  fn log_sweep_is_increasing_and_matches_pointwise_evaluation() {
    let spec = butterworth_lowpass(3);
    let pts = response_curve_log(&spec, 100.0, 10_000.0, 200);
    assert_eq!(pts.len(), 200);
    for w in pts.windows(2) {
      assert!(w[0].f_hz < w[1].f_hz, "频点必须严格递增：{}", w[0].f_hz);
    }
    assert!((pts[0].f_hz - 100.0).abs() < 1e-6, "起点 {}", pts[0].f_hz);
    assert!(
      (pts[199].f_hz - 10_000.0).abs() < 1e-3,
      "终点 {}",
      pts[199].f_hz
    );
    for p in [&pts[0], &pts[100], &pts[199]] {
      assert!(p.mag_db.is_finite() && p.group_delay_s.is_finite());
      assert!(
        (p.mag_db - response_at(&spec, p.f_hz).0).abs() < 1e-9,
        "幅频应与逐点求值一致：{} vs {}",
        p.mag_db,
        response_at(&spec, p.f_hz).0
      );
    }
  }

  /// 扫描区间退化（起点为负 / 终点小于起点 / 两端相等）时，频点不能倒退或重合。
  #[test]
  fn degenerate_sweep_bounds_still_increase() {
    let spec = butterworth_lowpass(2);
    for (f0, f1) in [(-5.0, -1.0), (10.0, 1.0), (0.0, 0.0)] {
      for pts in [
        response_curve(&spec, f0, f1, 5),
        response_curve_log(&spec, f0, f1, 5),
      ] {
        assert!(pts.iter().all(|p| p.f_hz.is_finite() && p.f_hz > 0.0));
        for w in pts.windows(2) {
          assert!(w[0].f_hz < w[1].f_hz, "({f0}, {f1}) 频点倒退");
        }
      }
    }
  }
}
