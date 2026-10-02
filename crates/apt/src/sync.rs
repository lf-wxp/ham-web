//! 行同步：在字速率包络上检测 sync A（1040 Hz）定位每行起点。
//!
//! 在 4160 Hz 采样下 sync A 为 4 样本/周期，采用与 `[1,0,-1,0]` / `[0,1,0,-1]`
//! 基准的 40 样本滑窗投影计算 1040 Hz 分量功率，峰值即行起点，再沿 0.5 秒
//! （2080 字）行周期锁定并逐行重锁。

use super::LINE_WORDS;

/// 行起点搜索窗口（字）：在预期位置 ±`SEARCH` 内重锁，容忍轻微时钟漂移。
const SEARCH: usize = 24;
/// sync A 检测窗长（字），10 个完整周期。
const WINDOW: usize = 40;

/// 检测每行起始位置（sync A 起点，字索引）。
pub fn find_lines(words: &[f32]) -> Vec<usize> {
  let power = sync_a_power(words);
  let max_power = power.iter().fold(0.0f32, |a, &v| a.max(v));
  if max_power < 1e-6 {
    return Vec::new();
  }
  let threshold = threshold(&power);

  // 候选：局部极大且超过阈值，间隔至少半行以避免同一 sync 内多次命中。
  let mut candidates = Vec::new();
  let mut i = 1usize;
  while i + 1 < power.len() {
    if power[i] > threshold && power[i] >= power[i - 1] && power[i] > power[i + 1] {
      candidates.push(i);
      i += LINE_WORDS / 2;
    } else {
      i += 1;
    }
  }

  let Some(&first) = candidates.first() else {
    return Vec::new();
  };

  // 沿行周期锁定，逐行在预期位置附近重锁最强 sync。
  let mut lines = vec![first];
  let mut cursor = first;
  while lines.len() < words.len() / LINE_WORDS + 4 {
    let expected = cursor + LINE_WORDS;
    if expected >= power.len() {
      break;
    }
    let lo = expected.saturating_sub(SEARCH);
    let hi = (expected + SEARCH).min(power.len() - 1);
    let mut best = expected;
    let mut best_v = f32::MIN;
    for (j, &v) in power.iter().enumerate().take(hi + 1).skip(lo) {
      if v > best_v {
        best_v = v;
        best = j;
      }
    }
    // 失锁时仍按预期推进，避免整段丢失。
    if best_v >= threshold {
      cursor = best;
    } else {
      cursor = expected;
    }
    lines.push(cursor);
  }
  lines
}

/// 计算每个位置上的 1040 Hz 分量功率（40 样本滑窗投影，O(n·WINDOW)）。
fn sync_a_power(words: &[f32]) -> Vec<f32> {
  let n = words.len();
  let mut power = vec![0.0f32; n];
  if n < WINDOW {
    return power;
  }
  for i in 0..=(n - WINDOW) {
    let mut c = 0.0f32;
    let mut s = 0.0f32;
    for k in 0..WINDOW {
      let v = words[i + k];
      match k & 3 {
        0 => c += v,
        1 => s += v,
        2 => c -= v,
        _ => s -= v,
      }
    }
    power[i] = c * c + s * s;
  }
  power
}

/// 自适应阈值：取功率的近似中位数与最大值的折中。
fn threshold(power: &[f32]) -> f32 {
  let mut vals: Vec<f32> = power.iter().step_by(4).copied().collect();
  if vals.is_empty() {
    return f32::INFINITY;
  }
  vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
  let median = vals[vals.len() / 2];
  let max = *vals.last().unwrap_or(&0.0);
  (median * 8.0).max(max * 0.3)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{SYNC_A_HZ, SYNC_WORDS, WORD_RATE};

  /// 生成带 sync A 突发的包络：每行 2080 字，sync A 位于行首。
  fn synth(lines: usize) -> Vec<f32> {
    let mut out = Vec::new();
    for _ in 0..lines {
      for i in 0..SYNC_WORDS {
        let v = 0.5 + 0.5 * (std::f32::consts::TAU * SYNC_A_HZ * i as f32 / WORD_RATE as f32).sin();
        out.push(v);
      }
      for i in 0..(LINE_WORDS - SYNC_WORDS) {
        out.push(0.5 + 0.05 * (i as f32 / 100.0).sin());
      }
    }
    out
  }

  #[test]
  fn finds_all_line_starts() {
    let words = synth(6);
    let lines = find_lines(&words);
    assert!(
      lines.len() >= 5,
      "应检测到至少 5 行，实际 {} 行",
      lines.len()
    );
    for (i, &s) in lines.iter().enumerate() {
      let expected = i * LINE_WORDS;
      assert!(
        s.abs_diff(expected) <= 2,
        "第 {i} 行起点应在 {expected}±2，得到 {s}"
      );
    }
  }

  #[test]
  fn empty_on_silence() {
    let words = vec![0.0f32; LINE_WORDS * 4];
    assert!(find_lines(&words).is_empty());
  }
}
