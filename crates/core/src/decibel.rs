//! 分贝与功率 / 电压比：dB ↔ 倍数换算。
//!
//! 功率比按 `dB = 10·lg(P₁/P₀)`，电压比按 `dB = 20·lg(V₁/V₀)` ——
//! 同一阻抗下 `V² ∝ P`，所以电压比的指数是功率比的一半。

/// dB → 功率倍数（`10^(dB/10)`）。
#[must_use]
pub fn db_to_power_ratio(db: f64) -> f64 {
  10f64.powf(db / 10.0)
}

/// 功率倍数 → dB（`10·lg(P₁/P₀)`）；倍数必须为正，否则返回 [`f64::NAN`]。
#[must_use]
pub fn power_ratio_to_db(ratio: f64) -> f64 {
  if ratio <= 0.0 {
    return f64::NAN;
  }
  10.0 * ratio.log10()
}

/// dB → 电压倍数（`10^(dB/20)`）。
#[must_use]
pub fn db_to_voltage_ratio(db: f64) -> f64 {
  10f64.powf(db / 20.0)
}

/// dBm → 毫瓦：`10^(dBm/10)`。dBm 以 1 mW 为 0 dB 基准。
#[must_use]
pub fn dbm_to_mw(dbm: f64) -> f64 {
  10f64.powf(dbm / 10.0)
}

/// 毫瓦 → dBm：`10·lg(P/mW)`；功率必须为正，否则返回 [`f64::NAN`]。
#[must_use]
pub fn mw_to_dbm(mw: f64) -> f64 {
  if mw <= 0.0 {
    return f64::NAN;
  }
  10.0 * mw.log10()
}

/// 对照表的档位（dB，由小到大）：衰减与增益各取几个记得住的整数刻度。
pub const COMMON_DB_STEPS: &[f64] = &[
  -30.0, -20.0, -10.0, -6.0, -3.0, -1.0, 0.0, 1.0, 2.0, 3.0, 4.0, 6.0, 10.0, 20.0, 30.0,
];

/// 对照表里的倍数排版：按量级取小数位，让 `0.001` / `0.10` / `2.00` / `10.0` / `100` 宽度协调。
#[must_use]
pub fn format_ratio(v: f64) -> String {
  if v.abs() < 0.01 {
    format!("{v:.3}")
  } else if v.abs() < 10.0 {
    format!("{v:.2}")
  } else if v.abs() < 100.0 {
    format!("{v:.1}")
  } else {
    format!("{v:.0}")
  }
}

/// dBm ↔ mW 常用对照的档位（dBm，由小到大）：整数台阶每 +10 dB 功率 ×10，
/// 再补 3 / 6 dB 的 ×2 / ×4 台阶，构成「1 / 2 / 4 / 10 / 20 / 50 / 100 …」功率阶梯。
pub const COMMON_DBM_STEPS: &[f64] = &[
  -30.0, -20.0, -10.0, 0.0, 3.0, 6.0, 10.0, 13.0, 17.0, 20.0, 23.0, 27.0, 30.0,
];

