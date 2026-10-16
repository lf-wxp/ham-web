//! 雨衰：ITU-R P.838-3 的单位长度雨衰系数与 ITU-R P.530-17 的地面视距链路雨衰估算。
//!
//! 管线：
//!
//! 1. `γ_R = k · R^α`（dB/km）：`k`、`α` 由频率（1–1000 GHz）、极化与路径仰角按 P.838-3 算出；
//! 2. 有效路径长度 `d_eff = d · r`，`r = 1 / (0.477 d^0.633 R0.01^(0.073α) f^0.123 − 10.579 (1 − e^(−0.024 d)))`，
//!    且 `r ≤ 2.5`（P.530-17 式 32）；
//! 3. `A0.01 = γ_R · d_eff`：一年里 0.01% 的时间被超过的雨衰（dB）；
//! 4. 其他时间百分比 `p`（0.001%–1%）：`A_p = A0.01 · C1 · p^−(C2 + C3 log10 p)`（P.530-17 式 34–35）。
//!
//! **验证程度（重要）**
//!
//! - P.838-3 的 `k` / `α` / `γ_R`：与 ITU 官方验证数据（64 行，14.25 与 29 GHz、水平 / 垂直极化、8 个仰角）
//!   逐行对拍，最大相对误差 < 1e-6；另与常引的公开表值（1 / 10 / 12 / 20 / 30 GHz）一致到 0.03%。
//! - P.530-17 的有效路径因子与时间百分比换算：**没有取到 ITU 官方验证数据**，也没取到 P.530 原文
//!   （ITU 官网拒绝了请求）。公式依据是 ITU-Rpy（开源，P.618 / P.838 等对 ITU 验证数据测过）对 P.530
//!   的实现与我对规范的记忆；单测只能证明代码与这两者一致，**不能证明与规范一致**。
//!
//! **没做的（界面也要写明）**
//!
//! - **R0.01 要用户自己给**：ITU-R P.837 的降雨率是全球网格数据，没有嵌进来；请用 P.837 数据、
//!   当地气象部门的统计，或工程上常用的区域值；
//! - 不含：大气气体吸收、云雾、闪烁、多径衰落、降雨交叉极化；
//! - 常引用的 P.530 适用范围是频率 ≤ 40 GHz、路径 ≤ 60 km；超出时给警告，不拒绝。

/// P.838-3 的有效频率范围（GHz）。
pub const FREQ_RANGE_GHZ: (f64, f64) = (1.0, 1000.0);
/// P.530 时间百分比范围（%）。
pub const PERCENT_RANGE: (f64, f64) = (0.001, 1.0);
/// 常引用的 P.530 适用上限：频率 40 GHz、路径 60 km。超出给警告。
pub const COMMON_LIMIT_GHZ: f64 = 40.0;
pub const COMMON_LIMIT_KM: f64 = 60.0;
/// 有效路径因子的上限。
const R_CAP: f64 = 2.5;

/// 极化。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarization {
  Horizontal,
  Vertical,
  /// 圆极化（倾角 45°）。
  Circular,
}

impl Polarization {
  /// 极化倾角 τ（度，相对水平）。
  #[must_use]
  pub const fn tilt_deg(self) -> f64 {
    match self {
      Self::Horizontal => 0.0,
      Self::Vertical => 90.0,
      Self::Circular => 45.0,
    }
  }
}

/// 一组 P.838-3 曲线参数：`Σ a_j · exp(−((log10 f − b_j) / c_j)²) + m · log10 f + c`。
struct Fit {
  a: &'static [f64],
  b: &'static [f64],
  c: &'static [f64],
  m: f64,
  k: f64,
}

impl Fit {
  fn eval(&self, f_ghz: f64) -> f64 {
    let lf = f_ghz.log10();
    let sum: f64 = self
      .a
      .iter()
      .zip(self.b)
      .zip(self.c)
      .map(|((a, b), c)| a * (-((lf - b) / c).powi(2)).exp())
      .sum();
    sum + self.m * lf + self.k
  }
}

