//! M2-2（80k 符号率）链路前端：`0x27` 重同步 + 符号级卷积去交织 + DQPSK 差分。
//!
//! 转写自 artlav/meteor_decoder（GPL-3.0）`medet.dpr` 的 80k 分支（接收侧）：
//! - `resync_stream`：80k 流没有逐帧 ASM，同步字是每 80 符号重复一次的
//!   `00100111`（0x27）—— 8 个符号的 I 位 MSB-first 拼成该字节；找 4 次重复
//!   定位对齐，随后「前瞻保持」防弱信号失锁，每 80 符号取 72 个数据符号；
//! - `deint_block`：36 分支 × 2048 延迟的卷积去交织（符号粒度）；
//! - `de_diffcode`：差分译码 —— 软判决 `I' = corr(I,I₋₁)`、`Q' = −corr(Q,Q₋₁)`，
//!   硬判决等价为 QPSK 相位差（`I' = I⊕I₋₁`、`Q' = Q⊕Q₋₁⊕1`）。
//!
//! 链路（本 crate 的自洽约定，与参考实现的接收侧一一对应）：
//! ```text
//! 帧 → 连续卷积 → 符号(2 位) → 差分 → 交织 → 0x27 同步插入 → QPSK
//! ```
//! 如实说明：artlav 只提供接收侧，发送侧结构由**往返自洽**兜底；真实 M2-2
//! 录音的互操作对拍仍待 SatNOGS 过境样本（参考实现的帧同步器
//! `mtd_one_frame` 软相关器未转写 —— 本模块用连续 Viterbi + ASM 替代）。

use crate::cadu::{ASM, FRAME_LEN, deinterleave, interleave};
use crate::prand;
use crate::rrc;
use crate::viterbi;

/// 80k 流的同步字节：`00100111`（8 符号的 I 位，MSB-first）。
const SYNC_BYTE: u8 = 0x27;
/// 同步步长（符号）。
const SYNC_STEP: usize = 80;
/// 每个步长里的数据符号数（8 符号同步 + 72 数据）。
const DATA_PER_STEP: usize = 72;

/// 符号索引（0..3，QPSK）的 I 位。
fn i_bit(sym: u8) -> u8 {
  sym >> 1
}

/// 8 个连续符号的 I 位 → MSB-first 字节（同步判定用）。
fn pack_i(syms: &[u8]) -> u8 {
  let mut b = 0u8;
  for (i, &s) in syms.iter().take(8).enumerate() {
    b |= i_bit(s) << (7 - i);
  }
  b
}

/// `resync_stream` 的转写：在符号流里找 0x27 对齐，输出剥离同步后的数据符号。
///
/// 失败策略与参考实现一致：找不到同步就跳过 3 个步长继续找；找到后
/// 「前瞻保持」—— 接下来 128 个步长里还能见到同步就继续，否则失锁。
#[must_use]
pub fn resync_0x27(syms: &[u8]) -> Vec<u8> {
  let mut out = Vec::with_capacity(syms.len());
  let mut pos = 0usize;
  // `<=`：剩余长度恰好为一个搜索窗（SYNC_STEP×4）时仍要搜最后一次。
  while pos + SYNC_STEP * 4 <= syms.len() {
    let found = (0..SYNC_STEP).find_map(|off| {
      let start = pos + off;
      if start + SYNC_STEP * 4 > syms.len() {
        return None;
      }
      (0..4)
        .all(|j| pack_i(&syms[start + j * SYNC_STEP..]) == SYNC_BYTE)
        .then_some(start)
    });
    let Some(sync_pos) = found else {
      pos += SYNC_STEP * 3;
      continue;
    };
    pos = sync_pos;
    while pos + SYNC_STEP <= syms.len() {
      let keep = (0..128).any(|i| {
        let at = pos + i * SYNC_STEP;
        at + 8 <= syms.len() && pack_i(&syms[at..]) == SYNC_BYTE
      });
      if !keep {
        break;
      }
      out.extend_from_slice(&syms[pos + 8..pos + SYNC_STEP]);
      pos += SYNC_STEP;
    }
  }
  out
}

/// 符号索引流 → I/Q 交错位流。
#[must_use]
pub fn syms_to_bits(syms: &[u8]) -> Vec<u8> {
  let mut out = Vec::with_capacity(syms.len() * 2);
  for &s in syms {
    out.push(s >> 1);
    out.push(s & 1);
  }
  out
}

/// I/Q 交错位流 → 符号索引流。
#[must_use]
pub fn bits_to_syms(bits: &[u8]) -> Vec<u8> {
  bits
    .chunks(2)
    .map(|c| (c[0] << 1) | (c.get(1).copied().unwrap_or(0) & 1))
    .collect()
}

/// 差分译码（硬判决，QPSK 相位差）：`I' = I ⊕ I₋₁`、`Q' = Q ⊕ Q₋₁ ⊕ 1`。
#[must_use]
pub fn dediff_bits(bits: &[u8]) -> Vec<u8> {
  let mut out = vec![0u8; bits.len()];
  for k in 1..bits.len() / 2 {
    out[2 * k] = bits[2 * k] ^ bits[2 * k - 2];
    out[2 * k + 1] = bits[2 * k + 1] ^ bits[2 * k - 1] ^ 1;
  }
  out
}

