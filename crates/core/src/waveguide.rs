//! 矩形波导：截止频率、模式、导波波长、波阻抗、导体损耗、截止以下的衰减。
//!
//! 只做**矩形**波导的 `TE_mn` / `TM_mn` 截止与主模 `TE10` 的传输参数；不含圆波导、脊波导，
//! 也不含介质损耗（`tanδ`）与表面粗糙度。导体损耗是光滑理想表面的**理论下限**，
//! 实际波导（尤其是铝材、没镀银的内壁）通常高出一截。
//!
//! 公式（Pozar《Microwave Engineering》第 3 章，填充介质相对介电常数 `εr`）：
//!
//! ```text
//! fc_mn   = c / (2 √εr) · √((m/a)² + (n/b)²)
//! λg      = (c / (f √εr)) / √(1 − (fc/f)²)
//! Z_TE10  = η / √(1 − (fc/f)²)，η = η0 / √εr
//! αc_TE10 = Rs / (a³ b β k η) · (2 b π² + a³ k²)        [Np/m]，Rs = √(π f μ0 / σ)
//! α(f<fc) = (2π fc √εr / c) · √(1 − (f/fc)²)            [Np/m]
//! ```
//!
//! 标准尺寸与推荐频段取自 Wikipedia 的矩形波导表（EIA WR 系列），计算的截止频率与该表
//! 公布值逐项相符（最大偏差 0.16%，来自表中取整）。

/// 光速（m/s，精确值）。
pub const C: f64 = 299_792_458.0;
/// 自由空间波阻抗 η0（Ω，CODATA 2018）。
pub const ETA0: f64 = 376.730_313_668;
/// 真空磁导率 μ0（H/m，CODATA 2018）。
pub const MU0: f64 = 1.256_637_062_12e-6;
/// 退火铜标准电导率（S/m，IACS 100% 的定义值）。
pub const SIGMA_COPPER: f64 = 5.8e7;
/// Np → dB。
pub const NP_TO_DB: f64 = 8.685_889_638_065_035;

/// 常规单模工作带下沿 / 上沿相对 TE10 截止频率的倍数。
pub const BAND_LOW: f64 = 1.25;
pub const BAND_HIGH: f64 = 1.9;

/// 一个 EIA 标准尺寸。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Standard {
  /// 如 `WR-90`。
  pub name: &'static str,
  /// 内宽 a（英寸）。
  pub a_in: f64,
  /// 内高 b（英寸）。
  pub b_in: f64,
  /// 公布的推荐工作频段（GHz）。
  pub band_ghz: (f64, f64),
}

impl Standard {
  #[must_use]
  pub fn a_mm(&self) -> f64 {
    self.a_in * 25.4
  }
  #[must_use]
  pub fn b_mm(&self) -> f64 {
    self.b_in * 25.4
  }
}

/// 常用的 EIA WR 标准尺寸（从大到小）。
pub const STANDARDS: &[Standard] = &[
  Standard {
    name: "WR-650",
    a_in: 6.5,
    b_in: 3.25,
    band_ghz: (1.15, 1.72),
  },
  Standard {
    name: "WR-510",
    a_in: 5.1,
    b_in: 2.55,
    band_ghz: (1.45, 2.20),
  },
  Standard {
    name: "WR-430",
    a_in: 4.3,
    b_in: 2.15,
    band_ghz: (1.72, 2.60),
  },
  Standard {
    name: "WR-340",
    a_in: 3.4,
    b_in: 1.7,
    band_ghz: (2.20, 3.30),
  },
  Standard {
    name: "WR-284",
    a_in: 2.84,
    b_in: 1.34,
    band_ghz: (2.60, 3.95),
  },
  Standard {
    name: "WR-229",
    a_in: 2.29,
    b_in: 1.145,
    band_ghz: (3.30, 4.90),
  },
  Standard {
    name: "WR-187",
    a_in: 1.872,
    b_in: 0.872,
    band_ghz: (3.95, 5.85),
  },
  Standard {
    name: "WR-159",
    a_in: 1.59,
    b_in: 0.795,
    band_ghz: (4.90, 7.05),
  },
  Standard {
    name: "WR-137",
    a_in: 1.372,
    b_in: 0.622,
    band_ghz: (5.85, 8.20),
  },
  Standard {
    name: "WR-112",
    a_in: 1.122,
    b_in: 0.497,
    band_ghz: (7.05, 10.0),
  },
  Standard {
    name: "WR-90",
    a_in: 0.9,
    b_in: 0.4,
    band_ghz: (8.2, 12.4),
  },
  Standard {
    name: "WR-75",
    a_in: 0.75,
    b_in: 0.375,
    band_ghz: (10.0, 15.0),
  },
  Standard {
    name: "WR-62",
    a_in: 0.622,
    b_in: 0.311,
    band_ghz: (12.4, 18.0),
  },
  Standard {
    name: "WR-42",
    a_in: 0.42,
    b_in: 0.17,
    band_ghz: (18.0, 26.5),
  },
  Standard {
    name: "WR-28",
    a_in: 0.28,
    b_in: 0.14,
    band_ghz: (26.5, 40.0),
  },
  Standard {
    name: "WR-22",
    a_in: 0.224,
    b_in: 0.112,
    band_ghz: (33.0, 50.0),
  },
  Standard {
    name: "WR-15",
    a_in: 0.148,
    b_in: 0.074,
    band_ghz: (50.0, 75.0),
  },
  Standard {
    name: "WR-10",
    a_in: 0.1,
    b_in: 0.05,
    band_ghz: (75.0, 110.0),
  },
];

