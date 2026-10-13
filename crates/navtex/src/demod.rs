//! FSK 解调与音调自动估计。
//!
//! NAVTEX 是 100 波特、170 Hz 频移的 FSK；但真实录音的电台调谐各不相同
//! （SigidWiki 的两段样本音调分别在 2110/2310 与 1904/2104 Hz），所以解调前
//! 先从频谱估计两个音调，而不是硬编码标准频率。

use ham_web_dsp::tone_power;

/// 第一个「够响」的样本下标（跳过开头的静音 / 静噪载波）。
///
/// 音调估计与相位同步都要从这里开始：把静音段算进去会同时毁掉两者。
fn first_signal_index(samples: &[f32]) -> Option<usize> {
  // 阈值取整段峰值的 5%，而不是固定 0.02：低电平录音（整体峰值不到 0.02，SDR 导出常见）
  // 在绝对阈值下会被整段判成「没有信号」。绝对下限 1e-4 约等于 16 位量化噪声，
  // 挡住全零 / 近乎全零的文件。
  let peak = samples.iter().fold(0.0f32, |m, &x| m.max(x.abs()));
  if !peak.is_finite() || peak < 1e-4 {
    return None;
  }
  let threshold = peak * 0.05;
  samples.iter().position(|&x| x.abs() > threshold)
}

/// 在频谱上找两个最强的音调峰（带限 300–3600 Hz，两者间隔 100–500 Hz），
/// 返回 `(较低频率, 较高频率)` —— NAVTEX 的 Mark 低于 Space。
///
/// 用 1 秒窗口 + 1 Hz 步进的 DFT 峰搜索；噪声与语音录音可能给出任意峰，
/// 调用方要拿结果去解调并对拍（权重 4 校验会兜底）。
///
/// 步进必须是 1 Hz：纯音在整数 bin 上其他 bin 严格为零，5 Hz 网格会把
/// 1904 / 2104 这类非 5 倍频的音调整个错过（实测教训）。
#[must_use]
pub fn estimate_tones(samples: &[f32], rate: u32) -> Option<(f64, f64)> {
  if rate < 800 {
    return None;
  }
  let n = rate as usize; // 1 秒窗口
  // 从第一个「够响」的样本起取窗口：录音开头常有超过 1 秒的静音 / 静噪载波，
  // 从文件起点取会估出错误的音调（甚至直接判成没信号）。不足 1 秒时用剩下的部分
  // 降级估计（峰值位置不变，只是分辨率变粗），太短则没有意义。
  let first = first_signal_index(samples)?;
  let xs = samples.get(first..)?;
  let xs = &xs[..xs.len().min(n)];
  if xs.len() < n / 4 {
    return None;
  }
  let lo = 300usize;
  // 2× 电话信道带宽上限；rate 至少要到 7202 Hz 才到 3600，实际 8 kHz 起。
  let hi = (3600usize).min(rate as usize / 2 - 1);
  // 每个 bin 用 Goertzel 递推（一次 cos，内层零三角函数）替代直接 DFT：
  // 3300 bin × 8000 样本 × (cos+sin) ≈ 5280 万次三角函数 → 免掉。
  let powers: Vec<(usize, f64)> = (lo..hi)
    .map(|k| (k, tone_power(xs, rate, k as f64)))
    .collect();
  // 非极大值抑制式的选峰：先取全局最强的 bin，再只在与它间隔 100–500 Hz 的 bin 里找第二个。
  // 旧做法取功率最高的 6 个 bin 再配对 —— 强音调两侧相邻的 bin 功率与主峰几乎相同，会把 6 个
  // 名额占满，Mark / Space 不平衡（选频衰落、接收机滤波器斜坡）时第二个音调根本进不了前 6。
  let &(k1, p1) = powers
    .iter()
    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))?;
  if p1 <= 0.0 || !p1.is_finite() {
    return None;
  }
  // 第二个音调至少要有主峰的 1/1000（−30 dB）：低于它的只是泄漏 / 噪底。
  let &(k2, _) = powers
    .iter()
    .filter(|&&(k, p)| (100..=500).contains(&k.abs_diff(k1)) && p >= p1 * 1e-3)
    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))?;
  let (lo_f, hi_f) = if k1 < k2 { (k1, k2) } else { (k2, k1) };
  Some((lo_f as f64, hi_f as f64))
}

/// 抽取后的目标采样率下限：NAVTEX 音调带限 3.6 kHz，12 kHz 仍留足余量。
const DECIMATED_MIN_RATE: u32 = 12_000;

