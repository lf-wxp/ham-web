//! 单位换算纯计算内核：dBm ↔ 功率、dB ↔ 倍率、频率 ↔ 波长。
//!
//! 这是开放 API（`/api/v1/units/convert`）与前端计算器的共享内核；
//! 前端 `/tools` 页的换算工具目前各带一份本地算式，后续按这里收敛。

/// mW → dBm（`mw` 必须 > 0）。
#[must_use]
pub fn mw_to_dbm(mw: f64) -> Option<f64> {
  (mw > 0.0 && mw.is_finite()).then(|| 10.0 * mw.log10())
}

/// dBm → mW。极端输入（如 `dbm = 1e308`）会溢出为无穷，返回 `None`。
#[must_use]
pub fn dbm_to_mw(dbm: f64) -> Option<f64> {
  let v = 10f64.powf(dbm / 10.0);
  v.is_finite().then_some(v)
}

/// W → dBm。
#[must_use]
pub fn watts_to_dbm(w: f64) -> Option<f64> {
  mw_to_dbm(w * 1000.0)
}

/// dB → 倍率。极端输入溢出时返回 `None`。
#[must_use]
pub fn db_to_ratio(db: f64) -> Option<f64> {
  let v = 10f64.powf(db / 10.0);
  v.is_finite().then_some(v)
}

/// 倍率 → dB（`ratio` 必须 > 0）。
#[must_use]
pub fn ratio_to_db(ratio: f64) -> Option<f64> {
  (ratio > 0.0 && ratio.is_finite()).then(|| 10.0 * ratio.log10())
}

/// 频率（MHz）→ 波长（米）：`λ = 300 / f`（`mhz` 必须 > 0）。
///
/// 结果侧也查有限性：极小频率（如 `1e-308`）会让商溢出为 `inf`，而开放 API
/// 把 `inf` 序列化成 `null` —— 客户端区分不了「非法输入」与「结果缺失」。
#[must_use]
pub fn mhz_to_wavelength(mhz: f64) -> Option<f64> {
  let v = 300.0 / mhz;
  (mhz > 0.0 && mhz.is_finite() && v.is_finite()).then_some(v)
}

/// 波长（米）→ 频率（MHz）：`f = 300 / λ`（`metres` 必须 > 0）。
#[must_use]
pub fn wavelength_to_mhz(metres: f64) -> Option<f64> {
  let v = 300.0 / metres;
  (metres > 0.0 && metres.is_finite() && v.is_finite()).then_some(v)
}

/// 把 `value` 折算成 `unit` 并按 `p` 分之一取整（`p = 100` 即两位小数）。
///
/// 用来判「取整后是否顶到了下一档」：档位本身仍按原始值选（`998 nF` 不能因为两位小数
/// 四舍五入就变成 `1.00 µF`），只有当取整结果真的写出来是 `1000.00` 时才升档。
fn round_to(value: f64, unit: f64, p: f64) -> f64 {
  (value / unit * p).round() / p
}

/// 距离文案：≥ 1 km 用公里，≥ 1 m 用米，否则用厘米。
///
/// 这是界面排版，不是换算，但 `/tools` 的短截线匹配与 `/vna-reader` 的 TDR 都要显示同一种
/// 距离格式 —— 放内核里由单测钉住，免得两份实现慢慢漂开（单位符号本身与语言无关）。
///
/// TDR 的「最大不模糊距离」在细步进扫频下可以到公里级，所以必须有公里这一档；毫米级
/// （< 1 cm）保留一位小数，不能因为取整显示成 `0 cm`。非法输入给占位符，别把 `NaN cm`
/// 画到界面上。
#[must_use]
pub fn format_distance_m(metres: f64) -> String {
  if !metres.is_finite() || metres < 0.0 {
    return "—".to_owned();
  }
  // 取整后顶到上一档就整体升档：否则 `999.999 m` 会写成「1000.00 m」，数字跨档而单位没跟过去。
  let m = round_to(metres, 1.0, 100.0);
  if metres >= 1000.0 || m >= 1000.0 {
    format!("{:.2} km", metres / 1000.0)
  } else if metres >= 1.0 {
    format!("{m:.2} m")
  } else {
    let cm = round_to(metres, 0.01, 10.0);
    if cm >= 100.0 {
      format!("{:.2} m", metres)
    } else if cm >= 10.0 {
      format!("{cm:.0} cm")
    } else {
      format!("{cm:.1} cm")
    }
  }
}

/// 容值文案：µF / nF / pF。
///
/// 与 [`format_distance_m`] 同理：单位切换与取整是界面契约（e2e 逐字断言），放内核里由单测钉住。
#[must_use]
pub fn format_capacitance_f(farads: f64) -> String {
  if !farads.is_finite() || farads < 0.0 {
    return "—".to_owned();
  }
  // `998 nF` 保持 nF（两位小数还写得下），只有取整真的变成 `1000.00` 才升到 µF。
  let nf = round_to(farads, 1e-9, 100.0);
  if farads >= 1e-6 || nf >= 1000.0 {
    return format!("{:.2} µF", round_to(farads, 1e-6, 100.0));
  }
  let pf = round_to(farads, 1e-12, 100.0);
  if farads >= 1e-9 || pf >= 1000.0 {
    format!("{nf:.2} nF")
  } else {
    format!("{pf:.2} pF")
  }
}

/// 电感文案：mH / µH / nH。
#[must_use]
pub fn format_inductance_h(henries: f64) -> String {
  if !henries.is_finite() || henries < 0.0 {
    return "—".to_owned();
  }
  let uh = round_to(henries, 1e-6, 100.0);
  if henries >= 1e-3 || uh >= 1000.0 {
    return format!("{:.2} mH", round_to(henries, 1e-3, 100.0));
  }
  let nh = round_to(henries, 1e-9, 1.0);
  if henries >= 1e-6 || nh >= 1000.0 {
    format!("{uh:.2} µH")
  } else {
    format!("{nh:.0} nH")
  }
}

