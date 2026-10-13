//! Walsh-Hadamard 变换、Gray 码与 Olivia 固定密钥。
//!
//! 转写自 sntfrc/olivia-python（MIT License，© 2020 Federico Santandrea）：
//! https://github.com/sntfrc/olivia-python
//!
//! 注意编码用 `ifwht`（步长从大到小、先差后和），解码用 `fwht`（步长从小到大、
//! 先和后差）—— 二者互为逆（差一个 1/64 的尺度）；解码端只做 `argmax |幅值|`，
//! 尺度与元素顺序的置换都不影响判定。

/// Olivia 规范中的 64 位固定密钥（使输出流呈现伪随机性）。
pub const KEY: u64 = 0xE257_E6D0_2915_74EC;

/// 编码侧的逆 Walsh-Hadamard（非归一化）。
///
/// 长度必须是 2 的幂（Olivia 恒为 64）：debug 下断言抓错，release 下循环条件
/// 保证不越界（非 2 幂的尾部元素不参与变换）。
pub fn ifwht(data: &mut [f64]) {
  debug_assert!(data.len().is_power_of_two(), "WHT 长度必须是 2 的幂");
  let n = data.len();
  let mut step = n / 2;
  while step >= 1 {
    let mut ptr = 0;
    while ptr + 2 * step <= n {
      for ptr2 in ptr..ptr + step {
        let bit1 = data[ptr2];
        let bit2 = data[ptr2 + step];
        data[ptr2] = bit1 - bit2;
        data[ptr2 + step] = bit1 + bit2;
      }
      ptr += 2 * step;
    }
    step /= 2;
  }
}

/// 解码侧的 Walsh-Hadamard（非归一化，`ifwht` 的逆）。长度约束同上。
pub fn fwht(data: &mut [f64]) {
  debug_assert!(data.len().is_power_of_two(), "WHT 长度必须是 2 的幂");
  let n = data.len();
  let mut step = 1;
  while step < n {
    let mut ptr = 0;
    while ptr + 2 * step <= n {
      for ptr2 in ptr..ptr + step {
        let bit1 = data[ptr2];
        let bit2 = data[ptr2 + step];
        data[ptr2] = bit2 + bit1;
        data[ptr2 + step] = bit2 - bit1;
      }
      ptr += 2 * step;
    }
    step *= 2;
  }
}

/// Gray 编码（相邻音调误判只错 1 位）。
#[must_use]
pub fn gray(n: u32) -> u32 {
  n ^ (n >> 1)
}

/// Gray 解码。
#[must_use]
pub fn degray(n: u32) -> u32 {
  let mut n = n;
  let mut mask = n;
  while mask != 0 {
    mask >>= 1;
    n ^= mask;
  }
  n
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn gray_round_trips() {
    for n in 0..64u32 {
      assert_eq!(degray(gray(n)), n, "gray({n})");
    }
    // 相邻值只差一位（Gray 码的性质）。
    for n in 0..63u32 {
      assert_eq!((gray(n) ^ gray(n + 1)).count_ones(), 1);
    }
  }

  #[test]
  fn wht_pair_recovers_the_impulse() {
    let mut v = vec![0.0f64; 64];
    v[20] = -1.0;
    let mut enc = v.clone();
    ifwht(&mut enc);
    fwht(&mut enc);
    let peak = enc
      .iter()
      .enumerate()
      .max_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap())
      .unwrap();
    assert_eq!(peak.0, 20, "峰值位置");
    assert!((peak.1 + 64.0).abs() < 1e-9, "幅值 -64（尺度）");
  }
}