/// 差分编码（TX，与 [`dediff_bits`] 互逆）。
#[must_use]
pub fn diffencode_bits(bits: &[u8]) -> Vec<u8> {
  let mut out = vec![0u8; bits.len()];
  for k in 1..bits.len() / 2 {
    out[2 * k] = bits[2 * k] ^ out[2 * k - 2];
    out[2 * k + 1] = bits[2 * k + 1] ^ out[2 * k - 1] ^ 1;
  }
  out
}

/// 发送侧 80k 链路（测试与对拍用）：帧 → 连续卷积 → 符号交织 → 差分 →
/// 0x27 同步插入 → RRC/OQPSK @ 320 kHz（80k × 4）。
#[must_use]
pub fn encode_stream80k(frames: &[[u8; crate::DATA_LEN]]) -> Vec<f32> {
  // 帧字节流 → 位 → 连续卷积。
  let mut bits = Vec::new();
  for data in frames {
    let mut cadu = Vec::with_capacity(1024);
    cadu.extend_from_slice(&ASM);
    let mut d = *data;
    for (i, b) in d.iter_mut().enumerate() {
      *b ^= prand::PRAND[i % prand::PRAND.len()];
    }
    cadu.extend_from_slice(&d);
    for &b in &cadu {
      for i in (0..8).rev() {
        bits.push((b >> i) & 1);
      }
    }
  }
  let mut state = 0u8;
  let coded = viterbi::conv_encode_continuous(&bits, &mut state);

  // 符号（2 位）粒度：差分 → 交织 → 同步插入。
  // 差分与交织**不可交换**：RX 是「resync → 去交织 → dediff」（参考实现顺序），
  // 因此 TX 必须先差分（作用在原序列的相邻符号上）再交织。
  let syms: Vec<u8> = coded.chunks(2).map(|c| (c[0] << 1) | c[1]).collect();
  let diffed = diffencode_bits(&syms_to_bits(&syms));
  let inter = interleave(&bits_to_syms(&diffed));
  // 同步按**符号**步进：每 80 符号 = 8 符号 0x27 同步 + 72 数据符号。
  let mut synced = Vec::new();
  for chunk in inter.chunks(DATA_PER_STEP) {
    for i in (0..8).rev() {
      synced.push(((SYNC_BYTE >> i) & 1) << 1); // Q 位取 0
    }
    synced.extend_from_slice(chunk);
  }

  // QPSK（每符号 2 位）→ 冲激 + RRC（与 72k 的 encode_stream288 同构）。
  let taps = rrc::rrc_taps();
  let n_taps = taps.len();
  let mut i_imp = vec![0.0f32; synced.len() * rrc::SPS];
  let mut q_imp = vec![0.0f32; synced.len() * rrc::SPS];
  for (k, &s) in synced.iter().enumerate() {
    // 极性约定对齐 lrpt-encoder：bit 0 → +1；Q 滞后半符号（SPS/2 样本），
    // 与 72k 链路的 encode_stream288 一致（解调端在 i + SPS/2 采 Q）。
    i_imp[k * rrc::SPS] = if s & 0b10 == 0 { 1.0 } else { -1.0 };
    let qpos = k * rrc::SPS + rrc::SPS / 2;
    if qpos < q_imp.len() {
      q_imp[qpos] = if s & 0b01 == 0 { 1.0 } else { -1.0 };
    }
  }
  let conv = |ch: &[f32]| -> Vec<f32> {
    let len = ch.len().saturating_sub(n_taps - 1);
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
      let mut acc = 0.0f32;
      for (k, &t) in taps.iter().enumerate() {
        acc += ch[i + k] * t;
      }
      out.push(acc);
    }
    out
  };
  let fi = conv(&i_imp);
  let fq = conv(&q_imp);
  let mut out = Vec::with_capacity(fi.len() * 2);
  for (a, b) in fi.iter().zip(&fq) {
    out.push(*a);
    out.push(*b);
  }
  out
}

