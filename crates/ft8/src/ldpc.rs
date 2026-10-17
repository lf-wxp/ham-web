//! FT8 / FT4 的 LDPC(174,91) 软译码（belief propagation，sum-product）。
//!
//! 转写自 MIT 许可的 ft8_lib `ldpc.c` 的 `bp_decode`（decode.c 实际调用的就是它，
//! 同文件里的 `ldpc_decode` 已被 ft8_lib 注释掉）。校验矩阵见 [`crate::constants`]。
//!
//! 输入是 174 个对数似然比（log-likelihood），约定为 `log(P(x=1)/P(x=0))`：
//! 正值表示这一位更可能是 1，负值更可能是 0。ft8_lib 的注释写成了 `log(P(x=0)/P(x=1))`，
//! 与 `ft8_extract_symbol` 的实际符号相反——以代码为准（对拍验证过）。
//!
//! 软信息的**量级**要匹配似然归一化后的值（`ftx_normalize_logl` 把方差缩放到 24，约 ±4.9）：
//! 太大（如 ±50）会让 `fast_tanh` / `fast_atanh` 的近似饱和失真、反而纠不了错。

use crate::constants::{LDPC_MN, LDPC_NM, LDPC_NUM_ROWS};

/// `tanh` 的多项式近似（ft8_lib 原样）。
fn fast_tanh(x: f32) -> f32 {
  if x < -4.97 {
    return -1.0;
  }
  if x > 4.97 {
    return 1.0;
  }
  let x2 = x * x;
  let a = x * (945.0 + x2 * (105.0 + x2));
  let b = 945.0 + x2 * (420.0 + x2 * 15.0);
  a / b
}

/// `atanh` 的多项式近似（ft8_lib 原样）。
fn fast_atanh(x: f32) -> f32 {
  let x2 = x * x;
  let a = x * (945.0 + x2 * (-735.0 + x2 * 64.0));
  let b = 945.0 + x2 * (-1050.0 + x2 * 225.0);
  a / b
}

/// 统计 174 位码字不满足的校验数（0 表示合法码字）。
fn ldpc_check(codeword: &[u8; 174]) -> usize {
  let mut errors = 0;
  for m in 0..LDPC_NUM_ROWS.len() {
    let mut x = 0u8;
    for i in 0..LDPC_NUM_ROWS[m] as usize {
      x ^= codeword[LDPC_NM[m][i] as usize - 1];
    }
    if x != 0 {
      errors += 1;
    }
  }
  errors
}

/// 软译码：把 174 个对数似然比译成 174 位硬判决，返回「最佳结果的校验错误数」（0 表示完美）。
///
/// - `codeword`：174 个 `log(P(1)/P(0))`；
/// - `plain`：输出 174 位（0/1）；
/// - `max_iters`：BP 迭代上限（ft8_lib 的 demo 用 25）。
pub fn bp_decode(codeword: &[f32; 174], max_iters: usize, plain: &mut [u8; 174]) -> usize {
  let mut tov = [[0.0f32; 3]; 174];
  let mut toc = [[0.0f32; 7]; 83];
  let mut min_errors = LDPC_NUM_ROWS.len();

  for _ in 0..max_iters {
    // 硬判决（第一轮 tov 全 0）。
    let mut plain_sum = 0usize;
    for n in 0..174 {
      let bit = if codeword[n] + tov[n][0] + tov[n][1] + tov[n][2] > 0.0 {
        1
      } else {
        0
      };
      plain[n] = bit;
      plain_sum += usize::from(bit);
    }
    if plain_sum == 0 {
      // 收敛到全零（协议禁止），直接退出。
      break;
    }
    let errors = ldpc_check(plain);
    if errors < min_errors {
      min_errors = errors;
      if errors == 0 {
        break; // 找到合法码字。
      }
    }

    // 变量节点 → 校验节点。
    for m in 0..83 {
      for n_idx in 0..LDPC_NUM_ROWS[m] as usize {
        let n = LDPC_NM[m][n_idx] as usize - 1;
        let mut tnm = codeword[n];
        for m_idx in 0..3 {
          if LDPC_MN[n][m_idx] as usize - 1 != m {
            tnm += tov[n][m_idx];
          }
        }
        toc[m][n_idx] = fast_tanh(-tnm / 2.0);
      }
    }

    // 校验节点 → 变量节点。
    for n in 0..174 {
      for m_idx in 0..3 {
        let m = LDPC_MN[n][m_idx] as usize - 1;
        let mut tmn = 1.0f32;
        for n_idx in 0..LDPC_NUM_ROWS[m] as usize {
          if LDPC_NM[m][n_idx] as usize - 1 != n {
            tmn *= toc[m][n_idx];
          }
        }
        tov[n][m_idx] = -2.0 * fast_atanh(tmn);
      }
    }
  }
  min_errors
}

