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
