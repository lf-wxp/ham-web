//! 解码页对「录音时钟误差 / 频漂跟踪」结果的呈现口径。
//!
//! MFSK / THOR 解码器会把录音切块、拟合平滑曲线来修正时钟误差与频漂；没有跟踪到时两个量
//! **精确为 0**（见 `ham_web_mfsk::track`）。页面只在确实修正过时才多写一行，所以这里决定
//! 「取整到显示精度后还剩不剩东西」—— 干净录音一个字都不该多出来，不能写成「0 ppm」。

/// 时钟误差的显示精度：取整到 1 ppm。
const PPM_STEP: f64 = 1.0;
/// 频漂的显示精度：取整到 0.01 Hz/s（对拍里估值与注入值相差 < 0.02 Hz/s，再细没有意义）。
const DRIFT_STEP: f64 = 0.01;

/// 取整到显示精度后仍非零的跟踪结果；对应项为 `None` 表示该项没有修正。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tracking {
  /// 符号周期相对名义值的偏差（ppm），正 = 录音时钟偏慢。
  pub clock_ppm: Option<f64>,
  /// 频漂（Hz / 秒），正 = 越往后频率越高。
  pub drift_hz_per_s: Option<f64>,
}

/// 把解码器回报的两个量整理成可展示的结果；两项取整后都是 0（或非有限值）时返回 `None`。
#[must_use]
pub fn summarize(clock_ppm: f64, drift_hz_per_s: f64) -> Option<Tracking> {
  let clock_ppm = displayed(clock_ppm, PPM_STEP);
  let drift_hz_per_s = displayed(drift_hz_per_s, DRIFT_STEP);
  (clock_ppm.is_some() || drift_hz_per_s.is_some()).then_some(Tracking {
    clock_ppm,
    drift_hz_per_s,
  })
}

/// 取整到 `step` 的整数倍；取整后为 0 或不是有限数就当没有。
fn displayed(value: f64, step: f64) -> Option<f64> {
  if !value.is_finite() {
    return None;
  }
  let rounded = (value / step).round() * step;
  // 取整后的 -0.0 也是 0，必须落到 None 一侧，否则页面会写出「-0 ppm」。
  (rounded != 0.0).then_some(rounded)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn untracked_recording_shows_nothing() {
    // 解码器在没有跟踪时回报的就是精确的 0。
    assert_eq!(summarize(0.0, 0.0), None);
  }

  #[test]
  fn values_that_round_to_zero_are_hidden() {
    assert_eq!(summarize(0.4, 0.004), None);
    assert_eq!(summarize(-0.4, -0.004), None);
  }

  #[test]
  fn non_finite_values_are_ignored() {
    assert_eq!(summarize(f64::NAN, f64::INFINITY), None);
    let t = summarize(f64::NAN, 0.5).expect("频漂仍应显示");
    assert_eq!(t.clock_ppm, None);
    assert_eq!(t.drift_hz_per_s, Some(0.5));
  }

  #[test]
  fn each_quantity_is_reported_independently() {
    let clock_only = summarize(1000.2, 0.0).unwrap();
    assert_eq!(clock_only.clock_ppm, Some(1000.0));
    assert_eq!(clock_only.drift_hz_per_s, None);

    let drift_only = summarize(0.0, -0.034).unwrap();
    assert_eq!(drift_only.clock_ppm, None);
    // 0.01 的整数倍，容许浮点乘法的末位误差。
    assert!((drift_only.drift_hz_per_s.unwrap() + 0.03).abs() < 1e-12);
  }

  #[test]
  fn sign_is_kept() {
    let t = summarize(-1500.0, -0.3).unwrap();
    assert_eq!(t.clock_ppm, Some(-1500.0));
    assert!((t.drift_hz_per_s.unwrap() + 0.3).abs() < 1e-12);
  }

  #[test]
  fn rounding_boundary_is_half_away_from_zero() {
    assert_eq!(summarize(0.5, 0.0).unwrap().clock_ppm, Some(1.0));
    assert_eq!(summarize(-0.5, 0.0).unwrap().clock_ppm, Some(-1.0));
  }
}