/// 把 174 位硬判决打包成 22 字节（MSB first），对应 ft8_lib `pack_bits`。
#[must_use]
pub fn pack_bits(bits: &[u8; 174]) -> [u8; 22] {
  let mut out = [0u8; 22];
  for (i, &b) in bits.iter().enumerate() {
    if b != 0 {
      out[i / 8] |= 0x80 >> (i % 8);
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::crc::add_crc;
  use crate::encode174;

  /// 用一个真实 payload 编出合法码字，转软信息后软译码，应当原样译回（0 校验错、逐位一致）。
  fn codeword_for(payload: &[u8; 10]) -> [u8; 22] {
    let mut a91 = [0u8; 12];
    add_crc(payload, &mut a91);
    let mut cw = [0u8; 22];
    encode174(&a91, &mut cw);
    cw
  }

  fn soft_from_bits(bits: &[u8; 174], amp: f32) -> [f32; 174] {
    let mut out = [0.0f32; 174];
    for (i, o) in out.iter_mut().enumerate() {
      // 约定 log(P(1)/P(0))：bit=1 → +amp，bit=0 → -amp。
      *o = if bits[i] != 0 { amp } else { -amp };
    }
    out
  }

  fn bits_of(codeword: &[u8; 22]) -> [u8; 174] {
    let mut bits = [0u8; 174];
    for i in 0..174 {
      bits[i] = (codeword[i / 8] >> (7 - (i % 8))) & 1;
    }
    bits
  }

  /// 对拍值来自 ft8_lib 编译运行：payload `11000000000e100401` 的码字，以及干净 / 翻转 1-3 位都能译回。
  #[test]
  fn clean_and_flipped_codewords_decode_back_to_the_original() {
    let payload: [u8; 10] = [0x11, 0x00, 0x00, 0x00, 0x00, 0x0e, 0x10, 0x04, 0x01, 0x00];
    let codeword = codeword_for(&payload);
    // 逐字节对拍 ft8_lib。
    assert_eq!(
      codeword,
      [
        0x11, 0x00, 0x00, 0x00, 0x00, 0x0e, 0x10, 0x04, 0x01, 0x06, 0x7e, 0xb9, 0xea, 0xe9, 0xaf,
        0x34, 0x84, 0x28, 0x21, 0x7d, 0x41, 0x9c,
      ]
    );

    let bits = bits_of(&codeword);
    for flips in [&[0usize][..], &[0, 1], &[5, 50, 100]] {
      let mut flipped = bits;
      for &f in flips {
        flipped[f] ^= 1;
      }
      for amp in [4.0f32, 5.0, 8.0] {
        let soft = soft_from_bits(&flipped, amp);
        let mut plain = [0u8; 174];
        let errors = bp_decode(&soft, 25, &mut plain);
        assert_eq!(errors, 0, "flips={flips:?} amp={amp}");
        assert_eq!(plain, bits, "flips={flips:?} amp={amp}");
        // 打包后与码字一致。
        assert_eq!(pack_bits(&plain), codeword);
      }
    }
  }

  #[test]
  fn ldpc_check_accepts_a_valid_codeword_and_rejects_garbage() {
    let payload: [u8; 10] = [0x11, 0x00, 0x00, 0x00, 0x00, 0x0e, 0x10, 0x04, 0x01, 0x00];
    let codeword = codeword_for(&payload);
    assert_eq!(ldpc_check(&bits_of(&codeword)), 0);
    let mut garbage = bits_of(&codeword);
    garbage[0] ^= 1;
    assert!(ldpc_check(&garbage) > 0);
  }
}