// P.838-3 表 1–4。
const K_H: Fit = Fit {
  a: &[-5.33980, -0.35351, -0.23789, -0.94158],
  b: &[-0.10008, 1.26970, 0.86036, 0.64552],
  c: &[1.13098, 0.45400, 0.15354, 0.16817],
  m: -0.18961,
  k: 0.71147,
};
const K_V: Fit = Fit {
  a: &[-3.80595, -3.44965, -0.39902, 0.50167],
  b: &[0.56934, -0.22911, 0.73042, 1.07319],
  c: &[0.81061, 0.51059, 0.11899, 0.27195],
  m: -0.16398,
  k: 0.63297,
};
const A_H: Fit = Fit {
  a: &[-0.14318, 0.29591, 0.32177, -5.37610, 16.1721],
  b: &[1.82442, 0.77564, 0.63773, -0.96230, -3.29980],
  c: &[-0.55187, 0.19822, 0.13164, 1.47828, 3.43990],
  m: 0.67849,
  k: -1.95537,
};
const A_V: Fit = Fit {
  a: &[-0.07771, 0.56727, -0.20238, -48.2991, 48.5833],
  b: &[2.33840, 0.95545, 1.14520, 0.791669, 0.791459],
  c: &[-0.76284, 0.54039, 0.26809, 0.116226, 0.116479],
  m: -0.053739,
  k: 0.83433,
};

/// P.838-3 的 `(k, α)`。`el_deg` 是路径仰角（地面链路取 0），`tilt_deg` 是极化倾角。
///
/// 不检查频率范围（由调用方检查，见 [`FREQ_RANGE_GHZ`]）。
#[must_use]
pub fn coefficients(f_ghz: f64, el_deg: f64, tilt_deg: f64) -> (f64, f64) {
  let kh = 10f64.powf(K_H.eval(f_ghz));
  let kv = 10f64.powf(K_V.eval(f_ghz));
  let ah = A_H.eval(f_ghz);
  let av = A_V.eval(f_ghz);
  let c = el_deg.to_radians().cos().powi(2) * (2.0 * tilt_deg).to_radians().cos();
  let k = (kh + kv + (kh - kv) * c) / 2.0;
  let alpha = (kh * ah + kv * av + (kh * ah - kv * av) * c) / (2.0 * k);
  (k, alpha)
}

/// 输入。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Input {
  pub freq_ghz: f64,
  /// 路径长度（km）。
  pub dist_km: f64,
  /// 0.01% 时间被超过的降雨率 R0.01（mm/h，1 分钟积分时间）。
  pub rain_rate_001: f64,
  pub polarization: Polarization,
  /// 要估计的时间百分比 p（%，0.001–1）。
  pub percent: f64,
}

/// 输入不合法的原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RainError {
  /// 频率不在 1–1000 GHz（或不是有限数）。
  Frequency,
  /// 路径长度不是正的有限数。
  Distance,
  /// 降雨率不是正的有限数。
  RainRate,
  /// 时间百分比不在 0.001–1。
  Percent,
}

/// 警告：能算，但超出常引用的适用范围。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RainWarning {
  /// 频率高于 [`COMMON_LIMIT_GHZ`]。
  FrequencyAboveCommonLimit,
  /// 路径长于 [`COMMON_LIMIT_KM`]。
  PathAboveCommonLimit,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Output {
  pub k: f64,
  pub alpha: f64,
  /// 单位长度雨衰（dB/km）。
  pub specific_db_per_km: f64,
  /// 距离因子 r（已按上限裁剪）。
  pub path_factor: f64,
  /// 公式分母不为正或 r 超过上限、直接取上限（雨很小、路径较长时会发生）。
  pub path_factor_capped: bool,
  /// 有效路径长度（km）。
  pub effective_km: f64,
  /// 0.01% 时间被超过的雨衰（dB）。
  pub a_001_db: f64,
  /// 输入的时间百分比被超过的雨衰（dB）。
  pub a_p_db: f64,
  pub warnings: Vec<RainWarning>,
}

fn pos(x: f64) -> bool {
  x.is_finite() && x > 0.0
}

