//! NRZ-M 差分编码/译码（Meteor-M2 系列 LRPT 链路的一部分）。
//!
//! 转写自 lrpt-encoder（MIT）`differential.rs`：`out = in ^ prev_out`（状态
//! 跨帧保留）。NRZ-M 对整流极性翻转**不敏感**（`(1−b) ^ (1−p) = b ^ p`），
//! 所以译码后无需再做极性判定。

/// NRZ-M 编码（位级，`last` 跨调用保留）。
#[must_use]
pub fn nrzm_encode(bits: &[u8], last: &mut u8) -> Vec<u8> {
  let mut out = Vec::with_capacity(bits.len());
  for &b in bits {
    let e = b ^ (*last & 1);
    *last = e;
    out.push(e);
  }
  out
}

/// NRZ-M 译码（位级，`last` 跨调用保留）：`out = in ^ prev_in`。
#[must_use]
pub fn nrzm_decode(bits: &[u8], last: &mut u8) -> Vec<u8> {
  let mut out = Vec::with_capacity(bits.len());
  for &b in bits {
    out.push(b ^ (*last & 1));
    *last = b;
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn round_trips_with_continuity() {
    let bits: Vec<u8> = (0..300).map(|i| (i * 7 % 3 % 2) as u8).collect();
    let mut tx = 0u8;
    let mut encoded = Vec::new();
    for chunk in bits.chunks(50) {
      encoded.extend(nrzm_encode(chunk, &mut tx));
    }
    let mut rx = 0u8;
    let mut decoded = Vec::new();
    for chunk in encoded.chunks(50) {
      decoded.extend(nrzm_decode(chunk, &mut rx));
    }
    assert_eq!(decoded, bits);
  }

  #[test]
  fn channel_polarity_flip_is_invariant() {
    // NRZ-M 的性质：**信道**把整条编码流取反（整流器极性翻转）不影响译码结果
    // —— 除了流首第一位（初始状态不确定，这是 NRZ-M 的固有歧义，靠 ASM 兜底）。
    let bits = vec![1, 0, 1, 1, 0, 0, 1, 0, 1, 1, 0, 1, 1, 0, 0, 1];
    let mut t = 0u8;
    let encoded = nrzm_encode(&bits, &mut t);
    let flipped: Vec<u8> = encoded.iter().map(|b| 1 - b).collect();
    let mut r1 = 0u8;
    let mut r2 = 0u8;
    let a = nrzm_decode(&encoded, &mut r1);
    let b = nrzm_decode(&flipped, &mut r2);
    assert_eq!(&a[1..], &b[1..]);
  }
}
