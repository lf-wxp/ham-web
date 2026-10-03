//! VIS（Vertical Interval Signaling）头检测：识别 SSTV 模式。

use super::WORK_RATE;

/// 每个 VIS 位时长（毫秒）。
const BIT_MS: f32 = 30.0;
/// VIS 起始位时长（毫秒）。
const START_MS: f32 = 300.0;

/// 检测 VIS 码，返回 `(7 位数据码, VIS 结束位置)`。
///
/// VIS 结构：起始位（1200 Hz，300 ms）+ 8 位（各 30 ms，1100 Hz=0 / 1300 Hz=1，LSB 先发）
/// + 停止位（1200 Hz，30 ms）。第 8 位为偶校验，返回时剥离。
pub fn detect(freq: &[f32]) -> Option<(u8, usize)> {
  let bit = (BIT_MS / 1000.0 * WORK_RATE as f32).round() as usize;
  let start = (START_MS / 1000.0 * WORK_RATE as f32).round() as usize;

  // 起始位判定放宽到约 75%：FM 解调在频率跳变（leader → 起始位）处有瞬态，
  // 会吃掉起始位前段几十毫秒，按满长连续计数会漏检。
  let min_run = (start * 3 / 4).max(1);
  let start_idx = find_sync_run(freq, min_run)?;

  let mut raw = 0u8;
  let mut pos = start_idx + start;
  for b in 0..8 {
    let end = pos + bit;
    if end > freq.len() {
      return None;
    }
    let avg = mean(&freq[pos..end]);
    // 1100 Hz = 0，1300 Hz = 1，阈值取 1200。
    if avg > 1200.0 {
      raw |= 1 << b;
    }
    pos = end;
  }

  // VIS 结束 = 起始 + 8 位 + 停止位。
  Some((raw & 0x7F, pos + bit))
}

/// 找一段持续至少 `min_len` 个样本的 1200 Hz 同步区段，返回其起始位置。
fn find_sync_run(freq: &[f32], min_len: usize) -> Option<usize> {
  let mut run = 0usize;
  for (i, &f) in freq.iter().enumerate() {
    if (1150.0..=1250.0).contains(&f) {
      run += 1;
      if run >= min_len {
        return Some(i + 1 - run);
      }
    } else {
      run = 0;
    }
  }
  None
}

fn mean(xs: &[f32]) -> f32 {
  if xs.is_empty() {
    return 0.0;
  }
  xs.iter().sum::<f32>() / xs.len() as f32
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 合成一段 VIS 头（起始 + 8 位 + 停止），用恒定频率表示。
  fn synth_vis(code: u8) -> Vec<f32> {
    let bit = (BIT_MS / 1000.0 * WORK_RATE as f32).round() as usize;
    let start = (START_MS / 1000.0 * WORK_RATE as f32).round() as usize;
    let mut out = Vec::new();
    // 起始位 1200 Hz
    out.extend(std::iter::repeat_n(1200.0f32, start));
    // 8 位（LSB first），含偶校验
    for b in 0..8 {
      let f = if (code >> b) & 1 == 1 { 1300.0 } else { 1100.0 };
      out.extend(std::iter::repeat_n(f, bit));
    }
    // 停止位 1200 Hz
    out.extend(std::iter::repeat_n(1200.0f32, bit));
    out
  }

  #[test]
  fn detects_vis_code() {
    let freq = synth_vis(60); // Scottie S1
    let (code, end) = detect(&freq).unwrap();
    assert_eq!(code, 60 & 0x7F);
    assert!(end <= freq.len());
  }

  #[test]
  fn none_on_plain_tone() {
    let freq = vec![1900.0f32; WORK_RATE as usize]; // 无 1200 Hz 区段
    assert!(detect(&freq).is_none());
  }
}
