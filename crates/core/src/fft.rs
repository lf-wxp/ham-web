//! 通用 radix-2 FFT（纯 Rust，无 unsafe），供频谱分析使用。
//!
//! 实现为迭代式 Cooley-Tukey 蝶形：先按位反转重排，再逐级合并。
//! 输入长度须为 2 的幂。

use std::f32::consts::TAU;

/// 就地 radix-2 迭代 FFT。
///
/// `re` / `im` 为复数样本的实部与虚部，长度须相等且为 2 的幂。
/// 变换后原地覆盖为正频率（0..n）的频谱（未归一化）。
pub fn fft_in_place(re: &mut [f32], im: &mut [f32]) {
  let n = re.len();
  assert!(n.is_power_of_two(), "FFT 长度须为 2 的幂，得到 {n}");
  assert_eq!(n, im.len(), "实部与虚部长度须一致");

  // 位反转置换。
  let mut j = 0usize;
  for i in 1..n {
    let mut bit = n >> 1;
    while j & bit != 0 {
      j ^= bit;
      bit >>= 1;
    }
    j ^= bit;
    if i < j {
      re.swap(i, j);
      im.swap(i, j);
    }
  }

  // 蝶形。
  let mut len = 2usize;
  while len <= n {
    let ang = -TAU / len as f32;
    let (wlen_re, wlen_im) = (ang.cos(), ang.sin());
    for start in (0..n).step_by(len) {
      let (mut w_re, mut w_im) = (1.0f32, 0.0f32);
      for k in 0..len / 2 {
        let a = start + k;
        let b = start + k + len / 2;
        let (u_re, u_im) = (re[a], im[a]);
        let v_re = re[b] * w_re - im[b] * w_im;
        let v_im = re[b] * w_im + im[b] * w_re;
        re[a] = u_re + v_re;
        im[a] = u_im + v_im;
        re[b] = u_re - v_re;
        im[b] = u_im - v_im;
        let nw_re = w_re * wlen_re - w_im * wlen_im;
        let nw_im = w_re * wlen_im + w_im * wlen_re;
        w_re = nw_re;
        w_im = nw_im;
      }
    }
    len <<= 1;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn single_tone_peaks_at_expected_bin() {
    let n = 1024;
    let bin = 128;
    let mut re = vec![0f32; n];
    let mut im = vec![0f32; n];
    for (i, r) in re.iter_mut().enumerate() {
      *r = (TAU * bin as f32 * i as f32 / n as f32).sin();
    }
    fft_in_place(&mut re, &mut im);

    let peak = (0..n)
      .max_by(|&a, &b| re[a].hypot(im[a]).partial_cmp(&re[b].hypot(im[b])).unwrap())
      .unwrap();
    assert_eq!(peak, bin, "峰值 bin 应为 {bin}");
  }

  #[test]
  fn fft_preserves_energy_parseval() {
    let n = 256;
    let mut re: Vec<f32> = (0..n)
      .map(|i| {
        (TAU * 3.0 * i as f32 / n as f32).sin() + 0.5 * (TAU * 7.0 * i as f32 / n as f32).sin()
      })
      .collect();
    let mut im = vec![0f32; n];
    let time_energy: f32 = re.iter().map(|x| x * x).sum();
    fft_in_place(&mut re, &mut im);
    // Parseval：时域能量 = 频域能量 / n。
    let freq_energy: f32 = re
      .iter()
      .zip(im.iter())
      .map(|(r, i)| r * r + i * i)
      .sum::<f32>()
      / n as f32;
    assert!((time_energy - freq_energy).abs() < 1e-3 * time_energy);
  }
}
