//! CADU（Channel Access Data Unit）：帧同步、去交织、去随机化。
//!
//! 参数逐字转写自 artlav/meteor_decoder（GPL-3.0）：
//! - 帧 = 1024 字节 = ASM（4 字节 `1A CF FC 1D`）+ 1020 字节数据；
//! - 去随机化：`data[j] ^= PRAND[j % 255]`（ASM 不动）；
//! - 卷积交织器（Meteor-M2-2 / 80k 信号用）：分支 36、延迟 2048 字节
//!   （`pos = i + (B−1)·D − (i mod B)·(B·D) + (B/2)·(B·D)`）。

use crate::prand::PRAND;

/// ASM 同步字（传输字节序）。
pub const ASM: [u8; 4] = [0x1A, 0xCF, 0xFC, 0x1D];
/// 帧长（字节）。
pub const FRAME_LEN: usize = 1024;
/// 数据区长度（帧长 − ASM）。
pub const DATA_LEN: usize = FRAME_LEN - ASM.len();

/// 卷积交织器参数（M2-2 / 80k）。
pub const INTERLEAVE_BRANCHES: usize = 36;
pub const INTERLEAVE_DELAY: usize = 2048;

/// 在字节流里搜 ASM（±`tol` 位容错），返回命中位置（字节偏移）。
///
/// ASM 之后跟着的是卷积编码前的数据，因此同步在「Viterbi 译码后」的字节流上做；
/// 极性歧义由调用方用 `check_polarity` 处理。
#[must_use]
pub fn find_asm(bytes: &[u8], tol: usize) -> Vec<usize> {
  let asm_bits: Vec<u8> = ASM
    .iter()
    .flat_map(|&b| (0..8).map(move |i| (b >> (7 - i)) & 1))
    .collect();
  let mut hits = Vec::new();
  if bytes.len() < FRAME_LEN {
    return hits;
  }
  for start in 0..=bytes.len() - FRAME_LEN {
    let mut bad = 0usize;
    for (k, &bit) in asm_bits.iter().enumerate() {
      let byte = bytes[start + k / 8];
      let b = (byte >> (7 - k % 8)) & 1;
      if b != bit {
        bad += 1;
        if bad > tol {
          break;
        }
      }
    }
    if bad <= tol {
      hits.push(start);
    }
  }
  hits
}

/// 极性校验：译码后的 ASM 应是 `1A CF FC 1D` 或它的逐位反码（卷积码的对称性
/// 允许整帧翻转）。返回「是否需要整帧翻转」。
#[must_use]
pub fn check_polarity(frame: &[u8; FRAME_LEN]) -> bool {
  let mut same = 0usize;
  let mut flipped = 0usize;
  for i in 0..ASM.len() {
    same += (frame[i] ^ ASM[i]).count_ones() as usize;
    flipped += (frame[i] ^ !ASM[i]).count_ones() as usize;
  }
  flipped < same
}

/// 去随机化（原地）：`frame[4..1024]` 逐字节 XOR `PRAND[i % 255]`。
pub fn derandomize(frame: &mut [u8; FRAME_LEN]) {
  for i in 0..DATA_LEN {
    frame[ASM.len() + i] ^= PRAND[i % PRAND.len()];
  }
}

/// 卷积交织（TX 侧，与 [`deinterleave`] 互为转置）。
///
/// 去交织（参考实现原样）是 `dst[pos(i)] = src[i]`，
/// ```text
/// pos(i) = i + (B−1)·D − (i mod B)·(B·D) + (B/2)·(B·D)
/// ```
/// 则交织 = `dst[i] = src[pos(i)]`（有限缓冲两端按 0 处理，与参考实现一致）。
#[must_use]
pub fn interleave(src: &[u8]) -> Vec<u8> {
  let (b, d) = (INTERLEAVE_BRANCHES, INTERLEAVE_DELAY);
  let base = b * d;
  let offset = (b / 2) * base;
  let mut dst = vec![0u8; src.len()];
  for (i, slot) in dst.iter_mut().enumerate() {
    let pos = i as i64 + ((b - 1) * d) as i64 - ((i % b) * base) as i64 + offset as i64;
    if pos >= 0 && pos < src.len() as i64 {
      *slot = src[pos as usize];
    }
  }
  dst
}

/// 卷积去交织（RX 侧，参考实现 `deint_block` 公式）。
#[must_use]
pub fn deinterleave(src: &[u8]) -> Vec<u8> {
  let (b, d) = (INTERLEAVE_BRANCHES, INTERLEAVE_DELAY);
  let base = b * d;
  let offset = (b / 2) * base;
  let mut dst = vec![0u8; src.len()];
  for (i, &v) in src.iter().enumerate() {
    let pos = i as i64 + ((b - 1) * d) as i64 - ((i % b) * base) as i64 + offset as i64;
    if pos >= 0 && pos < src.len() as i64 {
      dst[pos as usize] = v;
    }
  }
  dst
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn asm_search_finds_frames_with_tolerance() {
    let mut stream = vec![0u8; FRAME_LEN * 2 + 10];
    stream[10..14].copy_from_slice(&ASM);
    stream[10 + FRAME_LEN..10 + FRAME_LEN + 4].copy_from_slice(&ASM);
    assert_eq!(find_asm(&stream, 0), vec![10, 10 + FRAME_LEN]);
    // 3 个位错误仍命中。
    let mut noisy = stream.clone();
    noisy[10] ^= 0b0000_0111;
    assert!(find_asm(&noisy, 3).contains(&10));
  }

  #[test]
  fn polarity_check_detects_flipped_frames() {
    let mut frame = [0u8; FRAME_LEN];
    frame[..4].copy_from_slice(&ASM);
    assert!(!check_polarity(&frame));
    let mut flipped = frame;
    for b in &mut flipped {
      *b = !*b;
    }
    assert!(check_polarity(&flipped));
  }

  #[test]
  fn derandomize_is_self_inverse() {
    let mut frame = [0u8; FRAME_LEN];
    frame[..4].copy_from_slice(&ASM);
    for (i, b) in frame.iter_mut().enumerate().skip(4) {
      *b = (i * 31 % 251) as u8;
    }
    let orig = frame;
    derandomize(&mut frame);
    assert_ne!(frame[..8], orig[..8]);
    derandomize(&mut frame);
    assert_eq!(frame, orig, "XOR 两次回到原值");
  }

  #[test]
  fn interleave_deinterleave_round_trips() {
    // 交织器展宽 ~ (B−1)·D + (B/2)·(B·D) ≈ 1.4 MB，窗口必须远大于它，
    // 且比对区间要避开两端各 B·(B·D) 的模糊区。
    let len = 8_000_000usize;
    let data: Vec<u8> = (0..len).map(|i| (i * 13 % 256) as u8).collect();
    let interleaved = interleave(&data);
    assert_ne!(interleaved[..64], data[..64], "交织确实打乱顺序");
    let back = deinterleave(&interleaved);
    let skip = INTERLEAVE_BRANCHES * INTERLEAVE_BRANCHES * INTERLEAVE_DELAY;
    assert_eq!(
      back[skip..back.len() - skip],
      data[skip..data.len() - skip],
      "中部往返一致"
    );
  }
}