/// 电压文案：≥ 1 kV 用千伏，否则用伏。
#[must_use]
pub fn format_voltage_v(volts: f64) -> String {
  if !volts.is_finite() || volts < 0.0 {
    return "—".to_owned();
  }
  if volts >= 1000.0 {
    format!("{:.1} kV", volts / 1000.0)
  } else {
    format!("{:.0} V", volts)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
  }

  #[test]
  fn dbm_round_trips() {
    assert!(close(mw_to_dbm(1000.0).unwrap(), 30.0));
    assert!(close(mw_to_dbm(1.0).unwrap(), 0.0));
    assert!(close(watts_to_dbm(5.0).unwrap(), 36.989_700));
    assert!(close(dbm_to_mw(30.0).unwrap(), 1000.0));
    assert!(close(dbm_to_mw(-30.0).unwrap(), 0.001));
    assert!(dbm_to_mw(1e308).is_none(), "溢出为 inf 应拒绝");
  }

  #[test]
  fn db_round_trips() {
    assert!(close(db_to_ratio(3.0).unwrap(), 1.995_262));
    assert!(close(db_to_ratio(10.0).unwrap(), 10.0));
    assert!(db_to_ratio(1e308).is_none(), "溢出为 inf 应拒绝");
    assert!(close(ratio_to_db(2.0).unwrap(), 3.010_300));
    assert!(close(ratio_to_db(10.0).unwrap(), 10.0));
  }

  #[test]
  fn wavelength_round_trips() {
    assert!(close(mhz_to_wavelength(146.0).unwrap(), 2.054_795));
    assert!(close(mhz_to_wavelength(7.05).unwrap(), 42.553_191));
    assert!(close(wavelength_to_mhz(2.0).unwrap(), 150.0));
    assert!(close(
      wavelength_to_mhz(mhz_to_wavelength(14.2).unwrap()).unwrap(),
      14.2
    ));
  }

  #[test]
  fn distance_format_switches_unit_at_one_metre() {
    assert_eq!(format_distance_m(2.117), "2.12 m");
    assert_eq!(format_distance_m(1.0), "1.00 m");
    assert_eq!(format_distance_m(0.114), "11 cm");
    assert_eq!(format_distance_m(0.0), "0.0 cm");
  }

  #[test]
  fn distance_format_has_a_kilometre_branch_and_sub_centimetre_decimals() {
    // TDR 的最大不模糊距离可以到公里级；毫米级不能被取整成「0 cm」。
    assert_eq!(format_distance_m(28_060.0), "28.06 km");
    assert_eq!(format_distance_m(1000.0), "1.00 km");
    assert_eq!(format_distance_m(0.004), "0.4 cm");
    // 非法输入给占位符，不把 NaN / 负数画到界面上。
    assert_eq!(format_distance_m(f64::NAN), "—");
    assert_eq!(format_distance_m(-1.0), "—");
  }

  #[test]
  fn format_boundaries_promote_instead_of_showing_a_thousand() {
    // 取整把数字顶到下一档时，单位必须跟着升档：不能出现「1000.00 m / 1000.00 nF」。
    assert_eq!(format_distance_m(999.999), "1.00 km");
    assert_eq!(format_distance_m(0.999_999), "1.00 m");
    assert_eq!(format_capacitance_f(999.999e-9), "1.00 µF");
    assert_eq!(format_capacitance_f(0.999_999e-9), "1.00 nF");
    assert_eq!(format_inductance_h(0.999_999e-3), "1.00 mH");
    assert_eq!(format_inductance_h(0.999_999e-6), "1.00 µH");
    // 差一点没顶到档位时仍留在原档。
    assert_eq!(format_distance_m(999.9), "999.90 m");
    assert_eq!(format_capacitance_f(998e-9), "998.00 nF");
  }

  #[test]
  fn component_value_formats_switch_units() {
    // 容值：µF / nF / pF 三档。
    assert_eq!(format_capacitance_f(1e-6), "1.00 µF");
    assert_eq!(format_capacitance_f(100e-9), "100.00 nF");
    assert_eq!(format_capacitance_f(47e-12), "47.00 pF");
    // 电感：mH / µH / nH。
    assert_eq!(format_inductance_h(2.2e-3), "2.20 mH");
    assert_eq!(format_inductance_h(2.94e-6), "2.94 µH");
    assert_eq!(format_inductance_h(470e-9), "470 nH");
    // 电压：V / kV。
    assert_eq!(format_voltage_v(12.0), "12 V");
    assert_eq!(format_voltage_v(5961.0), "6.0 kV");
    // 非法输入一律占位符。
    for f in [f64::NAN, f64::INFINITY, -1.0] {
      assert_eq!(format_capacitance_f(f), "—");
      assert_eq!(format_inductance_h(f), "—");
      assert_eq!(format_voltage_v(f), "—");
    }
  }

  #[test]
  fn invalid_inputs_are_rejected() {
    assert!(mw_to_dbm(0.0).is_none());
    assert!(mw_to_dbm(-1.0).is_none());
    assert!(mw_to_dbm(f64::INFINITY).is_none());
    assert!(ratio_to_db(0.0).is_none());
    assert!(mhz_to_wavelength(0.0).is_none());
    assert!(wavelength_to_mhz(-2.0).is_none());
    // 极小输入会让商溢出为 inf：结果侧同样拒绝，别让 API 序列化成 null。
    assert!(mhz_to_wavelength(1e-308).is_none());
    assert!(wavelength_to_mhz(1e-308).is_none());
  }
}
