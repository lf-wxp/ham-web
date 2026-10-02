//! 天线尺寸估算：Yagi 振子与垂直天线辐射体 / 地网的长度（教学近似）。

/// Yagi 各振子尺寸（米）。
#[derive(Debug, Clone, PartialEq)]
pub struct YagiDims {
  /// 反射器长度。
  pub reflector: f64,
  /// 激励振子长度。
  pub driven: f64,
  /// 引向器长度（从激励侧向远端逐个变短）。
  pub directors: Vec<f64>,
  /// 反射器与激励振子间距。
  pub refl_spacing: f64,
  /// 相邻引向器间距。
  pub dir_spacing: f64,
}

/// 计算 Yagi 振子尺寸（米）。
///
/// `directors` 为引向器数量（0 时退化为反射器 + 激励的二单元）。
#[must_use]
pub fn yagi_dims(freq_mhz: f64, directors: usize) -> Option<YagiDims> {
  if freq_mhz <= 0.0 {
    return None;
  }
  let lambda = 300.0 / freq_mhz;
  let reflector = lambda * 0.5 * 1.05;
  let driven = lambda * 0.5 * 0.95;
  let directors = (0..directors)
    .map(|i| lambda * 0.5 * (0.90 - 0.02 * i as f64).max(0.80))
    .collect();
  Some(YagiDims {
    reflector,
    driven,
    directors,
    refl_spacing: lambda * 0.2,
    dir_spacing: lambda * 0.15,
  })
}

/// 垂直天线尺寸：返回（辐射体长度，单根地网长度，米）。
#[must_use]
pub fn vertical_dims(freq_mhz: f64, k: f64) -> Option<(f64, f64)> {
  if freq_mhz <= 0.0 || !(0.5..=1.0).contains(&k) {
    return None;
  }
  let lambda = 300.0 / freq_mhz;
  // 辐射体约 1/4λ（乘缩短系数 k），地网略长于 1/4λ。
  Some((lambda * 0.25 * k, lambda * 0.25 * 1.05))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn yagi_dims_order() {
    let d = yagi_dims(14.2, 3).expect("valid");
    // 反射器最长，激励次之，引向器最短且逐个变短。
    assert!(d.reflector > d.driven);
    assert_eq!(d.directors.len(), 3);
    assert!(d.driven > d.directors[0]);
    assert!(d.directors[0] > d.directors[2]);
    assert!(d.refl_spacing > d.dir_spacing);
  }

  #[test]
  fn yagi_without_directors() {
    let d = yagi_dims(14.2, 0).expect("valid");
    assert!(d.directors.is_empty());
    assert!(d.reflector > d.driven);
  }

  #[test]
  fn vertical_dims_and_guards() {
    let (radiator, radial) = vertical_dims(14.2, 0.95).expect("valid");
    assert!(radial > radiator);
    assert!(vertical_dims(0.0, 0.95).is_none());
    assert!(vertical_dims(14.2, 0.4).is_none());
  }
}
