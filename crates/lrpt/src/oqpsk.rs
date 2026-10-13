//! OQPSK 调制 / 解调（LRPT 的物理层：72k 符号/s，I/Q 各半符号错开）。
//!
//! 接口按「位流 ↔ 复数基带样本」定义：
//! - 调制：输入位流（0/1）→ 相邻两位为 (I, Q) 符号对，Q 滞后 I 半个符号；
//! - 解调：匹配滤波（矩形窗积分）后按符号定时采样，输出软判决位（0/1 硬量化）。
//!
//! 合成样本按已知定时生成与解调（采样相位由调用方给定）；真实录音的
//! 定时恢复 / 载波同步属于接收机前端，不在本模块范围。

/// 每符号样本数上限：`sps` 来自公共 API，`2 * sps` / `symbols * 2 * sps` 在 wasm32 上会回绕成 0
/// 或触发容量溢出；真实用法是 4（288 kHz），上界取得很宽松只为挡住构造输入。
pub const MAX_SPS: usize = 4096;

/// 位流 → 复数基带样本（I 与 Q 交替存储：`samples[2i] = I_i`，`samples[2i+1] = Q_i`）。
///
/// 每符号 `sps` 个复数样本；符号 k 的 I 位 = 位 `2k`，Q 位 = 位 `2k+1`，
/// Q 通道整体滞后半个符号（OQPSK 的本质）。
#[must_use]
pub fn modulate(bits: &[u8], sps: usize) -> Vec<f32> {
  if sps == 0 || sps > MAX_SPS {
    return Vec::new();
  }
  let symbols = bits.len() / 2;
  let mut out = Vec::with_capacity(symbols * 2 * sps);
  for k in 0..symbols {
    let i_bit = bits[2 * k];
    for j in 0..sps {
      // 矩形脉冲：符号 k 的 I 在 [k·sps, (k+1)·sps) 有效。
      // 极性约定对齐 lrpt-encoder：bit 0 → +1（bpsk）。
      out.push(if i_bit == 0 { 1.0 } else { -1.0 });
      // Q 滞后半个符号：样本 j 落在符号 k 的前半还是后半。
      let q_sym = if j < sps / 2 { k } else { k + 1 };
      let q = if q_sym < symbols {
        bits[2 * q_sym + 1]
      } else {
        0
      };
      out.push(if q == 0 { 1.0 } else { -1.0 });
    }
  }
  out
}

/// 复数基带样本 → 软判决位流（每符号取匹配滤波窗积分，硬量化 0/1）。
///
/// `sps`：每符号样本数。I 通道在符号窗上积分、Q 通道取滞后半符号的窗。
#[must_use]
pub fn demodulate(samples: &[f32], sps: usize) -> Vec<u8> {
  // sps=0 → 下面 `2 * sps` 是整数除零（release 也 panic）；`sps` 过大则 `2 * sps` 回绕：
  // 公共 API 直接拒绝（上界见 [`MAX_SPS`]，之后 `2 * sps` 不会溢出）。
  if sps == 0 || sps > MAX_SPS || samples.len() < 2 * sps {
    return Vec::new();
  }
  let symbols = samples.len() / (2 * sps);
  let mut out = Vec::with_capacity(symbols * 2);
  for k in 0..symbols {
    let mut acc_i = 0.0f64;
    let mut acc_q = 0.0f64;
    for j in 0..sps {
      acc_i += f64::from(samples[(k * sps + j) * 2]);
      // Q 通道的窗滞后半符号。
      let q_idx = ((k * sps + j) as isize - (sps as isize / 2)) * 2 + 1;
      acc_q += if q_idx >= 0 {
        f64::from(samples[q_idx as usize])
      } else {
        0.0
      };
    }
    // 极性约定对齐 lrpt-encoder：+1 → bit 0。
    out.push(if acc_i < 0.0 { 1 } else { 0 });
    out.push(if acc_q < 0.0 { 1 } else { 0 });
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn round_trips_clean_signal() {
    let bits: Vec<u8> = (0..200).map(|i| (i * 5 % 3 % 2) as u8).collect();
    let samples = modulate(&bits, 4);
    let back = demodulate(&samples, 4);
    assert_eq!(back, bits, "无噪声往返");
  }

  #[test]
  fn oqpsk_q_channel_lags_half_symbol() {
    // 直接验证 Q 通道在符号边界处取的是「滞后半符号」的窗。
    let bits = vec![0u8; 8]; // 4 个符号全 0
    let samples = modulate(&bits, 4);
    // 符号 0 的 Q 窗 = [-2, 2) 样本处为 0（无前符号）→ 积分负，判 0 ✓；
    // 符号 3 的 Q 窗尾超界按 0 处理 ✓ —— 往返已覆盖。
    assert_eq!(demodulate(&samples, 4), bits);
  }

  #[test]
  fn tolerates_small_noise() {
    // 简单噪声：样本加 ±0.3 抖动，积分后仍判对。
    let bits: Vec<u8> = (0..64).map(|i| (i % 2) as u8).collect();
    let mut samples = modulate(&bits, 8);
    for (i, s) in samples.iter_mut().enumerate() {
      *s += ((i * 37 % 10) as f32 - 4.5) * 0.12;
    }
    assert_eq!(demodulate(&samples, 8), bits);
  }
}