/// 按名字找标准尺寸。
#[must_use]
pub fn standard(name: &str) -> Option<&'static Standard> {
  STANDARDS.iter().find(|s| s.name == name)
}

/// 推荐频段包含 `freq_ghz` 的标准尺寸（可能不止一个，相邻尺寸的频段有重叠）。
#[must_use]
pub fn standards_for(freq_ghz: f64) -> Vec<&'static Standard> {
  STANDARDS
    .iter()
    .filter(|s| s.band_ghz.0 <= freq_ghz && freq_ghz <= s.band_ghz.1)
    .collect()
}

/// 几何与填充。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
  /// 内宽（米，长边）。
  pub a_m: f64,
  /// 内高（米，短边）。
  pub b_m: f64,
  /// 填充介质相对介电常数（空气 = 1）。
  pub er: f64,
}

/// 输入不合法的原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WgError {
  /// 尺寸不是正的有限数。
  Size,
  /// 内高比内宽大：按约定 a 是长边（TE10 的宽边）。
  HeightExceedsWidth,
  /// 介电常数不在 [1, 100]。
  Permittivity,
  /// 频率不是正的有限数。
  Frequency,
  /// 电导率不是正的有限数。
  Conductivity,
}

impl Geometry {
  fn validate(&self) -> Result<(), WgError> {
    let pos = |x: f64| x.is_finite() && x > 0.0;
    if !pos(self.a_m) || !pos(self.b_m) {
      return Err(WgError::Size);
    }
    if self.b_m > self.a_m {
      return Err(WgError::HeightExceedsWidth);
    }
    if !(self.er.is_finite() && (1.0..=100.0).contains(&self.er)) {
      return Err(WgError::Permittivity);
    }
    Ok(())
  }
}

/// `TE_mn` / `TM_mn` 的截止频率（Hz）。`(0, 0)` 不是模式、几何不合法，都返回 `None`。
#[must_use]
pub fn cutoff_hz(g: &Geometry, m: u32, n: u32) -> Option<f64> {
  if m == 0 && n == 0 {
    return None;
  }
  // 本函数是 `pub`：调用方可能先算了别的量再进来，尺寸为 0 会算出 `inf` 这种
  // 「看着像结果」的数。校验成本是三次浮点判断，不值得为省它留一个坑。
  g.validate().ok()?;
  let (m, n) = (f64::from(m), f64::from(n));
  let cutoff = C / (2.0 * g.er.sqrt()) * (m / g.a_m).hypot(n / g.b_m);
  cutoff.is_finite().then_some(cutoff)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
  Te,
  Tm,
}

/// 一个模式。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mode {
  pub family: Family,
  pub m: u32,
  pub n: u32,
  pub fc_hz: f64,
}

