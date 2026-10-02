//! 线性插值重采样到工作采样率，降采样前做抗混叠低通。

use super::{Error, filter};

/// 将任意采样率线性重采样到 `target_rate`。
///
/// 源采样率高于目标时先做抗混叠低通；相等时原样返回。
pub fn to_rate(samples: &[f32], source_rate: u32, target_rate: u32) -> Result<Vec<f32>, Error> {
  if samples.is_empty() {
    return Err(Error::TooShort);
  }
  if source_rate == 0 {
    return Err(Error::UnsupportedFormat);
  }
  if source_rate == target_rate {
    return Ok(samples.to_vec());
  }

  let filtered: Vec<f32>;
  let samples = if source_rate > target_rate {
    // 抗混叠：截止频率取目标奈奎斯特的 0.9 倍。
    let cutoff = (target_rate as f32 * 0.45) / source_rate as f32;
    let coeffs = filter::lowpass(cutoff, 33);
    filtered = filter::apply(samples, &coeffs);
    filtered.as_slice()
  } else {
    samples
  };

  let ratio = target_rate as f64 / source_rate as f64;
  let out_len = ((samples.len() as f64) * ratio).floor() as usize;
  let mut out = Vec::with_capacity(out_len);
  let last = samples.len() - 1;
  for i in 0..out_len {
    let pos = i as f64 / ratio;
    let p0 = pos.floor() as usize;
    let p1 = (p0 + 1).min(last);
    let frac = (pos - p0 as f64) as f32;
    out.push(samples[p0] + (samples[p1] - samples[p0]) * frac);
  }
  Ok(out)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn identity_when_same_rate() {
    let input = vec![0.0f32, 0.5, 1.0, 0.5];
    let out = to_rate(&input, 8000, 8000).unwrap();
    assert_eq!(out, input);
  }

  #[test]
  fn upsamples_length() {
    let input = vec![0.0f32; 100];
    let out = to_rate(&input, 4000, 8000).unwrap();
    assert!((out.len() as i64 - 200).abs() <= 2);
  }

  #[test]
  fn rejects_empty() {
    assert_eq!(to_rate(&[], 8000, 8000), Err(Error::TooShort));
  }
}
