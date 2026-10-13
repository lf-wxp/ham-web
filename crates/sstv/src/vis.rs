//! VIS（Vertical Interval Signaling）头检测：识别 SSTV 模式。
//!
//! 真实信号的头部是：
//!
//! ```text
//! [VOX 音调序列（可选）] 引导音 1900 Hz 300 ms │ 间隔 1200 Hz 10 ms │ 引导音 1900 Hz 300 ms
//!   │ 起始位 1200 Hz 30 ms │ 8 位 × 30 ms（**1100 Hz = 逻辑 1 / 1300 Hz = 逻辑 0**，LSB 先发，
//!   第 8 位为偶校验）│ 停止位 1200 Hz 30 ms │ 图像第一行
//! ```
//!
//! 此前的代码有**两处**与真实信号不符，各自都足以让真实录音检测失败：
//!
//! 1. 起始位只有 **30 ms**，代码把它当成 300 ms；
//! 2. 位定义写反了（`1100 = 0 / 1300 = 1`）。实测 Wikimedia Commons 的 Martin M1 样本，8 位读数
//!    `1300 1299 1101 1101 1300 1099 1300 1101` 取反后恰好是 44（Martin M1 的 VIS）+ 校验位 1。
//!
//! 编码器与解码器共用同一份错误假设，所以自洽测试一直是绿的 —— 它们互相验证了彼此的错误。

use super::WORK_RATE;
use super::modes;

/// 每个 VIS 位时长（毫秒）。
const BIT_MS: f64 = 30.0;
/// 起始位 / 停止位下限与上限（毫秒）：标称 30 ms，容忍 FM 解调瞬态吃掉的边缘。
const START_MIN_MS: f64 = 20.0;
const START_MAX_MS: f64 = 45.0;

fn ms_to_samples(ms: f64) -> usize {
  (ms / 1000.0 * f64::from(WORK_RATE)).round() as usize
}

/// 一次成功的 VIS 检测。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vis {
  /// 7 位数据码。
  pub code: u8,
  /// VIS 结束（停止位结束）的样本位置：此后紧接着图像第一行。
  pub end: usize,
}

/// 滑动中值滤波：去掉 FM 解调的单点毛刺，同时保留 30 ms 量级的台阶边沿。
pub(crate) fn median_filter(freq: &[f32], window: usize) -> Vec<f32> {
  let half = window / 2;
  let mut buf = Vec::with_capacity(window);
  (0..freq.len())
    .map(|i| {
      buf.clear();
      buf.extend_from_slice(&freq[i.saturating_sub(half)..(i + half + 1).min(freq.len())]);
      buf.sort_unstable_by(f32::total_cmp);
      buf[buf.len() / 2]
    })
    .collect()
}

/// 所有 1200 Hz 持续段 `(起点, 长度)`，长度在 `[min, max]` 内。
fn sync_runs(freq: &[f32], min: usize, max: usize) -> Vec<(usize, usize)> {
  let mut out = Vec::new();
  let mut run = 0usize;
  for (i, &f) in freq.iter().enumerate() {
    if (1130.0..=1260.0).contains(&f) {
      run += 1;
    } else {
      if (min..=max).contains(&run) {
        out.push((i - run, run));
      }
      run = 0;
    }
  }
  if (min..=max).contains(&run) {
    out.push((freq.len() - run, run));
  }
  out
}

/// 检测 VIS 码。
///
/// 候选 = 任何 20 ~ 45 ms 的 1200 Hz 持续段（起始位）。逐个验证：8 个数据位读得出、**偶校验
/// 通过**、数据码对应一个已知模式；取时间上第一个通过的。不依赖引导音，所以被截掉前导的录音
/// 也能识别；偶校验加「必须是已知模式」使随机命中的概率很低。
#[must_use]
pub fn detect(freq: &[f32]) -> Option<Vis> {
  let m = median_filter(freq, 9);
  let bit = ms_to_samples(BIT_MS);
  let (lo, hi) = (ms_to_samples(START_MIN_MS), ms_to_samples(START_MAX_MS));
  for (start, _run) in sync_runs(&m, lo, hi) {
    // 起始位的**起点**是 FM 解调后上升沿的位置：用标称时长从起点推算后续各位，而不是从终点
    // 倒推（这段 1200 Hz 的长度会因边沿被 FM 解调吃掉而偏短）。
    let mut raw = 0u8;
    let mut ok = true;
    for b in 0..8 {
      // 每位只取中间 60%：位与位之间的跳变区有 FM 解调的过渡。
      let from = start + bit * (b + 1) + bit / 5;
      let to = start + bit * (b + 2) - bit / 5;
      if to > m.len() {
        ok = false;
        break;
      }
      let avg = m[from..to].iter().sum::<f32>() / (to - from) as f32;
      // 1100 Hz = 逻辑 1，1300 Hz = 逻辑 0（与直觉相反，见模块文档）；
      // 偏离两个标称值太远说明不是 VIS 位。
      if !(1000.0..=1400.0).contains(&avg) {
        ok = false;
        break;
      }
      if avg < 1200.0 {
        raw |= 1 << b;
      }
    }
    if !ok || !raw.count_ones().is_multiple_of(2) {
      continue; // 偶校验：8 位里 1 的个数必须是偶数
    }
    let code = raw & 0x7F;
    if modes::by_vis_code(code).is_none() {
      continue;
    }
    return Some(Vis {
      code,
      end: start + bit * 10,
    });
  }
  None
}

