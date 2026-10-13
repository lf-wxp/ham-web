//! 线性插值重采样到工作采样率，降采样前做抗混叠低通。

use super::{Error, WORK_RATE};

/// 输入采样率下限：SSTV 子载波最高 2.3 kHz，低于 4 kHz 的声明采样率必然是损坏或伪造文件。
/// 不设下限时，声明 1 Hz 的 WAV 会让放大倍数达到 8000，输出长度饱和成 `usize` 极值，
/// 随后的 `Vec::with_capacity` 在 wasm32 上直接 trap。
const MIN_SOURCE_RATE: u32 = 4000;
/// 输出样本量上限：等价于 20 分钟 @ [`WORK_RATE`]（16-bit 单声道约 19 MB 源数据）。
/// SSTV 单帧最长约 3 分钟，正常录音远达不到；只有畸形输入（合法下限采样率 + 超大数据）
/// 才会碰到，此时拒绝而不是分配天文数字内存。
const MAX_OUT_SAMPLES: usize = WORK_RATE as usize * 20 * 60;

/// 汉明窗 sinc 低通系数（直流增益归一化为 1）。
fn lowpass(cutoff: f32, taps: usize) -> Vec<f32> {
  let taps = (taps.max(3) | 1).min(255);
  let m = (taps - 1) / 2;
  let mut coeffs = Vec::with_capacity(taps);
  let mut sum = 0.0f32;
  for i in 0..taps {
    let n = i as isize - m as isize;
    let h = if n == 0 {
      2.0 * cutoff
    } else {
      (std::f32::consts::TAU * cutoff * n as f32).sin() / (std::f32::consts::PI * n as f32)
    };
    let w = 0.54 - 0.46 * (std::f32::consts::TAU * i as f32 / (taps - 1) as f32).cos();
    let c = h * w;
    coeffs.push(c);
    sum += c;
  }
  for c in &mut coeffs {
    *c /= sum;
  }
  coeffs
}

/// FIR 直接卷积（边界补零）。
fn fir(x: &[f32], coeffs: &[f32]) -> Vec<f32> {
  let n = x.len();
  let mut out = vec![0.0f32; n];
  let m = (coeffs.len() - 1) / 2;
  for (i, o) in out.iter_mut().enumerate() {
    let mut acc = 0.0f32;
    for (k, &c) in coeffs.iter().enumerate() {
      let idx = i as isize + k as isize - m as isize;
      if idx >= 0 && (idx as usize) < n {
        acc += x[idx as usize] * c;
      }
    }
    *o = acc;
  }
  out
}

/// 将任意采样率线性重采样到 [`WORK_RATE`]。
pub fn to_work_rate(samples: &[f32], source_rate: u32) -> Result<Vec<f32>, Error> {
  if samples.is_empty() || source_rate == 0 || source_rate < MIN_SOURCE_RATE {
    return Err(Error::UnsupportedFormat);
  }
  if source_rate == WORK_RATE {
    return Ok(samples.to_vec());
  }

  let filtered: Vec<f32>;
  let src = if source_rate > WORK_RATE {
    // 抗混叠：截止取目标奈奎斯特的 0.9 倍。
    let cutoff = (WORK_RATE as f32 * 0.45) / source_rate as f32;
    let coeffs = lowpass(cutoff, 33);
    filtered = fir(samples, &coeffs);
    filtered.as_slice()
  } else {
    samples
  };

  let ratio = WORK_RATE as f64 / source_rate as f64;
  let out_len = ((src.len() as f64) * ratio).floor() as usize;
  if out_len > MAX_OUT_SAMPLES {
    return Err(Error::UnsupportedFormat);
  }
  let mut out = Vec::with_capacity(out_len);
  let last = src.len() - 1;
  for i in 0..out_len {
    let pos = i as f64 / ratio;
    let p0 = pos.floor() as usize;
    let p1 = (p0 + 1).min(last);
    let frac = (pos - p0 as f64) as f32;
    out.push(src[p0] + (src[p1] - src[p0]) * frac);
  }
  Ok(out)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn identity_when_same_rate() {
    let input = vec![0.0f32, 0.5, 1.0, 0.5];
    let out = to_work_rate(&input, WORK_RATE).unwrap();
    assert_eq!(out, input);
  }

  #[test]
  fn downsamples_length() {
    let input = vec![0.0f32; 16_000];
    let out = to_work_rate(&input, 16_000).unwrap();
    assert!((out.len() as i64 - 8_000).abs() <= 2);
  }

  #[test]
  fn rejects_absurd_source_rate() {
    // 声明 1 Hz 的畸形 WAV：放大倍数 8000，不设限时输出长度饱和成 usize 极值，
    // wasm32 上 `Vec::with_capacity` 直接 trap。
    assert_eq!(
      to_work_rate(&[0.0f32; 1024], 1),
      Err(Error::UnsupportedFormat)
    );
    assert_eq!(
      to_work_rate(&[0.0f32; 1024], 0),
      Err(Error::UnsupportedFormat)
    );
  }

  #[test]
  fn rejects_output_beyond_cap() {
    // 采样率合法（刚过下限）但数据量超过 20 分钟上限：拒绝而不是分配天文数字内存。
    let input = vec![0.0f32; MAX_OUT_SAMPLES / 2 + 1];
    assert_eq!(to_work_rate(&input, 4000), Err(Error::UnsupportedFormat));
    // 接近但未超上限的输入正常通过。
    let input = vec![0.0f32; 4_000];
    assert_eq!(to_work_rate(&input, 4000).unwrap().len(), 8_000);
  }
}
