//! 传播预测与最佳工作频率：MUF、LUF、OWF 与波段选择。

/// 核心概念。
pub const MUF_CONCEPTS: &[(&str, &str)] = &[
  (
    "MUF 最高可用频率",
    "电离层能反射回地面的最高频率，高于它信号穿透电离层。",
  ),
  (
    "LUF 最低可用频率",
    "能被接收的最低频率，低于它信号被吸收或噪声淹没。",
  ),
  ("OWF 最佳工作频率", "约为 MUF 的 85%，最可靠的通信频率。"),
  (
    "VOACAP",
    "业界常用的传播预测软件，输入两端位置与时间预测可用波段。",
  ),
];

/// 波段选择建议。
pub const BAND_CHOICE: &[(&str, &str)] = &[
  ("白天 · 高太阳活动", "15m、10m 等高波段开放良好。"),
  ("夜间", "40m、80m 等低波段更适合远距离。"),
  ("日出日落", "各波段的黄金时间，传播变化快。"),
  ("太阳活动低年", "低波段（160m/80m/40m）更可靠，高波段受限。"),
];

/// 太阳射电流量 SFI（10.7 cm）与太阳黑子数 SSN 的经验换算（近似线性拟合，仅用于量级估算）：`SFI ≈ 66 + 0.75 × SSN`。
///
/// 二者长期统计高度相关。全站用它把「以 SSN 为输入」与「以 SFI 为输入」的两套 foF2
/// 公式统一到同一口径，避免不同页面算出互相矛盾的临界频率。
#[must_use]
pub fn sfi_from_ssn(ssn: f64) -> f64 {
  (66.0 + 0.75 * ssn).max(0.0)
}

/// [`sfi_from_ssn`] 的逆运算。
#[must_use]
pub fn ssn_from_sfi(sfi: f64) -> f64 {
  ((sfi - 66.0) / 0.75).max(0.0)
}

/// 单跳 F2 的 MUF 因子：`MUF = foF2 × 因子`。
///
/// 由正割定律 `MUF = foF2 × sec(i)`：**仰角越低（跳距越远）→ 入射角 i 越大 →
/// 因子越大**。垂直入射时因子为 1，3000 km 一跳约 3.3（常用近似 3.0），
/// 4000 km 约 3.4。
///
/// 需要按距离而非固定值计算时，请用 [`crate::voacap::muf_factor_for_hop`]。
pub const MUF_FACTOR_1HOP: f64 = 3.0;

/// 夜间（无日照）时 foF2 相对正午的残留比例。
///
/// 实测中纬度夜间 foF2 通常降到正午的三到四成，取 0.35：再低会连 40m 都判为
/// 不可用，与「夜间靠低波段」的实际经验不符。
pub const NIGHT_FOF2_RATIO: f64 = 0.35;

/// 把 UTC 时刻换算为当地地方时（0–24）：`local = utc + 经度 / 15`。
#[must_use]
pub fn local_hour(utc_hour: f64, lon_deg: f64) -> f64 {
  (utc_hour + lon_deg / 15.0).rem_euclid(24.0)
}

/// 昼夜因子（[`NIGHT_FOF2_RATIO`]–1.0）：电离程度随日照变化，地方时正午（约 12 时）最强、清晨日出（约 5 时）前后最弱。
///
/// 这是传播预测里最容易被忽略、但对结论影响最大的一项 —— 同一条路径在正午与
/// 凌晨的可用波段可以相差好几个。日出后上升快、日落后按正弦回落，其余时段维持
/// 夜间下限。
#[must_use]
pub fn diurnal_factor(local_hour: f64) -> f64 {
  let h = local_hour.rem_euclid(24.0);
  // 以 5 时为日出、19 时为日落，正弦半周覆盖白昼。
  let day = (std::f64::consts::PI * (h - 5.0) / 14.0).sin();
  if day <= 0.0 {
    NIGHT_FOF2_RATIO
  } else {
    NIGHT_FOF2_RATIO + (1.0 - NIGHT_FOF2_RATIO) * day
  }
}

/// 估算 F2 层临界频率 foF2（MHz）：基于太阳通量 SFI 的经验近似。
#[must_use]
pub fn estimate_fof2(sfi: f64) -> f64 {
  if sfi <= 0.0 {
    return 0.0;
  }
  0.5 + 0.9 * sfi.sqrt()
}

/// 估算单跳 F2 最高可用频率 MUF（MHz）。
#[must_use]
pub fn estimate_muf(sfi: f64) -> f64 {
  estimate_fof2(sfi) * MUF_FACTOR_1HOP
}

/// 最佳工作频率 OWF（MHz），约为 MUF 的 85%。
#[must_use]
pub fn estimate_owf(sfi: f64) -> f64 {
  estimate_muf(sfi) * 0.85
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn muf_data_populated() {
    assert!(!MUF_CONCEPTS.is_empty());
    assert!(!BAND_CHOICE.is_empty());
  }

  #[test]
  fn fof2_estimates_in_reasonable_range() {
    assert_eq!(estimate_fof2(0.0), 0.0);
    let f = estimate_fof2(150.0);
    assert!((f - 11.5).abs() < 2.0, "foF2 {f}");
    let m = estimate_muf(150.0);
    assert!(m > f, "muf {m} > foF2 {f}");
    let o = estimate_owf(150.0);
    assert!(o > 0.0 && o < m);
  }

  #[test]
  fn ssn_sfi_conversion_roundtrips() {
    assert_eq!(sfi_from_ssn(0.0), 66.0);
    assert!((sfi_from_ssn(100.0) - 141.0).abs() < 0.01);
    // 往返一致（负值会被截断到 0，故取正常范围验证）。
    for ssn in [0.0, 50.0, 100.0, 200.0] {
      assert!(
        (ssn_from_sfi(sfi_from_ssn(ssn)) - ssn).abs() < 0.01,
        "ssn {ssn}"
      );
    }
  }

  #[test]
  fn diurnal_factor_peaks_at_local_noon() {
    // 地方时 13 时最强、凌晨最弱，且始终落在 [NIGHT_FOF2_RATIO, 1.0]。
    let noon = diurnal_factor(13.0);
    let dawn = diurnal_factor(5.0);
    let night = diurnal_factor(2.0);
    assert!((noon - 1.0).abs() < 0.05, "noon {noon}");
    assert!((dawn - NIGHT_FOF2_RATIO).abs() < 0.01, "dawn {dawn}");
    assert!((night - NIGHT_FOF2_RATIO).abs() < 0.01, "night {night}");
    assert!(noon > diurnal_factor(9.0));
    for h in 0..24 {
      let f = diurnal_factor(f64::from(h));
      assert!((NIGHT_FOF2_RATIO..=1.0).contains(&f), "hour {h} factor {f}");
    }
  }

  #[test]
  fn local_hour_wraps_at_date_line() {
    // 东经 120°：UTC 20 时 → 次日 04 时
    assert!((local_hour(20.0, 120.0) - 4.0).abs() < 0.01);
    // 西经 120°：UTC 20 时 → 当天 12 时
    assert!((local_hour(20.0, -120.0) - 12.0).abs() < 0.01);
  }
}