/// 高采样率输入抽取到 ≥ [`DECIMATED_MIN_RATE`]（只取能整除的因子，采样率不取整漂移）。
///
/// 用 `factor` 点滑动平均当低通：对 3.6 kHz 以下的音调衰减 ≤ 3 dB，且对 Mark / Space
/// 近似对称；足以压住折叠进来的带外噪声。没有合适因子（或本来就不高）时原样返回。
#[must_use]
pub fn decimate_for_demod(samples: &[f32], rate: u32) -> (std::borrow::Cow<'_, [f32]>, u32) {
  let max_factor = rate / DECIMATED_MIN_RATE;
  let factor = (2..=max_factor).rev().find(|f| rate.is_multiple_of(*f));
  let Some(factor) = factor else {
    return (std::borrow::Cow::Borrowed(samples), rate);
  };
  let n = factor as usize;
  let out: Vec<f32> = samples
    .chunks_exact(n)
    .map(|c| c.iter().sum::<f32>() / n as f32)
    .collect();
  (std::borrow::Cow::Owned(out), rate / factor)
}

/// 解调所需的最低采样率：100 波特下每符号至少 1 个样本。
const MIN_DEMOD_RATE: u32 = 100;

/// 100 波特 FSK 解调：每个符号窗口比较 Mark / Space 的 Goertzel 功率。
///
/// 符号网格 = `rate / 100` 个样本（非整采样率按四舍五入累进，不漂移）。
/// **从 `offset` 开始**：真实录音开头常有静音 / 前导段，符号边界与文件起点
/// 不对齐 —— 用 [`sync_phase`] 找到相位后再解调（SigidWiki 样本当场教训：
/// 相位错半个符号时每个窗口都跨两个音，决策全废）。
#[must_use]
pub fn demodulate_at(
  samples: &[f32],
  rate: u32,
  mark_hz: f64,
  space_hz: f64,
  offset: usize,
) -> Vec<bool> {
  // rate=0 → spb=0 → `start += 0` 死循环；`rate < 100` 时 spb < 1，`start.round()` 可越过
  // `samples.len()` 使切片 panic（公共 API 也要防，不能只靠 `estimate_tones` 的守卫）；
  // 非有限频率会让 Goertzel 全 NaN。
  if rate < MIN_DEMOD_RATE || samples.is_empty() || !mark_hz.is_finite() || !space_hz.is_finite() {
    return Vec::new();
  }
  let spb = f64::from(rate) / 100.0;
  let mut out = Vec::with_capacity((samples.len() as f64 / spb) as usize);
  let mut start = offset as f64;
  while (start + spb) as usize <= samples.len() {
    let from = start.round() as usize;
    let to = (start + spb).round() as usize;
    let window = &samples[from..to.min(samples.len())];
    out.push(tone_power(window, rate, mark_hz) >= tone_power(window, rate, space_hz));
    start += spb;
  }
  out
}

/// 符号相位搜索：返回**绝对样本偏移**（供 [`demodulate_at`] 直接使用）。
///
/// 对比度 = |mark - space| / (mark + space) 逐符号求和：窗口落在单一音调上时趋近 1，
/// 跨两个音调时趋近 0，且两条音调得到对称、有界的评分。
/// 先跳过开头静音（第一个幅度超过 `noise_floor` 的样本）再取 ~2 秒窗口。
///
/// 注意：窗口恰落单一音调时，另一个音调的 Goertzel 功率**几乎为零**（频率差
/// 200 Hz × 80 样本恰好 2 个整周期，正交）—— 不能用 `ps > 阈值` 把这种窗口
/// 过滤掉，否则会把对齐得最好的窗口全扔掉（单测当场抓到）。
#[must_use]
pub fn sync_phase(samples: &[f32], rate: u32, mark_hz: f64, space_hz: f64) -> usize {
  if rate < MIN_DEMOD_RATE {
    return 0;
  }
  let spb = (f64::from(rate) / 100.0).round() as usize;
  let Some(first) = first_signal_index(samples) else {
    return 0;
  };
  // wasm32 上 `usize` 是 32 位：`2 * rate` 对超大采样率头会回绕。
  let window = (rate as usize)
    .saturating_mul(2)
    .min(samples.len().saturating_sub(first));
  if window < spb * 4 {
    return 0;
  }
  // rel 可为负：`first` 是第一个「够响」的样本，真正的符号边界可能在它之前
  // 半个符号内（信号从 sin(0)=0 起步时尤其如此）。
  let mut best = (0isize, f64::NEG_INFINITY);
  for rel in -(spb as isize)..(spb as isize) {
    let start = first as isize + rel;
    if start < 0 {
      continue;
    }
    let mut contrast = 0.0f64;
    let mut i = start as usize;
    while i + spb <= first + window {
      let w = &samples[i..i + spb];
      let pm = tone_power(w, rate, mark_hz);
      let ps = tone_power(w, rate, space_hz);
      if pm + ps > 1e-12 {
        contrast += (pm - ps).abs() / (pm + ps);
      }
      i += spb;
    }
    // PCM 量化后等价边界的评分会相差约 1e-7；保留较早边界，避免移开
    // 起始零样本后丢掉录音末尾的最后一个完整位窗。
    if contrast > best.1 + 1e-6 {
      best = (rel, contrast);
    }
  }
  (first as isize + best.0) as usize
}