#[cfg(test)]
mod tests {
  use super::*;

  fn tone(out: &mut Vec<f32>, hz: f32, ms: f64) {
    out.extend(std::iter::repeat_n(hz, ms_to_samples(ms)));
  }

  /// 按真实规格合成头部：引导音 / 间隔 / 引导音 / 起始位 / 8 位 / 停止位。
  fn synth_vis(code: u8) -> Vec<f32> {
    let mut out = Vec::new();
    tone(&mut out, 1900.0, 300.0);
    tone(&mut out, 1200.0, 10.0);
    tone(&mut out, 1900.0, 300.0);
    tone(&mut out, 1200.0, 30.0);
    let full = code | (((code.count_ones() % 2) as u8) << 7);
    for b in 0..8 {
      tone(
        &mut out,
        if (full >> b) & 1 == 1 { 1100.0 } else { 1300.0 },
        30.0,
      );
    }
    tone(&mut out, 1200.0, 30.0);
    out
  }

  #[test]
  fn detects_every_supported_vis_code() {
    for mode in modes::MODES {
      let freq = synth_vis(mode.vis_code);
      let v = detect(&freq).unwrap_or_else(|| panic!("{} 未检测到", mode.name));
      assert_eq!(v.code, mode.vis_code, "{}", mode.name);
      // 起始位起点 = 300 + 10 + 300 = 610 ms；VIS 共 300 ms，结束在 910 ms。
      let want = ms_to_samples(910.0);
      assert!(
        v.end.abs_diff(want) <= 12,
        "{}：end {} 应约 {}",
        mode.name,
        v.end,
        want
      );
    }
  }

  #[test]
  fn the_10ms_break_is_not_mistaken_for_a_start_bit() {
    // 间隔只有 10 ms（< 20 ms 下限），不能被当成起始位；真起始位在它之后。
    let freq = synth_vis(44);
    let v = detect(&freq).expect("应检测到");
    assert!(v.end > ms_to_samples(900.0));
  }

  #[test]
  fn works_without_a_leader() {
    // 被截掉前导的录音：只剩 起始位 + 8 位 + 停止位。
    let full = synth_vis(60);
    let cut = &full[ms_to_samples(610.0)..];
    assert_eq!(detect(cut).map(|v| v.code), Some(60));
  }

  #[test]
  fn rejects_a_parity_error() {
    let mut freq = synth_vis(44);
    // 翻转第 8 位（bit 7 = 校验位，起始位之后第 8 个 30 ms 窗口）：偶校验失败 → 拒收，
    // 而不是返回一个错误的模式。
    let bit7 = ms_to_samples(610.0 + 30.0 * 8.0);
    for f in &mut freq[bit7..bit7 + ms_to_samples(30.0)] {
      *f = if *f > 1200.0 { 1100.0 } else { 1300.0 };
    }
    assert!(detect(&freq).is_none());
  }

  #[test]
  fn rejects_unknown_codes() {
    // 7 位码 1 不对应任何已支持模式（即使校验正确）。
    assert!(detect(&synth_vis(1)).is_none());
  }

  #[test]
  fn bit_polarity_matches_a_real_martin_m1_recording() {
    // 真实录音（Wikimedia Commons「SSTV sunset audio」）里量到的 8 个位均值，先发在前。
    // 1100 Hz = 逻辑 1：取反后 = 0,0,1,1,0,1,0,1 → LSB 先发 → 数据 44（Martin M1）+ 校验位 1。
    let real = [
      1300.0f32, 1299.0, 1101.0, 1101.0, 1300.0, 1099.0, 1300.0, 1101.0,
    ];
    let mut freq = Vec::new();
    tone(&mut freq, 1900.0, 300.0);
    tone(&mut freq, 1200.0, 30.0);
    for f in real {
      tone(&mut freq, f, 30.0);
    }
    tone(&mut freq, 1200.0, 30.0);
    assert_eq!(detect(&freq).map(|v| v.code), Some(44));
  }

  #[test]
  fn none_on_plain_tone() {
    assert!(detect(&vec![1900.0f32; WORK_RATE as usize]).is_none());
    assert!(detect(&[]).is_none());
  }

  #[test]
  fn survives_single_sample_glitches() {
    let mut freq = synth_vis(12);
    // 在各位中间撒毛刺（FM 解调在过零附近的典型尖峰）。
    for k in (ms_to_samples(630.0)..freq.len()).step_by(37) {
      freq[k] = 2900.0;
    }
    assert_eq!(detect(&freq).map(|v| v.code), Some(12));
  }
}