/// 毫瓦排版：按量级切到 μW / mW / W，数值部分复用 [`format_ratio`] 的取位规则。
#[must_use]
pub fn format_mw(mw: f64) -> String {
  if mw >= 1000.0 {
    format!("{} W", format_ratio(mw / 1000.0))
  } else if mw >= 1.0 {
    format!("{} mW", format_ratio(mw))
  } else {
    format!("{} μW", format_ratio(mw * 1000.0))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn anchors_match_the_handy_values() {
    // 3 dB ≈ 2 倍功率、6 dB ≈ 2 倍电压 —— 速记值允许 1% 偏差，其余是精确刻度。
    assert!((db_to_power_ratio(0.0) - 1.0).abs() < 1e-12);
    assert!((db_to_power_ratio(3.0) - 2.0).abs() < 0.02);
    assert!((db_to_power_ratio(10.0) - 10.0).abs() < 1e-9);
    assert!((db_to_power_ratio(20.0) - 100.0).abs() < 1e-9);
    assert!((db_to_power_ratio(30.0) - 1000.0).abs() < 1e-9);
    assert!((db_to_power_ratio(-3.0) - 0.5).abs() < 0.005);
    assert!((db_to_voltage_ratio(6.0) - 2.0).abs() < 0.01);
    assert!((db_to_voltage_ratio(-20.0) - 0.1).abs() < 1e-9);
  }

  #[test]
  fn db_and_ratio_are_inverse() {
    for db in [-30.0, -6.0, -0.5, 0.0, 1.5, 9.0, 30.0] {
      let back = power_ratio_to_db(db_to_power_ratio(db));
      assert!((back - db).abs() < 1e-9, "{db} dB 往返不一致：{back}");
    }
  }

  #[test]
  fn non_positive_ratio_returns_nan() {
    assert!(power_ratio_to_db(0.0).is_nan());
    assert!(power_ratio_to_db(-2.0).is_nan());
  }

  #[test]
  fn voltage_ratio_is_sqrt_of_power_ratio() {
    for db in COMMON_DB_STEPS {
      let p = db_to_power_ratio(*db);
      assert!((db_to_voltage_ratio(*db) - p.sqrt()).abs() < 1e-9);
    }
  }

  #[test]
  fn steps_are_sorted_and_centred() {
    assert!(COMMON_DB_STEPS.windows(2).all(|w| w[0] < w[1]));
    assert!(COMMON_DB_STEPS.contains(&0.0));
  }

  #[test]
  fn ratios_are_formatted_by_magnitude() {
    assert_eq!(format_ratio(db_to_power_ratio(0.0)), "1.00");
    assert_eq!(format_ratio(db_to_power_ratio(3.0)), "2.00");
    assert_eq!(format_ratio(db_to_power_ratio(6.0)), "3.98");
    assert_eq!(format_ratio(db_to_power_ratio(10.0)), "10.0");
    assert_eq!(format_ratio(db_to_power_ratio(20.0)), "100");
    assert_eq!(format_ratio(db_to_power_ratio(30.0)), "1000");
    assert_eq!(format_ratio(db_to_power_ratio(-1.0)), "0.79");
    assert_eq!(format_ratio(db_to_power_ratio(-3.0)), "0.50");
    assert_eq!(format_ratio(db_to_power_ratio(-10.0)), "0.10");
    assert_eq!(format_ratio(db_to_power_ratio(-20.0)), "0.01");
    assert_eq!(format_ratio(db_to_power_ratio(-30.0)), "0.001");
    assert_eq!(format_ratio(db_to_voltage_ratio(-30.0)), "0.03");
  }

  #[test]
  fn dbm_and_mw_anchor_at_zero_and_thirty() {
    assert!((dbm_to_mw(0.0) - 1.0).abs() < 1e-12);
    assert!((dbm_to_mw(30.0) - 1000.0).abs() < 1e-9);
    assert!(mw_to_dbm(1.0).abs() < 1e-12);
    assert!((mw_to_dbm(1000.0) - 30.0).abs() < 1e-9);
  }

  #[test]
  fn dbm_and_mw_are_inverse() {
    for dbm in [-30.0, -10.0, 0.0, 3.0, 13.0, 27.0, 30.0] {
      let back = mw_to_dbm(dbm_to_mw(dbm));
      assert!((back - dbm).abs() < 1e-9, "{dbm} dBm 往返不一致：{back}");
    }
  }

  #[test]
  fn non_positive_mw_returns_nan() {
    assert!(mw_to_dbm(0.0).is_nan());
    assert!(mw_to_dbm(-1.0).is_nan());
  }

  #[test]
  fn common_dbm_steps_are_sorted_and_formatted_with_units() {
    assert!(COMMON_DBM_STEPS.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(format_mw(dbm_to_mw(0.0)), "1.00 mW");
    assert_eq!(format_mw(dbm_to_mw(3.0)), "2.00 mW");
    assert_eq!(format_mw(dbm_to_mw(10.0)), "10.0 mW");
    assert_eq!(format_mw(dbm_to_mw(20.0)), "100 mW");
    assert_eq!(format_mw(dbm_to_mw(30.0)), "1.00 W");
    assert_eq!(format_mw(dbm_to_mw(-10.0)), "100 μW");
    assert_eq!(format_mw(dbm_to_mw(-30.0)), "1.00 μW");
  }
}
