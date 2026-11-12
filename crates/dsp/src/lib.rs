//! 解码器共用的 DSP 原语。
//!
//! RTTY / Olivia / DominoEX / NAVTEX 各自曾内联一份逐字相同的 Goertzel；统一到这里，
//! 修一处就够了。刻意只放「**与具体调制无关**」的东西：`wspr` 的 Goertzel 按窗口长度取整到
//! 频点（语义不同），不在此列。

/// Goertzel：`samples` 在 `freq`（Hz）处的功率（未归一化，量纲 = 幅度² × 窗口长度²）。
///
/// 与按整数频点取整的 DFT 单点不同，这里直接用 `freq / rate` 的**非整数**角频率，所以
/// 可以对任意频率（含 1 Hz 步进的音调搜索）求值。`rate == 0` 或非有限频率得到 `NaN` /
/// `inf`，调用方负责事先拒绝（各解码器入口已有守卫）。
#[must_use]
pub fn tone_power(samples: &[f32], rate: u32, freq: f64) -> f64 {
  let w = 2.0 * std::f64::consts::PI * freq / f64::from(rate);
  let coeff = 2.0 * w.cos();
  let mut s0 = 0.0f64;
  let mut s1 = 0.0f64;
  for &x in samples {
    let s = f64::from(x) + coeff * s1 - s0;
    s0 = s1;
    s1 = s;
  }
  s1 * s1 + s0 * s0 - coeff * s1 * s0
}

/// 就地加入固定种子的 LCG 均匀噪声（幅度 `amp`，样本 ∈ `[-amp, amp]`）。
///
/// 各解码器 sample 生成器此前各抄一份相同的 LCG；统一到这里，保证「同文本 → 同噪声」的
/// 样本可复现（e2e / 手工对照依赖这一点）。
pub fn add_noise(signal: &mut [f32], amp: f32) {
  let mut seed = 0x1234_5678u32;
  for s in signal {
    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    let u = (seed >> 8) as f32 / 16_777_216.0;
    *s += amp * (u * 2.0 - 1.0);
  }
}

/// 线性插值重采样：`ratio` > 1 把信号拉长（模拟录音机 / 声卡时钟偏慢），< 1 压短。
///
/// 解码器的漂移跟踪测试共用（MFSK / THOR 各自原来抄了一份）。`ratio` 非正或非有限、输入为空时返回空。
#[must_use]
pub fn stretch(x: &[f32], ratio: f64) -> Vec<f32> {
  if x.is_empty() || !ratio.is_finite() || ratio <= 0.0 {
    return Vec::new();
  }
  let last = x.len() - 1;
  let n = (x.len() as f64 * ratio) as usize;
  (0..n)
    .map(|i| {
      let p = i as f64 / ratio;
      let k = p.floor() as usize;
      let f = (p - p.floor()) as f32;
      let a = x[k.min(last)];
      let b = x[(k + 1).min(last)];
      a + (b - a) * f
    })
    .collect()
}

/// 转速线性变化的重采样：输出里每秒对应输入 `1 / (1 + ratio0 + slope × t)` 秒（`t` 为输出秒数）。
///
/// `ratio0` 是常数的时钟误差，`slope` 让它随时间变：符号边界漂成二次曲线，音调频率按同一比例漂移
/// （录音机转速不稳 / 声卡时钟受温度影响）。`slope = 0` 时等价于 [`stretch`]`(x, 1 + ratio0)`。
/// 输入为空、`1 + ratio0 <= 0` 或参数非有限时返回空。
#[must_use]
pub fn warp(x: &[f32], rate: f64, ratio0: f64, slope: f64) -> Vec<f32> {
  let b = 1.0 + ratio0;
  if x.is_empty()
    || !(rate.is_finite() && rate > 0.0 && b.is_finite() && b > 0.0 && slope.is_finite())
  {
    return Vec::new();
  }
  let last = x.len() - 1;
  let total = x.len() as f64 / rate;
  let n_out = (total * (b + slope * total / 2.0) * rate) as usize;
  let a = slope / 2.0;
  (0..n_out)
    .map(|i| {
      let t = i as f64 / rate;
      // 输入时间 s 满足 t = s·(1 + ratio0) + slope·s²/2。
      let s = if a.abs() < 1e-18 {
        t / b
      } else {
        (-b + (b * b + 4.0 * a * t).sqrt()) / (2.0 * a)
      };
      let p = s * rate;
      let k = p.floor() as usize;
      let f = (p - p.floor()) as f32;
      let u = x[k.min(last)];
      let v = x[(k + 1).min(last)];
      u + (v - u) * f
    })
    .collect()
}