/// `lowest_modes` 的 `count` 防御性上限：枚举成本是 O(count²)，界面正常只列最低几个模式。
/// 调用方若把用户输入直接当 `count` 传进来，超出的部分按上限截断，不做超大规模的枚举。
pub const MAX_MODES: usize = 128;

/// 截止频率最低的 `count` 个模式（升序；同频时 TE 在前，再让 n 小的在前）。
///
/// `TM_mn` 要求 m、n 都 ≥ 1；`TE_mn` 只要不同时为 0。
/// `count` 超过 [`MAX_MODES`] 时按上限截断（防御性上限，正常调用远小于它）。
///
/// # Errors
///
/// 几何不合法。
pub fn lowest_modes(g: &Geometry, count: usize) -> Result<Vec<Mode>, WgError> {
  g.validate()?;
  let count = count.min(MAX_MODES);
  // m、n 取到 count 就一定够：fc 随 m、n 单调增，前 count 个模式的指标不会超过 count。
  let limit = u32::try_from(count).unwrap_or(u32::MAX).max(1);
  let mut all = Vec::new();
  for m in 0..=limit {
    for n in 0..=limit {
      let Some(fc_hz) = cutoff_hz(g, m, n) else {
        continue;
      };
      all.push(Mode {
        family: Family::Te,
        m,
        n,
        fc_hz,
      });
      if m >= 1 && n >= 1 {
        all.push(Mode {
          family: Family::Tm,
          m,
          n,
          fc_hz,
        });
      }
    }
  }
  all.sort_by(|x, y| {
    // 同频判定留一点相对容差：a = 2b 时 TE20 与 TE01 的 fc 在浮点上可能差一个 ulp。
    let tie = (x.fc_hz - y.fc_hz).abs() <= 1e-9 * x.fc_hz;
    if tie {
      // 并列时 TE 在前、再让 n 小的（即 TE10 在 TE01 前）在前，这是通行的写法。
      (x.family as u8, x.n, x.m).cmp(&(y.family as u8, y.n, y.m))
    } else {
      x.fc_hz.total_cmp(&y.fc_hz)
    }
  });
  all.truncate(count);
  Ok(all)
}

/// 主模 TE10 的截止频率（Hz）。
///
/// # Errors
///
/// 几何不合法。
pub fn te10_cutoff_hz(g: &Geometry) -> Result<f64, WgError> {
  g.validate()?;
  Ok(C / (2.0 * g.er.sqrt() * g.a_m))
}

/// 单模工作范围：TE10 截止到「下一个模式」的截止（Hz）。
///
/// # Errors
///
/// 几何不合法。
pub fn single_mode_range_hz(g: &Geometry) -> Result<(f64, f64), WgError> {
  let lo = te10_cutoff_hz(g)?;
  // 下一个模式是 TE20 与 TE01 中较低的那个（TE11 / TM11 都更高）。
  let hi = cutoff_hz(g, 2, 0)
    .unwrap_or(f64::INFINITY)
    .min(cutoff_hz(g, 0, 1).unwrap_or(f64::INFINITY));
  Ok((lo, hi))
}

/// 工作频率处的 TE10 参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operating {
  /// 高于截止：能传输。
  Propagating {
    /// 填充介质里的平面波波长（米）。
    wavelength_m: f64,
    /// 导波波长（米）。
    guide_wavelength_m: f64,
    /// 相速 / 光速（> 1 是正常的，信息不是以相速传播）。
    phase_velocity_ratio: f64,
    /// 群速 / 光速。
    group_velocity_ratio: f64,
    /// TE10 波阻抗（Ω）。
    wave_impedance_ohm: f64,
    /// 导体损耗（dB/m）；没给电导率时为 `None`。
    conductor_loss_db_per_m: Option<f64>,
    /// 是否落在单模范围里（低于下一个模式的截止）。
    single_mode: bool,
    /// 是否落在常规推荐带 [`BAND_LOW`]·fc ..= [`BAND_HIGH`]·fc 内。
    in_recommended_band: bool,
  },
  /// 低于截止：指数衰减（倏逝波）。
  BelowCutoff {
    /// 衰减（dB/m）。
    attenuation_db_per_m: f64,
    /// 长度正好等于一个截止波长（λc = 2a√εr⁻¹…，见实现）时的衰减（dB）。
    /// f ≪ fc 时趋近 54.6 dB，这就是「截止以下约 54.6 dB / λc」的经验数。
    db_per_cutoff_wavelength: f64,
  },
  /// 恰好等于截止频率：既不传输也谈不上衰减的闭式值。
  AtCutoff,
}