/// 便捷入口：先同步相位再解调。
#[must_use]
pub fn demodulate(samples: &[f32], rate: u32, mark_hz: f64, space_hz: f64) -> Vec<bool> {
  let offset = sync_phase(samples, rate, mark_hz, space_hz);
  demodulate_at(samples, rate, mark_hz, space_hz, offset)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn unusable_sample_rates_are_rejected_without_underflow() {
    for rate in [0, 1, 2, 49, 100, 799] {
      assert!(estimate_tones(&[0.0; 800], rate).is_none());
    }
  }

  #[test]
  fn demodulate_rejects_rates_below_one_sample_per_symbol() {
    // R2-04：公开的 demodulate / demodulate_at / sync_phase 在 rate 1–99 时曾可越界 panic。
    let samples = vec![0.5f32; 4096];
    for rate in [0, 1, 2, 49, 50, 99] {
      assert!(
        demodulate(&samples, rate, 1904.0, 2104.0).is_empty(),
        "rate={rate}"
      );
      assert!(demodulate_at(&samples, rate, 1904.0, 2104.0, 7).is_empty());
      assert_eq!(sync_phase(&samples, rate, 1904.0, 2104.0), 0);
    }
  }

  #[test]
  fn decimation_keeps_low_rates_and_reduces_high_ones() {
    let s = vec![0.25f32; 4800];
    for rate in [0, 1, 8000, 11_025, 12_000, 16_000, 22_050] {
      let (out, r) = decimate_for_demod(&s, rate);
      assert_eq!((out.len(), r), (s.len(), rate), "rate={rate}");
    }
    for (rate, expect) in [(48_000, 12_000), (44_100, 14_700), (96_000, 12_000)] {
      let (out, r) = decimate_for_demod(&s, rate);
      assert_eq!(r, expect, "rate={rate}");
      assert_eq!(out.len(), s.len() / (rate / expect) as usize);
      assert!(out.iter().all(|&x| (x - 0.25).abs() < 1e-6));
    }
  }

  #[test]
  fn demodulate_at_survives_huge_offset() {
    let samples = vec![0.5f32; 4096];
    assert!(demodulate_at(&samples, 8000, 1904.0, 2104.0, usize::MAX).is_empty());
  }

  fn synth_fsk(rate: u32, mark: f64, space: f64, bits: &[bool], noise: f32) -> Vec<f32> {
    let spb = (f64::from(rate) / 100.0).round() as usize;
    let mut out = Vec::new();
    let mut seed = 0x1234_5678u32;
    for &b in bits {
      let f = if b { mark } else { space };
      for _ in 0..spb {
        let t = out.len() as f64 / f64::from(rate);
        let mut v = (2.0 * std::f64::consts::PI * f * t).sin() as f32;
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let u = (seed >> 8) as f32 / 16_777_216.0;
        v += noise * (u * 2.0 - 1.0);
        out.push(v);
      }
    }
    out
  }

  #[test]
  fn demod_recovers_bits_with_noise() {
    let pattern = [
      true, false, true, true, false, false, true, false, true, true,
    ];
    let samples = synth_fsk(8000, 1904.0, 2104.0, &pattern, 0.03);
    let bits = demodulate(&samples, 8000, 1904.0, 2104.0);
    assert_eq!(bits.len(), pattern.len());
    for (i, (&a, &b)) in pattern.iter().zip(bits.iter()).enumerate() {
      assert_eq!(a, b, "第 {i} 个符号");
    }
  }

  #[test]
  fn tone_estimation_finds_the_fsk_pair() {
    let bits: Vec<bool> = (0..100).map(|i| i % 3 != 0).collect();
    let samples = synth_fsk(8000, 1904.0, 2104.0, &bits, 0.02);
    let (lo, hi) = estimate_tones(&samples, 8000).expect("应找到音调对");
    assert!((lo - 1904.0).abs() < 10.0, "{lo}");
    assert!((hi - 2104.0).abs() < 10.0, "{hi}");
    assert!(lo < hi);
  }

  #[test]
  fn tone_estimation_survives_unbalanced_tones() {
    // Mark 与 Space 相差 18 dB（选频衰落 / 接收机滤波器斜坡）：弱音调不能被强音调
    // 两侧相邻 bin 挤出候选。
    let samples: Vec<f32> = (0..8000)
      .map(|i| {
        let t = f64::from(i) / 8000.0;
        let m = (2.0 * std::f64::consts::PI * 1904.0 * t).sin();
        let s = (2.0 * std::f64::consts::PI * 2104.0 * t).sin();
        (0.5 * m + 0.06 * s) as f32
      })
      .collect();
    let (lo, hi) = estimate_tones(&samples, 8000).expect("弱音调也应被找到");
    assert_eq!((lo, hi), (1904.0, 2104.0));
  }

  #[test]
  fn low_level_recordings_still_find_the_tones() {
    // 整体峰值只有 0.01：旧的绝对阈值 0.02 会把它整段当静音。
    let bits: Vec<bool> = (0..100).map(|i| i % 3 != 0).collect();
    let quiet: Vec<f32> = synth_fsk(8000, 1904.0, 2104.0, &bits, 0.0)
      .iter()
      .map(|x| x * 0.01)
      .collect();
    // 峰值必须低于旧的绝对阈值 0.02，否则这个用例证明不了什么。
    assert!(quiet.iter().all(|x| x.abs() < 0.02));
    let (lo, hi) = estimate_tones(&quiet, 8000).expect("低电平也应找到音调");
    assert!(
      (lo - 1904.0).abs() < 10.0 && (hi - 2104.0).abs() < 10.0,
      "{lo} {hi}"
    );
  }

  #[test]
  fn tone_estimation_rejects_a_single_tone() {
    let samples: Vec<f32> = (0..8000)
      .map(|i| (2.0 * std::f64::consts::PI * 1904.0 * f64::from(i) / 8000.0).sin() as f32 * 0.5)
      .collect();
    assert!(estimate_tones(&samples, 8000).is_none());
  }

  #[test]
  fn tone_estimation_rejects_silence() {
    assert!(estimate_tones(&[0.0f32; 8000], 8000).is_none());
  }

  #[test]
  fn tone_estimation_skips_leading_silence() {
    // 前导静音超过 1 秒（真实录音常见）时，仍要从信号段估计音调，而不是把静音
    // 当作「没有信号」。
    let bits: Vec<bool> = (0..100).map(|i| i % 3 != 0).collect();
    let mut samples = vec![0.0f32; 8000 * 2];
    samples.extend(synth_fsk(8000, 1904.0, 2104.0, &bits, 0.02));
    let (lo, hi) = estimate_tones(&samples, 8000).expect("应找到音调对");
    assert!((lo - 1904.0).abs() < 10.0, "{lo}");
    assert!((hi - 2104.0).abs() < 10.0, "{hi}");
  }

  #[test]
  fn sync_phase_recovers_alignment_after_leading_silence() {
    let pattern = [
      true, false, true, true, false, false, true, false, true, true,
    ];
    let signal = synth_fsk(8000, 1904.0, 2104.0, &pattern, 0.01);
    // 前导静音 37 个样本：符号边界与文件起点错位 37 mod 80。
    let mut samples = vec![0.0f32; 37];
    samples.extend_from_slice(&signal);
    let offset = sync_phase(&samples, 8000, 1904.0, 2104.0);
    assert_eq!(offset, 37, "同步应找回 37 样本的错位");
    let bits = demodulate_at(&samples, 8000, 1904.0, 2104.0, offset);
    // 37 + 信号长度 → 有效符号 = 模式长度（尾巴不足一个符号的被丢弃）。
    assert_eq!(bits.len(), pattern.len());
    for (i, (&a, &b)) in pattern.iter().zip(bits.iter()).enumerate() {
      assert_eq!(a, b, "第 {i} 个符号");
    }
  }

  #[test]
  fn misaligned_windows_produce_garbage_but_sync_fixes_it() {
    let pattern = [
      true, true, false, true, false, false, true, false, false, true,
    ];
    let signal = synth_fsk(8000, 1904.0, 2104.0, &pattern, 0.01);
    // 半符号（40 样本）错位：未同步的解调会全错。
    let mut samples = vec![0.0f32; 40];
    samples.extend_from_slice(&signal);
    let offset = sync_phase(&samples, 8000, 1904.0, 2104.0);
    let bits = demodulate_at(&samples, 8000, 1904.0, 2104.0, offset);
    for (i, (&a, &b)) in pattern.iter().zip(bits.iter()).enumerate() {
      assert_eq!(a, b, "第 {i} 个符号");
    }
  }
}
