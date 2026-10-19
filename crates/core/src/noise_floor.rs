//! 噪声本底估计：从一帧功率谱（dBFS）取低分位作为本底。
//!
//! 为什么用**分位**而不是均值：噪声本底测的是「安静时有多安静」，均值会被同频段的强
//! 信号直接拉高——有人说话的那一秒，均值能比本底高十几 dB。10% 分位对少量强信号不敏感，
//! 又比「最小值」抗偶发的陷波与量化底。
//!
//! 这里只做**相对**比较（同一条记录的前后差），不给绝对 dBm：Web Audio 的 dBFS 与天线端
//! 的场强之间隔着声卡增益、AGC 与校准，凭空换算出来的 dBm 是假的。绝对口径要等 ITU-R
//! P.372 的数值表（见 ROADMAP 的 S5）。

use serde::{Deserialize, Serialize};

/// 低于这个值视为无效 bin：Web Audio 在静音时会给 `-Infinity` 或极小值。
const MIN_VALID_DB: f32 = -200.0;
/// 判定「被占用」的门限：本底以上这么多 dB。
pub const OCCUPIED_MARGIN_DB: f32 = 6.0;
/// 本底取的分位。
pub const FLOOR_PERCENTILE: f32 = 0.10;
/// 太少的 bin 说明这一帧不可信（窗口还没填满）。
const MIN_BINS: usize = 8;

/// 一帧的本底估计。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Floor {
  /// 本底（dBFS）。
  pub floor_db: f32,
  /// 99% 分位，用于看动态范围。
  pub peak_db: f32,
  /// 高于「本底 + 6 dB」的 bin 占比（0–1）。
  ///
  /// 分母是**有效 bin 数**（剔掉 `-inf` / NaN / 量化底之后的条数），不是整帧长度：
  /// 一帧里若一半 bin 是数字静音，占比会按有效部分放大，这是刻意的口径。
  pub occupied_ratio: f32,
}

/// 一次记录。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sample {
  /// 用户给的标签（同一标签才会做前后比较）。
  pub label: String,
  /// Unix 毫秒。
  pub at_ms: f64,
  pub floor_db: f32,
  pub occupied_ratio: f32,
}

/// 估计一帧的本底；有效 bin 太少（或全是静音）时返回 `None`。
#[must_use]
pub fn estimate(spectrum_db: &[f32]) -> Option<Floor> {
  let mut valid: Vec<f32> = spectrum_db
    .iter()
    .copied()
    .filter(|v| v.is_finite() && *v > MIN_VALID_DB)
    .collect();
  if valid.len() < MIN_BINS {
    return None;
  }
  // 这是音频热路径（每帧一次）：整份排序是 O(n log n)，而只要取两个分位。
  // `select_nth_unstable_by` 是 O(n)，且不要求切片有序。
  let floor_db = rank(&mut valid, FLOOR_PERCENTILE);
  let peak_db = rank(&mut valid, 0.99);
  let threshold = floor_db + OCCUPIED_MARGIN_DB;
  let occupied = valid.iter().filter(|v| **v > threshold).count();
  #[allow(clippy::cast_precision_loss)]
  let occupied_ratio = occupied as f32 / valid.len() as f32;

  Some(Floor {
    floor_db,
    peak_db,
    occupied_ratio,
  })
}

/// 分位数（最近秩），按需重排切片；调用后元素顺序不再保证。
fn rank(values: &mut [f32], p: f32) -> f32 {
  if values.is_empty() {
    return 0.0;
  }
  #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
  let idx = (p.clamp(0.0, 1.0) * (values.len() - 1) as f32).round() as usize;
  let idx = idx.min(values.len() - 1);
  let (_, nth, _) = values.select_nth_unstable_by(idx, f32::total_cmp);
  *nth
}

/// 最后一条与**同标签里时刻最晚的另一条**的本底差（dB，负数表示更安静）。
///
/// 按 `at_ms` 而不是数组顺序取「上一条」：记录会经备份合并 / 导入恢复，数组顺序不等于
/// 时间顺序，按顺序取会拿一条更早的记录当「上次」。
#[must_use]
pub fn last_delta(samples: &[Sample]) -> Option<f32> {
  let last = samples.last()?;
  let prev = samples[..samples.len() - 1]
    .iter()
    .filter(|s| s.label == last.label)
    .max_by(|a, b| a.at_ms.total_cmp(&b.at_ms))?;
  Some(last.floor_db - prev.floor_db)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn sample(label: &str, floor_db: f32) -> Sample {
    Sample {
      label: label.to_owned(),
      at_ms: 0.0,
      floor_db,
      occupied_ratio: 0.0,
    }
  }

  #[test]
  fn flat_spectrum_has_no_occupancy() {
    let spectrum = vec![-80.0f32; 100];
    let f = estimate(&spectrum).expect("100 个有效 bin");
    assert!((f.floor_db + 80.0).abs() < 1e-6);
    assert!((f.peak_db + 80.0).abs() < 1e-6);
    assert_eq!(f.occupied_ratio, 0.0);
  }

  #[test]
  fn a_strong_half_raises_occupancy_but_not_the_floor() {
    let mut spectrum = vec![-80.0f32; 100];
    for v in spectrum.iter_mut().take(50) {
      *v = -40.0;
    }
    let f = estimate(&spectrum).expect("有效");
    // 10% 分位仍在安静的一半里。
    assert!((f.floor_db + 80.0).abs() < 1e-6, "{}", f.floor_db);
    assert!((f.occupied_ratio - 0.5).abs() < 1e-6);
  }

  #[test]
  fn a_single_tone_barely_moves_the_floor() {
    let mut spectrum = vec![-90.0f32; 100];
    spectrum[42] = -10.0;
    let f = estimate(&spectrum).expect("有效");
    assert!((f.floor_db + 90.0).abs() < 1e-6);
    assert!((f.occupied_ratio - 0.01).abs() < 1e-6);
  }

  #[test]
  fn silent_or_short_frames_have_no_estimate() {
    assert!(estimate(&[]).is_none());
    assert!(estimate(&[f32::NEG_INFINITY; 100]).is_none());
    assert!(estimate(&[-80.0; 7]).is_none());
  }

  #[test]
  fn delta_uses_timestamps_not_array_order() {
    // 数组顺序被备份合并打乱过：先放「时刻晚」的，再放「时刻早」的。按顺序取会选到
    // 时刻早的那条（-90 → 差值 -5），按 `at_ms` 取才是 -100 → 差值 +5。
    let mut late = sample("20m", -100.0);
    late.at_ms = 2_000.0;
    let mut early = sample("20m", -90.0);
    early.at_ms = 1_000.0;
    let mut newest = sample("20m", -95.0);
    newest.at_ms = 3_000.0;
    let delta = last_delta(&[late, early, newest]).expect("同标签有上一条");
    assert!((delta - 5.0).abs() < 1e-6, "{delta}");
  }

  #[test]
  fn delta_compares_against_the_same_label_only() {
    let samples = vec![
      sample("20m 夜", -100.0),
      sample("40m 夜", -95.0),
      sample("20m 夜", -97.0),
    ];
    let delta = last_delta(&samples).expect("同标签有上一条");
    assert!((delta - 3.0).abs() < 1e-6);
    // 只有一条时没有可比对象。
    assert!(last_delta(&[sample("20m 夜", -100.0)]).is_none());
    // 标签都不同。
    assert!(last_delta(&[sample("a", -100.0), sample("b", -90.0)]).is_none());
  }
}