/// 带线性频漂的单音序列：第 `n` 个符号的频率是 `freqs_hz[n]`，整体再加 `hz_per_s × 秒数`，相位连续。
///
/// `samples_per_symbol` 是（可以带小数的）符号长度。各调制自己算每个符号的音调频率，这里只管合成，
/// 用来造「发射机边发边漂」的信号（常规发端的音调频率是常数）。
#[must_use]
pub fn synth_drifting(
  freqs_hz: &[f64],
  rate: u32,
  samples_per_symbol: f64,
  hz_per_s: f64,
) -> Vec<f32> {
  let rate = f64::from(rate);
  let mut out = Vec::new();
  let mut phase = 0.0f64;
  for (n, &freq) in freqs_hz.iter().enumerate() {
    let end = ((n + 1) as f64 * samples_per_symbol).round() as usize;
    while out.len() < end {
      let secs = out.len() as f64 / rate;
      phase += std::f64::consts::TAU * (freq + hz_per_s * secs) / rate;
      phase %= std::f64::consts::TAU;
      out.push(phase.cos() as f32);
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::{stretch, synth_drifting, tone_power, warp};

  fn sine(freq: f64, rate: u32, n: usize) -> Vec<f32> {
    (0..n)
      .map(|i| (2.0 * std::f64::consts::PI * freq * i as f64 / f64::from(rate)).sin() as f32)
      .collect()
  }

  #[test]
  fn peaks_at_the_tone_frequency() {
    let x = sine(1000.0, 8000, 800);
    let on = tone_power(&x, 8000, 1000.0);
    let off = tone_power(&x, 8000, 1500.0);
    assert!(on > off * 100.0, "on={on} off={off}");
  }

  #[test]
  fn silence_has_no_power() {
    assert_eq!(tone_power(&[0.0; 256], 8000, 1000.0), 0.0);
    assert_eq!(tone_power(&[], 8000, 1000.0), 0.0);
  }

  #[test]
  fn power_scales_with_amplitude_squared() {
    let a = sine(700.0, 8000, 800);
    let b: Vec<f32> = a.iter().map(|v| v * 2.0).collect();
    let ratio = tone_power(&b, 8000, 700.0) / tone_power(&a, 8000, 700.0);
    assert!((ratio - 4.0).abs() < 1e-6, "{ratio}");
  }

  #[test]
  fn add_noise_is_seeded_and_bounded() {
    let mut a = vec![0.0f32; 100];
    let mut b = vec![0.0f32; 100];
    super::add_noise(&mut a, 0.1);
    super::add_noise(&mut b, 0.1);
    // 固定种子：两次结果完全一致。
    assert_eq!(a, b);
    // 幅度在 [-amp, amp] 内，且不全是 0。
    assert!(a.iter().all(|&v| v.abs() <= 0.1 + 1e-6));
    assert!(a.iter().any(|&v| v != 0.0));
  }

  #[test]
  fn stretch_scales_the_frequency_by_the_inverse_ratio() {
    // 拉长 1.25 倍：1000 Hz 的音变成 800 Hz。
    let y = stretch(&sine(1000.0, 8000, 8000), 1.25);
    assert_eq!(y.len(), 10_000);
    let (on, off) = (tone_power(&y, 8000, 800.0), tone_power(&y, 8000, 1000.0));
    assert!(on > off * 100.0, "on={on} off={off}");
  }

  #[test]
  fn stretch_and_warp_reject_degenerate_input() {
    for ratio in [0.0, -1.0, f64::NAN, f64::INFINITY] {
      assert!(stretch(&[1.0], ratio).is_empty(), "ratio {ratio}");
    }
    assert!(stretch(&[], 1.0).is_empty());
    assert!(warp(&[], 8000.0, 0.0, 0.0).is_empty());
    assert!(warp(&[1.0], 8000.0, -1.0, 0.0).is_empty());
    assert!(warp(&[1.0], 0.0, 0.0, 0.0).is_empty());
    assert!(warp(&[1.0], 8000.0, 0.0, f64::NAN).is_empty());
  }

  #[test]
  fn warp_without_slope_is_a_plain_stretch() {
    let x = sine(900.0, 8000, 4000);
    let (a, b) = (warp(&x, 8000.0, 1e-3, 0.0), stretch(&x, 1.001));
    // 两种写法的长度最多差取整的 1 个样本。
    assert!(a.len().abs_diff(b.len()) <= 1, "{} vs {}", a.len(), b.len());
    for (u, v) in a.iter().zip(&b) {
      assert!((u - v).abs() < 1e-4);
    }
  }

  #[test]
  fn warp_slope_makes_the_tail_run_slower_than_the_head() {
    // slope > 0：转速越来越慢，输出比「恒为 1 + ratio0」更长。
    let x = sine(1000.0, 8000, 80_000);
    assert!(warp(&x, 8000.0, 0.0, 2e-5).len() > x.len());
    assert!(warp(&x, 8000.0, 0.0, -2e-5).len() < x.len());
  }

  #[test]
  fn synth_drifting_follows_the_requested_frequency_ramp() {
    // 三个符号各 1 秒、频率 1000 Hz、每秒漂 +100 Hz：开头 0.25 秒在 1000–1025 Hz，最后 0.25 秒在 1275–1300 Hz。
    let y = synth_drifting(&[1000.0; 3], 8000, 8000.0, 100.0);
    assert_eq!(y.len(), 24_000);
    let head = &y[..2000];
    let tail = &y[22_000..];
    assert!(tone_power(head, 8000, 1000.0) > tone_power(head, 8000, 1300.0) * 20.0);
    assert!(tone_power(tail, 8000, 1287.0) > tone_power(tail, 8000, 1000.0) * 20.0);
    assert!(synth_drifting(&[], 8000, 8000.0, 0.0).is_empty());
  }
}