/// 计算 `f_hz` 处的 TE10 参数。`sigma_s_per_m` 给了就算导体损耗。
///
/// # Errors
///
/// 几何、频率或电导率不合法。
pub fn operate(g: &Geometry, f_hz: f64, sigma_s_per_m: Option<f64>) -> Result<Operating, WgError> {
  g.validate()?;
  if !(f_hz.is_finite() && f_hz > 0.0) {
    return Err(WgError::Frequency);
  }
  if sigma_s_per_m.is_some_and(|s| !(s.is_finite() && s > 0.0)) {
    return Err(WgError::Conductivity);
  }
  let fc = te10_cutoff_hz(g)?;
  let er_sqrt = g.er.sqrt();
  let c_medium = C / er_sqrt;

  if (f_hz - fc).abs() <= 1e-12 * fc {
    return Ok(Operating::AtCutoff);
  }
  if f_hz < fc {
    let alpha = (2.0 * std::f64::consts::PI * fc / c_medium) * (1.0 - (f_hz / fc).powi(2)).sqrt();
    // 截止波长 λc = c_medium / fc = 2a。
    let lambda_c = c_medium / fc;
    return Ok(Operating::BelowCutoff {
      attenuation_db_per_m: alpha * NP_TO_DB,
      db_per_cutoff_wavelength: alpha * lambda_c * NP_TO_DB,
    });
  }

  let root = (1.0 - (fc / f_hz).powi(2)).sqrt();
  let wavelength_m = c_medium / f_hz;
  let eta = ETA0 / er_sqrt;
  let (_, next) = single_mode_range_hz(g)?;

  let conductor_loss_db_per_m = sigma_s_per_m.map(|sigma| {
    let rs = (std::f64::consts::PI * f_hz * MU0 / sigma).sqrt();
    let k = 2.0 * std::f64::consts::PI * f_hz / c_medium;
    let beta = k * root;
    let (a, b) = (g.a_m, g.b_m);
    let np = rs / (a.powi(3) * b * beta * k * eta)
      * (2.0 * b * std::f64::consts::PI.powi(2) + a.powi(3) * k * k);
    np * NP_TO_DB
  });

  Ok(Operating::Propagating {
    wavelength_m,
    guide_wavelength_m: wavelength_m / root,
    phase_velocity_ratio: 1.0 / (er_sqrt * root),
    group_velocity_ratio: root / er_sqrt,
    wave_impedance_ohm: eta / root,
    conductor_loss_db_per_m,
    single_mode: f_hz < next,
    in_recommended_band: (BAND_LOW * fc..=BAND_HIGH * fc).contains(&f_hz),
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn guide(a_mm: f64, b_mm: f64) -> Geometry {
    Geometry {
      a_m: a_mm / 1000.0,
      b_m: b_mm / 1000.0,
      er: 1.0,
    }
  }

  fn wr90() -> Geometry {
    guide(22.86, 10.16)
  }

  fn near(a: f64, b: f64, rel: f64) {
    assert!(((a - b) / b).abs() <= rel, "{a} vs {b}");
  }

  /// 公布的截止频率（GHz），取自 Wikipedia 矩形波导表（EIA 行）。
  const PUBLISHED_FC_GHZ: &[(&str, f64)] = &[
    ("WR-650", 0.908),
    ("WR-510", 1.157),
    ("WR-430", 1.372),
    ("WR-340", 1.736),
    ("WR-284", 2.078),
    ("WR-229", 2.577),
    ("WR-187", 3.153),
    ("WR-159", 3.712),
    ("WR-137", 4.301),
    ("WR-112", 5.260),
    ("WR-90", 6.557),
    ("WR-75", 7.869),
    ("WR-62", 9.488),
    ("WR-42", 14.051),
    ("WR-28", 21.077),
    ("WR-22", 26.346),
    ("WR-15", 39.875),
    ("WR-10", 59.015),
  ];

  fn std_geometry(s: &Standard) -> Geometry {
    Geometry {
      a_m: s.a_in * 0.0254,
      b_m: s.b_in * 0.0254,
      er: 1.0,
    }
  }

  #[test]
  fn every_standard_size_reproduces_the_published_cutoff() {
    assert_eq!(PUBLISHED_FC_GHZ.len(), STANDARDS.len());
    for &(name, want) in PUBLISHED_FC_GHZ {
      let s = standard(name).unwrap_or_else(|| panic!("{name} 不在表里"));
      let got = te10_cutoff_hz(&std_geometry(s)).unwrap() / 1e9;
      // 公布值取整到 0.001 GHz，且个别行用的是取整后的毫米尺寸：容差 0.2%。
      near(got, want, 0.002);
    }
  }

  #[test]
  fn wr_number_is_the_inner_width_in_hundredths_of_an_inch() {
    for s in STANDARDS {
      let n: f64 = s.name.trim_start_matches("WR-").parse().unwrap();
      // 命名规则：WR 后面的数 = 内宽（英寸）× 100 取整（WR-187 的 1.872、WR-22 的 0.224 都是取整后的）。
      assert_eq!((s.a_in * 100.0).round(), n, "{}", s.name);
      assert!(
        s.b_in <= s.a_in && s.b_in >= s.a_in * 0.4,
        "{} 高宽比异常",
        s.name
      );
    }
  }

  #[test]
  fn published_bands_sit_at_the_conventional_multiples_of_cutoff() {
    // 公布的推荐频段落在 fc 的 1.2–1.4 倍到 1.85–1.95 倍之间；录入时敲错一位数会立刻暴露。
    for s in STANDARDS {
      let fc = te10_cutoff_hz(&std_geometry(s)).unwrap() / 1e9;
      let (lo, hi) = s.band_ghz;
      assert!(
        (1.2..=1.4).contains(&(lo / fc)),
        "{} 下沿 {}",
        s.name,
        lo / fc
      );
      assert!(
        (1.85..=1.95).contains(&(hi / fc)),
        "{} 上沿 {}",
        s.name,
        hi / fc
      );
    }
  }

  #[test]
  fn millimetre_sizes_agree_with_the_tables_rounded_values() {
    // 表里印的毫米尺寸（四舍五入）。
    //
    // WR-284 的 b 故意不在这里：该表印的是 34.94 mm，但同一行的英寸列是 1.340 in = 34.036 mm
    // （EIA 标准值就是 1.340 in），毫米列是来源表自己的笔误。本模块取英寸值。
    assert!((standard("WR-284").unwrap().b_mm() - 34.036).abs() < 0.001);
    for (name, a, b) in [
      ("WR-284", 72.14, 34.036),
      ("WR-187", 47.55, 22.2),
      ("WR-137", 34.9, 15.8),
      ("WR-112", 28.5, 12.6),
      ("WR-90", 22.9, 10.2),
      ("WR-42", 10.7, 4.32),
      ("WR-28", 7.11, 3.56),
    ] {
      let s = standard(name).unwrap();
      assert!((s.a_mm() - a).abs() < 0.06, "{name} a {}", s.a_mm());
      assert!((s.b_mm() - b).abs() < 0.06, "{name} b {}", s.b_mm());
    }
  }

  #[test]
  fn which_standard_covers_the_amateur_microwave_bands() {
    let names = |ghz: f64| {
      standards_for(ghz)
        .iter()
        .map(|s| s.name)
        .collect::<Vec<_>>()
    };
    assert!(names(10.368).contains(&"WR-90"));
    assert!(names(5.76).contains(&"WR-187"));
    assert!(names(24.048).contains(&"WR-42"));
    assert!(names(47.088).contains(&"WR-22"));
    assert!(names(2.304).contains(&"WR-430") || names(2.304).contains(&"WR-340"));
    // 刚好在两个频段的交界处会落进两个尺寸，不是 bug。
    assert!(names(12.4).contains(&"WR-90") && names(12.4).contains(&"WR-62"));
    assert!(names(0.1).is_empty() && names(1000.0).is_empty());
  }

  #[test]
  fn wr90_numbers_match_the_independent_script() {
    let g = wr90();
    near(te10_cutoff_hz(&g).unwrap() / 1e9, 6.557_140_376, 1e-9);
    let modes = lowest_modes(&g, 5).unwrap();
    let got: Vec<_> = modes
      .iter()
      .map(|m| {
        (
          m.family,
          m.m,
          m.n,
          (m.fc_hz / 1e9 * 1000.0).round() / 1000.0,
        )
      })
      .collect();
    // TE10 6.557、TE20 13.114、TE01 14.754、TE11 与 TM11 同为 16.145。
    assert_eq!(
      got,
      vec![
        (Family::Te, 1, 0, 6.557),
        (Family::Te, 2, 0, 13.114),
        (Family::Te, 0, 1, 14.754),
        (Family::Te, 1, 1, 16.145),
        (Family::Tm, 1, 1, 16.145),
      ]
    );
    let (lo, hi) = single_mode_range_hz(&g).unwrap();
    near(lo / 1e9, 6.557_140_376, 1e-9);
    near(hi / 1e9, 13.114_280_752, 1e-9); // TE20 比 TE01 低
  }

  #[test]
  fn wr90_at_10_ghz_matches_the_independent_script() {
    let Operating::Propagating {
      wavelength_m,
      guide_wavelength_m,
      phase_velocity_ratio,
      group_velocity_ratio,
      wave_impedance_ohm,
      conductor_loss_db_per_m,
      single_mode,
      in_recommended_band,
    } = operate(&wr90(), 10e9, Some(SIGMA_COPPER)).unwrap()
    else {
      panic!("10 GHz 应当在传输");
    };
    near(wavelength_m * 1000.0, 29.979_245_8, 1e-9);
    near(guide_wavelength_m * 1000.0, 39.707_119, 1e-6);
    near(phase_velocity_ratio, 1.324_486_929, 1e-8);
    near(group_velocity_ratio, 0.755_009_338, 1e-8);
    // 相速 × 群速 = c²。
    near(phase_velocity_ratio * group_velocity_ratio, 1.0, 1e-12);
    near(wave_impedance_ohm, 498.974_375, 1e-7);
    // 铜，10 GHz：Rs = 0.02609 Ω，αc = 0.012478 Np/m = 0.108385 dB/m。
    near(conductor_loss_db_per_m.unwrap(), 0.108_385_337, 1e-6);
    assert!(single_mode && in_recommended_band);
  }

  #[test]
  fn conductor_loss_has_two_independent_closed_forms_that_agree() {
    // Pozar (3.96) 与 Ramo/Collin 的写法是两个不同的推导，代数上相等：
    // αc = Rs / (η b √(1−(fc/f)²)) · (1 + 2b/a · (fc/f)²)。
    for (g, f, sigma) in [
      (wr90(), 10e9, 5.8e7),
      (wr90(), 9e9, 3.5e7),
      (guide(34.85, 15.8), 6e9, 5.8e7),
      (guide(10.668, 4.318), 24.048e9, 5.8e7),
      (Geometry { er: 2.2, ..wr90() }, 8e9, 5.8e7),
    ] {
      let Operating::Propagating {
        conductor_loss_db_per_m,
        ..
      } = operate(&g, f, Some(sigma)).unwrap()
      else {
        panic!("应当在传输");
      };
      let fc = te10_cutoff_hz(&g).unwrap();
      let eta = ETA0 / g.er.sqrt();
      let rs = (std::f64::consts::PI * f * MU0 / sigma).sqrt();
      let r = (fc / f).powi(2);
      let np = rs / (eta * g.b_m * (1.0 - r).sqrt()) * (1.0 + 2.0 * g.b_m / g.a_m * r);
      near(conductor_loss_db_per_m.unwrap(), np * NP_TO_DB, 1e-9);
    }
  }

  #[test]
  fn worse_conductors_lose_more_in_proportion_to_one_over_root_sigma() {
    let loss = |sigma| match operate(&wr90(), 10e9, Some(sigma)).unwrap() {
      Operating::Propagating {
        conductor_loss_db_per_m: Some(l),
        ..
      } => l,
      _ => panic!(),
    };
    // 损耗 ∝ Rs ∝ 1/√σ：电导率降到 1/4，损耗翻倍。
    near(loss(5.8e7 / 4.0) / loss(5.8e7), 2.0, 1e-9);
  }

  #[test]
  fn below_cutoff_is_54_6_db_per_cutoff_wavelength_far_below() {
    // 经验数：f ≪ fc 时，截止以下每个「截止波长」衰减 2π·8.686 = 54.575 dB。
    // 业余里常说「圆波导截止以下约 32 dB / 直径」是同一件事：λc = 1.706 D → 54.575 / 1.706 = 31.99。
    let Operating::BelowCutoff {
      db_per_cutoff_wavelength,
      attenuation_db_per_m,
    } = operate(&wr90(), 1e6, None).unwrap()
    else {
      panic!("1 MHz 远低于截止");
    };
    near(db_per_cutoff_wavelength, 54.575_054, 1e-6);
    // 截止波长 = 2a = 45.72 mm → 54.575 / 0.04572 = 1193.7 dB/m。
    near(attenuation_db_per_m, 54.575_054 / 0.045_72, 1e-6);
    near(54.575_054 / 1.706, 31.99, 1e-3);
    // WR-90 在 5 GHz：独立脚本 88.9095 Np/m = 772.258 dB/m。
    let Operating::BelowCutoff {
      attenuation_db_per_m,
      ..
    } = operate(&wr90(), 5e9, None).unwrap()
    else {
      panic!();
    };
    near(attenuation_db_per_m, 772.258_237_6, 1e-6);
  }

  #[test]
  fn attenuation_decreases_continuously_to_zero_at_cutoff() {
    let g = wr90();
    let fc = te10_cutoff_hz(&g).unwrap();
    let at = |f| match operate(&g, f, None).unwrap() {
      Operating::BelowCutoff {
        attenuation_db_per_m,
        ..
      } => attenuation_db_per_m,
      _ => panic!(),
    };
    assert!(at(0.5 * fc) > at(0.9 * fc) && at(0.9 * fc) > at(0.999 * fc));
    // 趋近于 0：α ∝ √(1 − (f/fc)²)，在 1−1e-10 处只剩 0.017 dB/m（全程最大是 1194 dB/m 量级）。
    assert!(at((1.0 - 1e-10) * fc) < 0.05);
    assert!(at((1.0 - 1e-6) * fc) < 2.0);
    assert!(matches!(
      operate(&g, fc, None).unwrap(),
      Operating::AtCutoff
    ));
  }

  #[test]
  fn dielectric_filling_lowers_cutoff_and_shortens_the_wavelength_by_root_er() {
    let air = wr90();
    let ptfe = Geometry { er: 2.1, ..air };
    near(
      te10_cutoff_hz(&ptfe).unwrap(),
      te10_cutoff_hz(&air).unwrap() / 2.1f64.sqrt(),
      1e-12,
    );
    let w = |g: &Geometry| match operate(g, 10e9, None).unwrap() {
      Operating::Propagating { wavelength_m, .. } => wavelength_m,
      _ => panic!(),
    };
    near(w(&ptfe), w(&air) / 2.1f64.sqrt(), 1e-12);
  }

  #[test]
  fn single_mode_and_recommended_band_flags_follow_the_cutoffs() {
    let g = wr90();
    let flags = |f| match operate(&g, f, None).unwrap() {
      Operating::Propagating {
        single_mode,
        in_recommended_band,
        ..
      } => (single_mode, in_recommended_band),
      _ => panic!(),
    };
    assert_eq!(flags(7e9), (true, false)); // 刚过截止，色散大，不在推荐带
    assert_eq!(flags(10e9), (true, true));
    assert_eq!(flags(12.4e9), (true, true));
    assert_eq!(flags(13.5e9), (false, false)); // 已过 TE20 截止 13.114 GHz：多模
  }

  #[test]
  fn square_guide_has_degenerate_te10_and_te01() {
    let sq = guide(20.0, 20.0);
    let m = lowest_modes(&sq, 3).unwrap();
    assert_eq!((m[0].m, m[0].n, m[1].m, m[1].n), (1, 0, 0, 1));
    near(m[0].fc_hz, m[1].fc_hz, 1e-12);
    // 单模范围到 TE20 / TE01 的下一个模式为止：方波导 TE01 与 TE10 同频，上沿就是 fc10 自己。
    let (lo, hi) = single_mode_range_hz(&sq).unwrap();
    assert!(hi <= 2.0 * lo + 1.0);
  }

  #[test]
  fn tm_modes_need_both_indices_and_zero_zero_is_not_a_mode() {
    let g = wr90();
    assert_eq!(cutoff_hz(&g, 0, 0), None);
    let modes = lowest_modes(&g, 40).unwrap();
    assert!(
      modes
        .iter()
        .all(|m| m.family == Family::Te || (m.m >= 1 && m.n >= 1))
    );
    assert!(
      modes
        .windows(2)
        .all(|w| w[0].fc_hz <= w[1].fc_hz * (1.0 + 1e-9))
    );
    assert!(!modes.iter().any(|m| m.m == 0 && m.n == 0));
  }

  #[test]
  fn hostile_mode_counts_are_capped_instead_of_enumerated() {
    // usize::MAX 不会被当真去枚举：结果截断到上限，不 panic、不耗尽内存。
    let modes = lowest_modes(&wr90(), usize::MAX).unwrap();
    assert_eq!(modes.len(), MAX_MODES);
    assert!(
      modes
        .windows(2)
        .all(|w| w[0].fc_hz <= w[1].fc_hz * (1.0 + 1e-9))
    );
  }

  #[test]
  fn rejects_nonsense_with_a_specific_reason() {
    let ok = wr90();
    assert_eq!(
      operate(&Geometry { a_m: 0.0, ..ok }, 10e9, None),
      Err(WgError::Size)
    );
    assert_eq!(
      operate(&Geometry { b_m: -1.0, ..ok }, 10e9, None),
      Err(WgError::Size)
    );
    assert_eq!(
      operate(
        &Geometry {
          a_m: f64::NAN,
          ..ok
        },
        10e9,
        None
      ),
      Err(WgError::Size)
    );
    assert_eq!(
      operate(
        &Geometry {
          a_m: 0.01,
          b_m: 0.02,
          er: 1.0
        },
        10e9,
        None
      ),
      Err(WgError::HeightExceedsWidth)
    );
    assert_eq!(
      operate(&Geometry { er: 0.5, ..ok }, 10e9, None),
      Err(WgError::Permittivity)
    );
    assert_eq!(
      operate(&Geometry { er: 101.0, ..ok }, 10e9, None),
      Err(WgError::Permittivity)
    );
    assert_eq!(operate(&ok, 0.0, None), Err(WgError::Frequency));
    assert_eq!(operate(&ok, f64::INFINITY, None), Err(WgError::Frequency));
    assert_eq!(operate(&ok, 10e9, Some(0.0)), Err(WgError::Conductivity));
    assert_eq!(
      operate(&ok, 10e9, Some(f64::NAN)),
      Err(WgError::Conductivity)
    );
    assert_eq!(
      lowest_modes(&Geometry { a_m: 0.0, ..ok }, 3),
      Err(WgError::Size)
    );
    assert_eq!(
      te10_cutoff_hz(&Geometry { er: 0.0, ..ok }),
      Err(WgError::Permittivity)
    );
  }

  #[test]
  fn cutoff_hz_rejects_geometry_that_is_not_usable() {
    // 尺寸为 0 会算出 `inf`：这种「看着像结果」的数不能从 `cutoff_hz` 出去。
    for bad in [
      Geometry {
        a_m: 0.0,
        b_m: 0.0,
        er: 1.0,
      },
      Geometry {
        a_m: 22.86e-3,
        b_m: 0.0,
        er: 1.0,
      },
      // 高边比长边长：不是矩形波导的几何约定。
      Geometry {
        a_m: 10.16e-3,
        b_m: 22.86e-3,
        er: 1.0,
      },
      Geometry {
        a_m: f64::NAN,
        b_m: 10.16e-3,
        er: 1.0,
      },
    ] {
      assert_eq!(cutoff_hz(&bad, 1, 0), None, "{bad:?}");
      // `(0, 0)` 不是模式，与几何无关地返回 None。
      assert_eq!(cutoff_hz(&bad, 0, 0), None);
    }
    // 合法几何照常给有限值。
    assert!(cutoff_hz(&wr90(), 1, 0).is_some_and(f64::is_finite));
  }
}