/// 接收侧 80k 链路：320 kHz I/Q → RRC 解调 → 0x27 重同步 → 去交织 → 差分
/// 译码 → 连续 Viterbi → ASM 同步 → 去扰码 → 帧。
///
/// 注意：采样相位硬编码为 0 —— 80k 链路尚未接线（内容层往返测试不含 RF 与 Viterbi），
/// 接上后要与 72k 链路一样搜索全部相位，这里不是已经就绪。
#[must_use]
pub fn decode_stream80k(iq: &[f32]) -> Vec<[u8; crate::DATA_LEN]> {
  let coded = rrc::demodulate_288(iq, 0);
  let delay_bits = rrc::FILTER_DELAY_SYMBOLS * 2;
  let drop_bits = (16 - delay_bits % 16) % 16;
  // 短输入解不出符号：返回空而不是切片越界 panic。
  if coded.len() <= drop_bits {
    return Vec::new();
  }
  let coded = &coded[drop_bits..];

  // 符号（2 位）→ 重同步 → 去交织 → 展平 → 差分译码。
  let syms: Vec<u8> = coded.chunks(2).map(|c| (c[0] << 1) | c[1]).collect();
  let resynced = resync_0x27(&syms);
  let deint = deinterleave(&resynced);
  let diffed = dediff_bits(&syms_to_bits(&deint));

  // 连续 Viterbi → 字节 → ASM → 去扰码。
  let vout = viterbi::viterbi_decode_continuous(&diffed);
  let mut bytes = Vec::with_capacity(vout.len() / 8);
  for chunk in vout.chunks(8).filter(|c| c.len() == 8) {
    let mut b = 0u8;
    for &x in chunk {
      b = (b << 1) | x;
    }
    bytes.push(b);
  }
  let hits = crate::cadu::find_asm(&bytes, 1);
  let mut out = Vec::new();
  for &start in &hits {
    if start + FRAME_LEN > bytes.len() {
      continue;
    }
    // 与 find_asm 的位级容错一致：只允许 ASM 区内 1 个错位（数据区不受影响）。
    let asm_bad = (0..ASM.len() * 8)
      .filter(|&bit| {
        let byte = bytes[start + bit / 8];
        ((byte >> (7 - bit % 8)) & 1) != ((ASM[bit / 8] >> (7 - bit % 8)) & 1)
      })
      .count();
    if asm_bad > 1 {
      continue;
    }
    let mut d = [0u8; crate::DATA_LEN];
    d.copy_from_slice(&bytes[start + ASM.len()..start + FRAME_LEN]);
    for (i, b) in d.iter_mut().enumerate() {
      *b ^= prand::PRAND[i % prand::PRAND.len()];
    }
    out.push(d);
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn diffencode_dediff_round_trips() {
    let bits: Vec<u8> = (0..1000).map(|i| (i * 7 % 2) as u8).collect();
    let enc = diffencode_bits(&bits);
    let dec = dediff_bits(&enc);
    assert_eq!(dec[2..], bits[2..], "首符号清零，其余往返一致");
  }

  #[test]
  fn resync_recovers_data_symbols() {
    // 构造带 0x27 同步的符号流：数据 72 符号 × 6 步 + 前导垃圾。
    let mut syms = vec![0u8, 1, 2, 3]; // 前导垃圾（不足一个步长）
    for _ in 0..6 {
      for i in (0..8).rev() {
        syms.push(((SYNC_BYTE >> i) & 1) << 1);
      }
      for i in 0..DATA_PER_STEP {
        syms.push((i % 2) as u8);
      }
    }
    let out = resync_0x27(&syms);
    assert_eq!(out.len(), 6 * DATA_PER_STEP);
  }

  #[test]
  fn stream80k_content_chain_matches_in_the_middle() {
    // 内容层对拍（不含 RF 与 Viterbi —— 那两层与 72k 链路共用实现、已有单测）：
    // 帧 → 连续卷积 → 差分 → 交织 → 0x27 同步 → 逆链，中部（跳过两端各
    // B·(B·D) 符号的模糊区）与卷积码位逐位一致。
    let n_frames = 2500;
    let mut bits = Vec::new();
    for k in 0..n_frames {
      let mut cadu_v = Vec::with_capacity(1024);
      cadu_v.extend_from_slice(&ASM);
      let mut d = [0u8; crate::DATA_LEN];
      for (i, b) in d.iter_mut().enumerate() {
        *b = (i * 13 + k * 7) as u8;
      }
      for (i, b) in d.iter_mut().enumerate() {
        *b ^= prand::PRAND[i % prand::PRAND.len()];
      }
      cadu_v.extend_from_slice(&d);
      for &b in &cadu_v {
        for i in (0..8).rev() {
          bits.push((b >> i) & 1);
        }
      }
    }
    let mut state = 0u8;
    let coded = viterbi::conv_encode_continuous(&bits, &mut state);
    let syms: Vec<u8> = coded.chunks(2).map(|c| (c[0] << 1) | c[1]).collect();
    let diffed = diffencode_bits(&syms_to_bits(&syms));
    let inter = interleave(&bits_to_syms(&diffed));
    let mut synced = Vec::new();
    for chunk in inter.chunks(DATA_PER_STEP) {
      for i in (0..8).rev() {
        synced.push(((SYNC_BYTE >> i) & 1) << 1);
      }
      synced.extend_from_slice(chunk);
    }
    // 逆链：resync → 去交织 → dediff。
    let resynced = resync_0x27(&synced);
    let deint = deinterleave(&resynced);
    let diffed_rx = dediff_bits(&syms_to_bits(&deint));
    // 模糊区 = 两端各 B·(B·D) 符号（= 2 倍位）。
    let skip = 2 * 36usize * 36 * 2048;
    let m = diffed_rx[skip..diffed_rx.len() - skip]
      .iter()
      .zip(&coded[skip..coded.len() - skip])
      .filter(|(a, b)| a != b)
      .count();
    assert_eq!(m, 0, "80k 内容层中部逐位一致");
  }
}