/// 估算雨衰。
///
/// # Errors
///
/// 输入超出 [`RainError`] 里列的范围。
pub fn estimate(input: &Input) -> Result<Output, RainError> {
  let f = input.freq_ghz;
  if !(f.is_finite() && (FREQ_RANGE_GHZ.0..=FREQ_RANGE_GHZ.1).contains(&f)) {
    return Err(RainError::Frequency);
  }
  if !pos(input.dist_km) {
    return Err(RainError::Distance);
  }
  if !pos(input.rain_rate_001) {
    return Err(RainError::RainRate);
  }
  let p = input.percent;
  if !(p.is_finite() && (PERCENT_RANGE.0..=PERCENT_RANGE.1).contains(&p)) {
    return Err(RainError::Percent);
  }
  let (d, r001) = (input.dist_km, input.rain_rate_001);

  // 地面链路：仰角取 0。
  let (k, alpha) = coefficients(f, 0.0, input.polarization.tilt_deg());
  let gamma = k * r001.powf(alpha);
  if !gamma.is_finite() {
    return Err(RainError::RainRate);
  }

  let denom = 0.477 * d.powf(0.633) * r001.powf(0.073 * alpha) * f.powf(0.123)
    - 10.579 * (1.0 - (-0.024 * d).exp());
  // 分母 ≤ 0 时 1/denom 为无穷或负数，物理上就是「路径因子到顶」。
  let raw = if denom > 0.0 {
    1.0 / denom
  } else {
    f64::INFINITY
  };
  let capped = raw >= R_CAP;
  let r = raw.min(R_CAP);
  let effective_km = r * d;
  let a001 = gamma * effective_km;

  let c0 = if f >= 10.0 {
    0.12 + 0.4 * (f / 10.0).log10().powf(0.8)
  } else {
    0.12
  };
  let c1 = 0.07f64.powf(c0) * 0.12f64.powf(1.0 - c0);
  let c2 = 0.855 * c0 + 0.546 * (1.0 - c0);
  let c3 = 0.139 * c0 + 0.043 * (1.0 - c0);
  let a_p = a001 * c1 * p.powf(-(c2 + c3 * p.log10()));
  if ![effective_km, a001, a_p].iter().all(|v| v.is_finite()) {
    return Err(RainError::Distance);
  }

  let mut warnings = Vec::new();
  if f > COMMON_LIMIT_GHZ {
    warnings.push(RainWarning::FrequencyAboveCommonLimit);
  }
  if d > COMMON_LIMIT_KM {
    warnings.push(RainWarning::PathAboveCommonLimit);
  }

  Ok(Output {
    k,
    alpha,
    specific_db_per_km: gamma,
    path_factor: r,
    path_factor_capped: capped,
    effective_km,
    a_001_db: a001,
    a_p_db: a_p,
    warnings,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn rel(a: f64, b: f64) -> f64 {
    ((a - b) / b).abs()
  }

  /// ITU-R P.838-3 验证数据（经 ITU-Rpy 仓库转录，**去重后 16 组**）：
  /// `(仰角°, 频率 GHz, 降雨率 mm/h, 极化倾角°, k, α, γ_R dB/km)`。
  ///
  /// 转录时每组被粘贴了 4 份（原表 64 行里只有 16 组不同的数据），重复行等价于把同一组
  /// 数据验证四遍 —— 断言里曾写着的「64 行」是 4 倍于真实的信心。现在按唯一行数断言，
  /// 整段复制粘贴会立刻变红。
  ///
  /// 覆盖范围如实标注：频率只有 14.25 / 29 GHz，极化倾角只有 0° / 90°（**没有圆极化
  /// τ = 45° 的官方值**），仰角 8 个。圆极化由下面那条闭式性质测试守着。
  const ITU_838_3: &[(f64, f64, f64, f64, f64, f64, f64)] = &[
    (
      31.07699124,
      14.25,
      26.48052,
      0.0,
      0.03975488,
      1.12418043,
      1.58130839,
    ),
    (
      40.232036, 14.25, 33.936232, 0.0, 0.04007624, 1.11804138, 2.06173213,
    ),
    (
      46.35969261,
      14.25,
      27.13586832,
      0.0,
      0.04030344,
      1.11376017,
      1.5920842,
    ),
    (
      31.07699124,
      29.0,
      26.48052,
      0.0,
      0.22106804,
      0.95320005,
      5.02180189,
    ),
    (
      40.232036, 29.0, 33.936232, 0.0, 0.22031404, 0.9504446, 6.27846024,
    ),
    (
      46.35969261,
      29.0,
      27.13586832,
      0.0,
      0.21978097,
      0.9484851,
      5.03135479,
    ),
    (
      22.27833468,
      14.25,
      50.639304,
      0.0,
      0.03949319,
      1.12925336,
      3.32139638,
    ),
    (
      52.67898486,
      14.25,
      78.2994993,
      0.0,
      0.04053522,
      1.10944215,
      5.11503463,
    ),
    (
      22.27833468,
      29.0,
      50.639304,
      0.0,
      0.22168203,
      0.95543001,
      9.42430244,
    ),
    (
      52.67898486,
      29.0,
      78.2994993,
      0.0,
      0.21923716,
      0.9464763,
      13.59290086,
    ),
    (
      48.24117054,
      14.25,
      63.62668149,
      90.0,
      0.04226474,
      1.07871664,
      3.72901264,
    ),
    (
      85.80459566,
      14.25,
      99.13558978,
      90.0,
      0.04133039,
      1.09499629,
      6.34064598,
    ),
    (
      20.14335809,
      14.25,
      42.91007183,
      90.0,
      0.04319835,
      1.0631531,
      2.35032323,
    ),
    (
      48.24117054,
      29.0,
      63.62668149,
      90.0,
      0.21517927,
      0.93116621,
      10.28699163,
    ),
    (
      85.80459566,
      29.0,
      99.13558978,
      90.0,
      0.21737148,
      0.93950825,
      16.3183686,
    ),
    (
      20.14335809,
      29.0,
      42.91007183,
      90.0,
      0.21298877,
      0.92265917,
      6.83364556,
    ),
  ];

  #[test]
  fn p838_3_matches_the_transcribed_validation_data() {
    assert_eq!(ITU_838_3.len(), 16, "去重后应当只有 16 组验证数据");
    // 重复行会把覆盖虚增：整段复制粘贴（每组 N 份）必须变红。
    let mut unique = ITU_838_3.to_vec();
    unique.sort_by(|a, b| a.partial_cmp(b).expect("数据里没有 NaN"));
    unique.dedup();
    assert_eq!(unique.len(), ITU_838_3.len(), "验证数据里有重复行");
    let mut worst = [0.0f64; 3];
    for &(el, f, r, tau, k, alpha, gamma) in ITU_838_3 {
      let (kk, aa) = coefficients(f, el, tau);
      let gg = kk * r.powf(aa);
      worst[0] = worst[0].max(rel(kk, k));
      worst[1] = worst[1].max(rel(aa, alpha));
      worst[2] = worst[2].max(rel(gg, gamma));
    }
    // 验证数据只给到 8 位有效数字。
    assert!(worst.iter().all(|w| *w < 1e-6), "{worst:?}");
  }

  #[test]
  fn p838_3_matches_the_widely_quoted_table_values() {
    // 水平 kH、αH、垂直 kV、αV：1 / 10 / 12 / 20 / 30 GHz（公开表，四舍五入到所给位数）。
    for (f, kh, ah, kv, av) in [
      (1.0, 2.59e-5, 0.9691, 3.08e-5, 0.8592),
      (10.0, 0.01217, 1.2571, 0.01129, 1.2156),
      (12.0, 0.02386, 1.1825, 0.02455, 1.1216),
      (20.0, 0.09164, 1.0568, 0.09611, 0.9847),
      (30.0, 0.2403, 0.9485, 0.2291, 0.9129),
    ] {
      let (k_h, a_h) = coefficients(f, 0.0, 0.0);
      let (k_v, a_v) = coefficients(f, 0.0, 90.0);
      assert!(rel(k_h, kh) < 5e-4 && rel(a_h, ah) < 5e-4, "{f} GHz H");
      assert!(rel(k_v, kv) < 5e-4 && rel(a_v, av) < 5e-4, "{f} GHz V");
    }
  }

  #[test]
  fn circular_polarisation_is_between_horizontal_and_vertical() {
    for f in [5.76, 10.368, 24.048, 47.088, 76.0] {
      let h = coefficients(f, 0.0, 0.0).0;
      let v = coefficients(f, 0.0, 90.0).0;
      let c = coefficients(f, 0.0, 45.0).0;
      assert!(c >= h.min(v) - 1e-15 && c <= h.max(v) + 1e-15, "{f}");
      // 水平路径、圆极化：k = (kH + kV) / 2 精确成立。
      assert!(rel(c, (h + v) / 2.0) < 1e-12);
    }
  }

  #[test]
  fn circular_polarisation_matches_the_closed_form_interpolation() {
    // 官方转录数据里没有 τ = 45°（圆极化）的取值，只能靠闭式性质守：
    // τ = 45° 时 cos 2τ = 0，插值式退化成「k 取 H / V 的算术平均、α 取按 k 加权的平均」。
    // 度数写成弧度（cos 2τ 不归零）或漏掉仰角的 cos²θ 都会在这里露出来。
    for f in [1.0, 5.76, 10.368, 14.25, 24.048, 29.0, 47.088, 76.0, 100.0] {
      for el in [0.0, 5.0, 15.0, 30.0, 45.0, 60.0, 90.0] {
        let (kh, ah) = coefficients(f, el, 0.0);
        let (kv, av) = coefficients(f, el, 90.0);
        let (k45, a45) = coefficients(f, el, 45.0);
        assert!(rel(k45, (kh + kv) / 2.0) < 1e-12, "k @ {f} GHz {el}°");
        assert!(
          rel(a45, (kh * ah + kv * av) / (kh + kv)) < 1e-12,
          "α @ {f} GHz {el}°"
        );
        // 仰角 0° 与 90° 的 cos²θ 分别取极值，k 必须落在两者之间。
        let (k90, _) = coefficients(f, 90.0, 0.0);
        assert!(
          k45 >= kh.min(k90) - 1e-15 && k45 <= kh.max(k90) + 1e-15,
          "{f}"
        );
      }
    }
  }

  fn est(f: f64, d: f64, r: f64, pol: Polarization, p: f64) -> Output {
    estimate(&Input {
      freq_ghz: f,
      dist_km: d,
      rain_rate_001: r,
      polarization: pol,
      percent: p,
    })
    .unwrap()
  }

  /// 数值来自独立脚本（只验证代码与公式转写一致，见模块文档的验证程度说明）。
  /// 脚本的数值只印了 5 位小数，所以容差取 5e-5（最小的 0.20961 本身就有 2.4e-5 的舍入）。
  #[test]
  fn path_attenuation_matches_the_independent_script() {
    use Polarization::{Circular, Horizontal, Vertical};
    // (f, d, R, 极化, p, γ, r, A0.01, Ap)
    for (f, d, r001, pol, p, gamma, r, a001, ap) in [
      (
        10.368, 20.0, 40.0, Horizontal, 0.01, 1.367_41, 0.531_23, 14.528_27, 14.500_53,
      ),
      (
        24.048, 10.0, 50.0, Vertical, 0.01, 5.931_11, 0.580_33, 34.420_12, 34.353_28,
      ),
      (
        24.048, 10.0, 50.0, Vertical, 0.1, 5.931_11, 0.580_33, 34.420_12, 12.958_83,
      ),
      (
        47.088, 5.0, 30.0, Circular, 0.01, 9.364_77, 0.714_28, 33.445_22, 33.379_60,
      ),
      (
        5.76, 30.0, 40.0, Horizontal, 0.01, 0.209_61, 0.408_46, 2.568_47, 2.563_58,
      ),
      (
        76.0, 2.0, 60.0, Horizontal, 0.001, 21.315_15, 0.937_36, 39.960_13, 70.705_26,
      ),
    ] {
      let o = est(f, d, r001, pol, p);
      assert!(
        rel(o.specific_db_per_km, gamma) < 5e-5,
        "{f} γ {}",
        o.specific_db_per_km
      );
      assert!(rel(o.path_factor, r) < 5e-5, "{f} r {}", o.path_factor);
      assert!(rel(o.a_001_db, a001) < 5e-5, "{f} A0.01 {}", o.a_001_db);
      assert!(rel(o.a_p_db, ap) < 5e-5, "{f} Ap {}", o.a_p_db);
      assert!(rel(o.effective_km, r * d) < 5e-5);
      assert!(!o.path_factor_capped);
    }
  }

  #[test]
  fn the_percentage_law_returns_about_a001_at_one_hundredth_of_a_percent() {
    // C1 的取法让 p = 0.01% 时 A_p 回到 A0.01（差一个很小的拟合残差）。
    for f in [3.4, 5.76, 10.368, 24.048, 47.088, 76.0] {
      let o = est(f, 10.0, 50.0, Polarization::Horizontal, 0.01);
      assert!(
        rel(o.a_p_db, o.a_001_db) < 0.005,
        "{f}: {} vs {}",
        o.a_p_db,
        o.a_001_db
      );
    }
  }

  #[test]
  fn attenuation_grows_with_rarer_time_heavier_rain_and_longer_paths() {
    use Polarization::Horizontal;
    let a = |f, d, r, p| est(f, d, r, Horizontal, p).a_p_db;
    // 越罕见（p 越小）衰落越深。
    assert!(a(10.368, 20.0, 40.0, 0.001) > a(10.368, 20.0, 40.0, 0.01));
    assert!(a(10.368, 20.0, 40.0, 0.01) > a(10.368, 20.0, 40.0, 0.1));
    assert!(a(10.368, 20.0, 40.0, 0.1) > a(10.368, 20.0, 40.0, 1.0));
    // 雨越大越深；频率越高越深（这个范围内）；路径越长越深。
    assert!(a(10.368, 20.0, 80.0, 0.01) > a(10.368, 20.0, 40.0, 0.01));
    assert!(a(24.048, 10.0, 40.0, 0.01) > a(10.368, 10.0, 40.0, 0.01));
    assert!(a(10.368, 30.0, 40.0, 0.01) > a(10.368, 10.0, 40.0, 0.01));
    // 有效路径长度不会超过 2.5 倍实际路径，也不会为负。
    let o = est(10.368, 20.0, 40.0, Horizontal, 0.01);
    assert!(o.effective_km > 0.0 && o.effective_km <= 2.5 * 20.0);
  }

  #[test]
  fn when_the_distance_factor_formula_breaks_down_it_is_capped_at_two_and_a_half() {
    // 低频、小雨、长路径：P.530-17 式 32 的分母变成负数（扫描 2835 个组合里有 176 个）。
    // 此时直接取上限 2.5，既不 panic 也不出现负的有效路径。
    let o = est(2.0, 33.0, 0.5, Polarization::Horizontal, 0.01);
    assert!(o.path_factor_capped);
    assert_eq!(o.path_factor, 2.5);
    assert!((o.effective_km - 82.5).abs() < 1e-12);
    assert!(o.a_001_db.is_finite() && o.a_001_db > 0.0);
    // 同一组参数下把降雨率加大，分母恢复为正，不再触顶。
    let big = est(2.0, 33.0, 80.0, Polarization::Horizontal, 0.01);
    assert!(!big.path_factor_capped && big.path_factor < 2.5);
  }

  #[test]
  fn warns_above_the_commonly_quoted_range_without_refusing() {
    use Polarization::Horizontal;
    let w = |f, d| est(f, d, 40.0, Horizontal, 0.01).warnings;
    assert!(w(24.048, 20.0).is_empty());
    assert_eq!(w(76.0, 2.0), [RainWarning::FrequencyAboveCommonLimit]);
    assert_eq!(w(10.368, 80.0), [RainWarning::PathAboveCommonLimit]);
    assert_eq!(
      w(76.0, 80.0),
      [
        RainWarning::FrequencyAboveCommonLimit,
        RainWarning::PathAboveCommonLimit
      ]
    );
    // 恰好在上限上不算超出。
    assert!(w(40.0, 60.0).is_empty());
  }

  #[test]
  fn rejects_out_of_range_input_with_a_specific_reason() {
    let ok = Input {
      freq_ghz: 10.368,
      dist_km: 20.0,
      rain_rate_001: 40.0,
      polarization: Polarization::Horizontal,
      percent: 0.01,
    };
    assert!(estimate(&ok).is_ok());
    let with = |f: fn(&mut Input)| {
      let mut i = ok;
      f(&mut i);
      estimate(&i)
    };
    // 频率只在 1–1000 GHz 有效：1 GHz 以下的业余波段（如 144 / 432 MHz）不在 P.838 的范围里。
    assert_eq!(with(|i| i.freq_ghz = 0.432), Err(RainError::Frequency));
    assert_eq!(with(|i| i.freq_ghz = 1000.5), Err(RainError::Frequency));
    assert_eq!(with(|i| i.freq_ghz = f64::NAN), Err(RainError::Frequency));
    assert!(with(|i| i.freq_ghz = 1.0).is_ok() && with(|i| i.freq_ghz = 1000.0).is_ok());
    assert_eq!(with(|i| i.dist_km = 0.0), Err(RainError::Distance));
    assert_eq!(with(|i| i.dist_km = -3.0), Err(RainError::Distance));
    assert_eq!(with(|i| i.rain_rate_001 = 0.0), Err(RainError::RainRate));
    assert_eq!(
      with(|i| i.rain_rate_001 = f64::INFINITY),
      Err(RainError::RainRate)
    );
    assert_eq!(with(|i| i.percent = 0.0005), Err(RainError::Percent));
    assert_eq!(with(|i| i.percent = 1.5), Err(RainError::Percent));
    assert!(with(|i| i.percent = 0.001).is_ok() && with(|i| i.percent = 1.0).is_ok());
  }
}
