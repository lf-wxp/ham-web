//! 卫星线性转发器：通带映射（上下行联动）与写**上行**频率前的硬护栏。
//!
//! # 线性转发器怎么联动
//!
//! 转发器把上行通带里的整段信号搬到下行通带，调谐点移动一份，对应的另一端就移动一份：
//!
//! ```text
//! 反相    上行 + 下行 = 常数   （上行向高，下行向低；AO-7 B、FO-29、RS-44、AO-73）
//! 非反相  上行 − 下行 = 常数   （同向；AO-7 A）
//! ```
//!
//! 这里的频率都是**卫星坐标系**（卫星自己收到 / 发出的频率）。地面要调的收发频率再各自做一次
//! 多普勒修正（[`crate::sat_doppler`]）。判据：地面发 `tx` → 卫星收到上行 → 转发成下行 →
//! 地面收到 `rx`，闭环后 `rx` 恰好等于对下行调谐点做的接收修正（单测逐个径向速度验证，误差为 0）。
//!
//! # 数据出处
//!
//! 通带取自 SatNOGS DB（`/api/transmitters/`，`type=Transponder` 且 `status=active`，取回日期 2026-10-16），
//! 逐条与本站 [`crate::satellites::SATELLITES`] 里人工写的频率互相印证（AO-7 B、FO-29 两处一致）。
//! 只收**上下行通带宽度相等**的条目（单测断言）：宽度不等就没有一一对应的映射。
//!
//! # 写上行的护栏
//!
//! 写发射频率是这个项目里风险最高的一件事，所以护栏放在这里（可单测），界面只负责展示原因，不能绕过：
//!
//! 1. 必须落在业余波段表内（[`crate::bands::AMATEUR_BAND_EDGES`]）；
//! 2. 必须落在**卫星业余业务**可用频段内（[`crate::bands::AMATEUR_SATELLITE_RANGES_MHZ`]）；
//! 3. 与「该转发器 / 信道的标称上行」的偏离不能超过多普勒的物理上限（±[`MAX_DOPPLER_PPM`]），
//!    线性转发器再加通带半宽——填错一位数、选错卫星都会被这条挡住。
//!
//! 任何一条不满足都**不写**并给出原因。**护栏不能保证电台实际往哪里发射**：那取决于电台上的分频设置，
//! 本项目不碰分频、不碰 PTT。

use crate::bands::{AMATEUR_BAND_EDGES, AMATEUR_SATELLITE_RANGES_MHZ};
use crate::sat_doppler::{C_KM_S, rx_frequency_hz, tx_frequency_hz};

/// 多普勒造成的相对频偏上限（ppm）。低轨径向速度不超过约 7.9 km/s（26 ppm）；留到 12 km/s（40 ppm）
/// 给高椭圆轨道，比这再大的相对速度不属于业余卫星多普勒的合理范围。
pub const MAX_DOPPLER_PPM: f64 = 40.0;

/// 一个线性转发器的通带（Hz，卫星坐标系）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transponder {
  /// 如 `AO-7 · 模式 B`。
  pub name: &'static str,
  pub norad: u64,
  pub uplink_lo: u64,
  pub uplink_hi: u64,
  pub downlink_lo: u64,
  pub downlink_hi: u64,
  /// 反相（上行向高，下行向低）。
  pub inverting: bool,
}

/// 预置的线性转发器：SatNOGS DB 里 `Transponder` 且 `active` 的条目（2026-10-16）。
///
/// 上行用 `type=Transponder` 的 `uplink_low/high`，下行用 `downlink_low/high`，`invert` 照抄。
pub const TRANSPONDERS: &[Transponder] = &[
  Transponder {
    name: "AO-7 · A",
    norad: 7_530,
    uplink_lo: 145_850_000,
    uplink_hi: 145_950_000,
    downlink_lo: 29_400_000,
    downlink_hi: 29_500_000,
    inverting: false,
  },
  Transponder {
    name: "AO-7 · B",
    norad: 7_530,
    uplink_lo: 432_125_000,
    uplink_hi: 432_175_000,
    downlink_lo: 145_925_000,
    downlink_hi: 145_975_000,
    inverting: true,
  },
  Transponder {
    name: "FO-29",
    norad: 24_278,
    uplink_lo: 145_900_000,
    uplink_hi: 146_000_000,
    downlink_lo: 435_800_000,
    downlink_hi: 435_900_000,
    inverting: true,
  },
  Transponder {
    name: "RS-44",
    norad: 44_909,
    uplink_lo: 145_935_000,
    uplink_hi: 145_995_000,
    downlink_lo: 435_610_000,
    downlink_hi: 435_670_000,
    inverting: true,
  },
  Transponder {
    name: "AO-73",
    norad: 39_444,
    uplink_lo: 435_130_000,
    uplink_hi: 435_150_000,
    downlink_lo: 145_950_000,
    downlink_hi: 145_970_000,
    inverting: true,
  },
];

/// 自定义通带不合法的原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassbandError {
  /// 某个端点不是正数。
  NotPositive,
  /// 下限不小于上限。
  Inverted,
  /// 上下行通带宽度不等：没有一一对应的映射。
  WidthMismatch,
  /// 宽度小于 [`MIN_PASSBAND_HZ`] 或大于 [`MAX_PASSBAND_HZ`]：多半是填错了单位或位数。
  WidthOutOfRange,
}

/// 自定义通带宽度下限（Hz）。预置里最窄的是 AO-73 的 20 kHz。
pub const MIN_PASSBAND_HZ: u64 = 1_000;
/// 自定义通带宽度上限（Hz）。预置里最宽的是 AO-7 A 与 FO-29 的 100 kHz；线性转发器的通带不会到兆赫。
pub const MAX_PASSBAND_HZ: u64 = 1_000_000;

impl Transponder {
  /// 由自定义输入构造（卫星新发射、预置表里没有的转发器）：端点为正、下限小于上限、上下行宽度相等且在合理范围。
  ///
  /// `name` 留空、`norad` 为 0：`name` 是 `&'static str`，运行时的名字放不进去，界面用词条显示「自定义」。
  /// **自定义通带不绕过上行护栏**：[`guard_uplink`] 对它同样检查业余波段、卫星业务段与离标称的距离
  /// （标称取通带中心、余量取半宽，与预置一致）。
  ///
  /// # Errors
  ///
  /// 见 [`PassbandError`]。
  pub fn custom(
    uplink: (u64, u64),
    downlink: (u64, u64),
    inverting: bool,
  ) -> Result<Self, PassbandError> {
    if [uplink.0, uplink.1, downlink.0, downlink.1].contains(&0) {
      return Err(PassbandError::NotPositive);
    }
    if uplink.0 >= uplink.1 || downlink.0 >= downlink.1 {
      return Err(PassbandError::Inverted);
    }
    let (uw, dw) = (uplink.1 - uplink.0, downlink.1 - downlink.0);
    if uw != dw {
      return Err(PassbandError::WidthMismatch);
    }
    if !(MIN_PASSBAND_HZ..=MAX_PASSBAND_HZ).contains(&uw) {
      return Err(PassbandError::WidthOutOfRange);
    }
    Ok(Self {
      name: "",
      norad: 0,
      uplink_lo: uplink.0,
      uplink_hi: uplink.1,
      downlink_lo: downlink.0,
      downlink_hi: downlink.1,
      inverting,
    })
  }

  /// 通带宽度（Hz）。
  #[must_use]
  pub const fn width(&self) -> u64 {
    self.downlink_hi - self.downlink_lo
  }

  /// 下行通带中心（Hz）。
  #[must_use]
  pub const fn downlink_center(&self) -> u64 {
    self.downlink_lo + self.width() / 2
  }

  /// 把调谐点夹到通带内。
  #[must_use]
  pub fn clamp_downlink(&self, dl_hz: u64) -> u64 {
    dl_hz.clamp(self.downlink_lo, self.downlink_hi)
  }

  /// 下行调谐点（卫星坐标系）对应的上行频率；调谐点在通带外为 `None`（**不夹**：越界就是越界）。
  #[must_use]
  pub fn uplink_for(&self, dl_hz: u64) -> Option<u64> {
    if !(self.downlink_lo..=self.downlink_hi).contains(&dl_hz) {
      return None;
    }
    Some(if self.inverting {
      self.uplink_lo + (self.downlink_hi - dl_hz)
    } else {
      self.uplink_lo + (dl_hz - self.downlink_lo)
    })
  }

  /// 把调谐点移动 `delta_hz`（正 = 下行向高），夹在通带内。
  #[must_use]
  pub fn step_downlink(&self, dl_hz: u64, delta_hz: i64) -> u64 {
    let moved = i128::from(dl_hz) + i128::from(delta_hz);
    let clamped = moved.clamp(i128::from(self.downlink_lo), i128::from(self.downlink_hi));
    u64::try_from(clamped).unwrap_or(self.downlink_lo)
  }
}

/// 地面该调的一对频率（Hz，已做多普勒修正）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pair {
  /// 卫星坐标系的下行调谐点。
  pub downlink_sat: u64,
  /// 卫星坐标系的上行（由通带映射）。
  pub uplink_sat: u64,
  /// 地面接收频率。
  pub rx_hz: f64,
  /// 地面发射频率。
  pub tx_hz: f64,
}

/// 由下行调谐点和径向速度算出地面该调的一对频率；调谐点在通带外或速度不合法为 `None`。
#[must_use]
pub fn pair(t: &Transponder, downlink_sat: u64, range_rate_km_s: f64) -> Option<Pair> {
  let uplink_sat = t.uplink_for(downlink_sat)?;
  Some(Pair {
    downlink_sat,
    uplink_sat,
    rx_hz: rx_frequency_hz(downlink_sat as f64, range_rate_km_s)?,
    tx_hz: tx_frequency_hz(uplink_sat as f64, range_rate_km_s)?,
  })
}

/// 拒绝写上行的原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxRefusal {
  /// 频率不是正的有限数。
  NotValid,
  /// 不在业余波段表内。
  OutsideAmateurBand,
  /// 在业余波段内，但不在卫星业余业务可用频段内。
  OutsideSatelliteService,
  /// 与标称上行偏离超过多普勒的物理上限。
  FarFromNominal,
}

fn in_amateur_band(mhz: f64) -> bool {
  AMATEUR_BAND_EDGES
    .iter()
    .any(|&(_, lo, hi)| (lo..hi).contains(&mhz))
}

fn in_satellite_service(mhz: f64) -> bool {
  AMATEUR_SATELLITE_RANGES_MHZ
    .iter()
    .any(|&(lo, hi)| (lo..hi).contains(&mhz))
}

/// 写上行前的硬护栏：`tx_hz` 是准备写进电台的发射频率，`nominal_hz` 是该信道 / 转发器的标称上行（FM 星的固定上行，
/// 或线性转发器的上行通带），`slack_hz` 是标称之外允许的余量（FM 为 0，线性为通带半宽）。
///
/// 返回 `Ok(tx_hz)` 才能写。顺序有意义：先挡「根本不是业余频率」，再挡「不是卫星业务」，最后挡「离标称太远」。
///
/// # Errors
///
/// 见 [`TxRefusal`]。
pub fn guard_uplink(tx_hz: u64, nominal_hz: u64, slack_hz: u64) -> Result<u64, TxRefusal> {
  if tx_hz == 0 || nominal_hz == 0 {
    return Err(TxRefusal::NotValid);
  }
  let mhz = tx_hz as f64 / 1e6;
  if !in_amateur_band(mhz) {
    return Err(TxRefusal::OutsideAmateurBand);
  }
  if !in_satellite_service(mhz) {
    return Err(TxRefusal::OutsideSatelliteService);
  }
  // 物理上限：多普勒最多让频率偏离 nominal·(ṙ/c)。业余频段下 nominal 远小于 2^53，f64 表示精确、
  // 不会溢出；向上取整给足余量。
  let doppler = (nominal_hz as f64 * MAX_DOPPLER_PPM / 1e6).ceil() as u64;
  let allowed = doppler.saturating_add(slack_hz);
  if tx_hz.abs_diff(nominal_hz) > allowed {
    return Err(TxRefusal::FarFromNominal);
  }
  Ok(tx_hz)
}

/// 多普勒造成的偏离上限折合的径向速度（km/s）：[`MAX_DOPPLER_PPM`] 对应多少，给文档 / 测试对照用。
#[must_use]
pub fn max_range_rate_km_s() -> f64 {
  MAX_DOPPLER_PPM / 1e6 * C_KM_S
}

#[cfg(test)]
mod tests {
  use super::*;

  fn by_name(n: &str) -> &'static Transponder {
    TRANSPONDERS.iter().find(|t| t.name == n).unwrap()
  }

  /// 独立脚本（Python，不复用本模块任何代码）算出的映射：`(名字, 下行, 上行)`。
  const MAPPINGS: &[(&str, u64, u64)] = &[
    ("AO-7 · B", 145_925_000, 432_175_000),
    ("AO-7 · B", 145_950_000, 432_150_000),
    ("AO-7 · B", 145_975_000, 432_125_000),
    ("AO-7 · A", 29_400_000, 145_850_000),
    ("AO-7 · A", 29_450_000, 145_900_000),
    ("AO-7 · A", 29_500_000, 145_950_000),
    ("FO-29", 435_800_000, 146_000_000),
    ("FO-29", 435_850_000, 145_950_000),
    ("FO-29", 435_900_000, 145_900_000),
    ("RS-44", 435_610_000, 145_995_000),
    ("RS-44", 435_640_000, 145_965_000),
    ("RS-44", 435_670_000, 145_935_000),
    ("AO-73", 145_950_000, 435_150_000),
    ("AO-73", 145_960_000, 435_140_000),
    ("AO-73", 145_970_000, 435_130_000),
  ];

  #[test]
  fn the_mapping_matches_the_independent_script() {
    for &(name, dl, ul) in MAPPINGS {
      assert_eq!(by_name(name).uplink_for(dl), Some(ul), "{name} {dl}");
    }
  }

  #[test]
  fn every_preset_has_equal_passband_widths_so_the_mapping_is_one_to_one() {
    for t in TRANSPONDERS {
      assert_eq!(
        t.uplink_hi - t.uplink_lo,
        t.downlink_hi - t.downlink_lo,
        "{}",
        t.name
      );
      assert!(t.uplink_lo < t.uplink_hi && t.downlink_lo < t.downlink_hi);
    }
  }

  #[test]
  fn inverting_keeps_the_sum_constant_and_non_inverting_the_difference() {
    for t in TRANSPONDERS {
      let mut seen = std::collections::BTreeSet::new();
      let step = (t.width() / 4).max(1);
      let mut dl = t.downlink_lo;
      while dl <= t.downlink_hi {
        let ul = t.uplink_for(dl).unwrap();
        seen.insert(if t.inverting {
          i128::from(ul) + i128::from(dl)
        } else {
          i128::from(ul) - i128::from(dl)
        });
        dl += step;
      }
      assert_eq!(seen.len(), 1, "{}: {seen:?}", t.name);
    }
    // 常数值与独立脚本一致。
    assert_eq!(
      i128::from(by_name("FO-29").uplink_for(435_850_000).unwrap()) + 435_850_000,
      581_800_000
    );
    assert_eq!(
      i128::from(by_name("AO-7 · A").uplink_for(29_450_000).unwrap()) - 29_450_000,
      116_450_000
    );
  }

  #[test]
  fn band_edges_map_to_opposite_ends_when_inverting_and_matching_ends_when_not() {
    let fo = by_name("FO-29");
    assert_eq!(fo.uplink_for(fo.downlink_lo), Some(fo.uplink_hi));
    assert_eq!(fo.uplink_for(fo.downlink_hi), Some(fo.uplink_lo));
    let a = by_name("AO-7 · A");
    assert_eq!(a.uplink_for(a.downlink_lo), Some(a.uplink_lo));
    assert_eq!(a.uplink_for(a.downlink_hi), Some(a.uplink_hi));
  }

  #[test]
  fn tuning_outside_the_passband_is_refused_not_clamped() {
    let fo = by_name("FO-29");
    assert_eq!(fo.uplink_for(fo.downlink_lo - 1), None);
    assert_eq!(fo.uplink_for(fo.downlink_hi + 1), None);
    assert_eq!(fo.uplink_for(0), None);
    // 夹紧是另一个显式动作。
    assert_eq!(fo.clamp_downlink(fo.downlink_lo - 5_000), fo.downlink_lo);
    assert_eq!(fo.clamp_downlink(fo.downlink_hi + 5_000), fo.downlink_hi);
  }

  #[test]
  fn stepping_stays_inside_the_passband_and_never_underflows() {
    let fo = by_name("FO-29");
    let c = fo.downlink_center();
    assert_eq!(c, 435_850_000);
    assert_eq!(fo.step_downlink(c, 1_000), 435_851_000);
    assert_eq!(fo.step_downlink(c, -1_000), 435_849_000);
    assert_eq!(fo.step_downlink(fo.downlink_hi, 1_000_000), fo.downlink_hi);
    assert_eq!(fo.step_downlink(fo.downlink_lo, -1_000_000), fo.downlink_lo);
    // 极端步长：i64 的两端也不 panic、不回绕。
    assert_eq!(fo.step_downlink(c, i64::MAX), fo.downlink_hi);
    assert_eq!(fo.step_downlink(c, i64::MIN), fo.downlink_lo);
    // 起点在通带外（比如手填）：一步也会被拉回通带内。
    assert_eq!(fo.step_downlink(1, 10), fo.downlink_lo);
  }

  /// 判据：地面发 tx → 卫星收到上行 → 转发 → 地面收到 rx，闭环后 rx 等于对下行调谐点的接收修正。
  /// 转发用独立的「常数」公式（反相：K − 上行；非反相：K + 上行），不调用 `uplink_for`。
  #[test]
  fn the_round_trip_through_the_transponder_lands_on_the_rx_correction() {
    for t in TRANSPONDERS {
      // 反相：下行 = (ul_lo + dl_hi) − 上行；非反相：下行 = dl_lo + (上行 − ul_lo)。
      let relay = |ul_heard: f64| {
        if t.inverting {
          (t.uplink_lo + t.downlink_hi) as f64 - ul_heard
        } else {
          t.downlink_lo as f64 + (ul_heard - t.uplink_lo as f64)
        }
      };
      for rr in [-7.9, -3.0, 0.0, 4.2, 7.9] {
        for dl in [t.downlink_lo, t.downlink_center(), t.downlink_hi] {
          let p = pair(t, dl, rr).unwrap();
          let heard_by_sat = p.tx_hz * (1.0 - rr / C_KM_S);
          let sent_by_sat = relay(heard_by_sat);
          let heard_on_ground = sent_by_sat / (1.0 + rr / C_KM_S);
          assert!(
            (heard_on_ground - p.rx_hz).abs() < 1e-3,
            "{} dl={dl} rr={rr}: {heard_on_ground} vs {}",
            t.name,
            p.rx_hz
          );
          // 卫星听到的上行，就是映射给出的上行（多普勒修正正好抵消）。
          assert!((heard_by_sat - p.uplink_sat as f64).abs() < 1e-3);
        }
      }
    }
  }

  #[test]
  fn fo29_pair_matches_the_independent_script() {
    let t = by_name("FO-29");
    // (径向速度, 地面应收, 地面应发)，来自独立脚本。
    for (rr, rx, tx) in [
      (-7.0, 435_860_177.1, 145_946_592.2),
      (-3.5, 435_855_088.5, 145_948_296.1),
      (0.0, 435_850_000.0, 145_950_000.0),
      (3.5, 435_844_911.6, 145_951_703.9),
      (7.0, 435_839_823.4, 145_953_407.9),
    ] {
      let p = pair(t, 435_850_000, rr).unwrap();
      assert!((p.rx_hz - rx).abs() < 0.06, "rx {rr}: {}", p.rx_hz);
      assert!((p.tx_hz - tx).abs() < 0.06, "tx {rr}: {}", p.tx_hz);
      assert_eq!((p.downlink_sat, p.uplink_sat), (435_850_000, 145_950_000));
    }
  }

  #[test]
  fn approaching_raises_rx_and_lowers_the_tx_you_need() {
    let p = pair(by_name("FO-29"), 435_850_000, -7.0).unwrap();
    assert!(p.rx_hz > 435_850_000.0, "靠近时接收频率升高");
    assert!(
      p.tx_hz < 145_950_000.0,
      "靠近时要发得更低，卫星才收到标称上行"
    );
  }

  #[test]
  fn pair_refuses_a_tuning_point_outside_the_passband_or_a_bad_speed() {
    let t = by_name("FO-29");
    assert_eq!(pair(t, 435_000_000, 0.0), None);
    assert_eq!(pair(t, 435_850_000, f64::NAN), None);
    assert_eq!(pair(t, 435_850_000, C_KM_S), None);
  }

  #[test]
  fn custom_passbands_are_validated() {
    let ok =
      Transponder::custom((145_900_000, 146_000_000), (435_800_000, 435_900_000), true).unwrap();
    // 与预置的 FO-29 行为完全一致。
    let fo = by_name("FO-29");
    for dl in [435_800_000, 435_850_000, 435_900_000] {
      assert_eq!(ok.uplink_for(dl), fo.uplink_for(dl));
    }
    assert_eq!((ok.name, ok.norad), ("", 0));
    assert_eq!(
      Transponder::custom((0, 1), (2, 3), true),
      Err(PassbandError::NotPositive)
    );
    assert_eq!(
      Transponder::custom((146_000_000, 145_900_000), (435_800_000, 435_900_000), true),
      Err(PassbandError::Inverted)
    );
    assert_eq!(
      Transponder::custom((145_900_000, 145_900_000), (435_800_000, 435_900_000), true),
      Err(PassbandError::Inverted)
    );
    assert_eq!(
      Transponder::custom((145_900_000, 146_000_000), (435_800_000, 435_850_000), true),
      Err(PassbandError::WidthMismatch)
    );
  }

  #[test]
  fn custom_passband_width_must_be_plausible() {
    // 恰在界限上可以，差 1 Hz 不行。
    let mk = |w: u64| {
      Transponder::custom(
        (145_900_000, 145_900_000 + w),
        (435_800_000, 435_800_000 + w),
        true,
      )
    };
    assert!(mk(MIN_PASSBAND_HZ).is_ok());
    assert!(mk(MAX_PASSBAND_HZ).is_ok());
    assert_eq!(mk(MIN_PASSBAND_HZ - 1), Err(PassbandError::WidthOutOfRange));
    assert_eq!(mk(MAX_PASSBAND_HZ + 1), Err(PassbandError::WidthOutOfRange));
    // 典型笔误：把 kHz 当成 Hz 填（宽度变成 100 Hz）；把 MHz 当成 Hz 填（宽度变成 100 MHz）。
    assert_eq!(mk(100), Err(PassbandError::WidthOutOfRange));
    assert_eq!(mk(100_000_000), Err(PassbandError::WidthOutOfRange));
    // 所有预置都在范围内（范围的依据）。
    for t in TRANSPONDERS {
      assert!(
        (MIN_PASSBAND_HZ..=MAX_PASSBAND_HZ).contains(&t.width()),
        "{}",
        t.name
      );
    }
  }

  #[test]
  fn a_custom_passband_does_not_bypass_the_uplink_guard() {
    // 把 FO-29 的上行通带整体挪到 146.2–146.3 MHz（2 m 业余波段内，但在卫星业务段 144–146 之外）：护栏照样拒写。
    let t =
      Transponder::custom((146_200_000, 146_300_000), (435_800_000, 435_900_000), true).unwrap();
    let nominal = t.uplink_lo + t.width() / 2;
    let tx = pair(&t, t.downlink_center(), 0.0).unwrap().tx_hz.round() as u64;
    assert_eq!(
      guard_uplink(tx, nominal, t.width() / 2),
      Err(TxRefusal::OutsideSatelliteService)
    );
    // 挪到 14 MHz 以下根本不是业余波段（电视频段）：同样被拒。
    let t =
      Transponder::custom((88_000_000, 88_100_000), (435_800_000, 435_900_000), true).unwrap();
    let nominal = t.uplink_lo + t.width() / 2;
    let tx = pair(&t, t.downlink_center(), 0.0).unwrap().tx_hz.round() as u64;
    assert_eq!(
      guard_uplink(tx, nominal, t.width() / 2),
      Err(TxRefusal::OutsideAmateurBand)
    );
    // 而一个合法的自定义通带（落在 2 m 卫星段内）可以写。
    let t =
      Transponder::custom((145_100_000, 145_200_000), (435_300_000, 435_400_000), true).unwrap();
    let nominal = t.uplink_lo + t.width() / 2;
    let tx = pair(&t, t.downlink_center(), 0.0).unwrap().tx_hz.round() as u64;
    assert_eq!(guard_uplink(tx, nominal, t.width() / 2), Ok(tx));
  }

  #[test]
  fn presets_agree_with_the_hand_written_satellite_list() {
    // 本站 `satellites.rs` 里人工写的 AO-7 B 与 FO-29，与 SatNOGS 的数据逐端点一致。
    let ao7b = by_name("AO-7 · B");
    assert_eq!((ao7b.uplink_lo, ao7b.uplink_hi), (432_125_000, 432_175_000));
    assert_eq!(
      (ao7b.downlink_lo, ao7b.downlink_hi),
      (145_925_000, 145_975_000)
    );
    let fo = by_name("FO-29");
    assert_eq!((fo.uplink_lo, fo.uplink_hi), (145_900_000, 146_000_000));
    assert_eq!((fo.downlink_lo, fo.downlink_hi), (435_800_000, 435_900_000));
    let list: String = crate::satellites::SATELLITES
      .iter()
      .map(|s| format!("{}|{}|{}", s.name, s.uplink, s.downlink))
      .collect();
    assert!(list.contains("432.125–432.175"));
    assert!(list.contains("145.900–146.000"));
    assert!(list.contains("435.800–435.900"));
  }

  #[test]
  fn presets_are_unique_and_sorted_by_nothing_surprising() {
    let mut names: Vec<_> = TRANSPONDERS.iter().map(|t| t.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), TRANSPONDERS.len());
    assert!(TRANSPONDERS.iter().all(|t| t.norad > 0));
  }

  // ───── 护栏 ─────

  /// 上行整段落在卫星业余业务段内的预置（AO-7 A、RS-44、AO-73），在多普勒最坏情形下通带里**每个**调谐点都必须能写。
  /// 独立脚本逐点扫过（每颗 3 个径向速度 × 201 个调谐点）：这三颗被拒的点数都是 0。
  #[test]
  fn the_guard_lets_every_tuning_point_through_when_the_uplink_is_inside_the_service() {
    for name in ["AO-7 · A", "RS-44", "AO-73"] {
      let t = by_name(name);
      let nominal = t.uplink_lo + t.width() / 2;
      let slack = t.width() / 2;
      let step = (t.width() / 200).max(1);
      let mut dl = t.downlink_lo;
      while dl <= t.downlink_hi {
        for rr in [-7.9, 0.0, 7.9] {
          let tx = pair(t, dl, rr).unwrap().tx_hz.round() as u64;
          assert_eq!(
            guard_uplink(tx, nominal, slack),
            Ok(tx),
            "{name} dl={dl} rr={rr}"
          );
        }
        dl += step;
      }
    }
  }

  /// FO-29 的上行通带是 145.900–146.000 MHz，上沿恰好是 2 m 卫星段 `[144, 146)` 的开区间上界。
  /// 零多普勒时只有 146.000 MHz 这一点被拒；靠近卫星需要发得更低、被拒的区间缩小，远离时发得更高，被拒区间变宽：
  /// 独立脚本给出最坏情形（ṙ = +7.9 km/s）下通带里下行最低的约 3.85 kHz 对应的上行落在 146 MHz 之上。
  /// 护栏的约定（半开区间）是全站统一的，这里照实陈述，不迁就。
  #[test]
  fn fo29s_top_edge_is_refused_because_146_mhz_is_the_open_upper_bound_of_the_segment() {
    let t = by_name("FO-29");
    let nominal = t.uplink_lo + t.width() / 2;
    let slack = t.width() / 2;
    let tx = |dl, rr| pair(t, dl, rr).unwrap().tx_hz.round() as u64;
    // 零多普勒：下行低端 → 上行 146.000 MHz，被拒；往里 10 Hz 就能写了。
    assert_eq!(
      guard_uplink(tx(435_800_000, 0.0), nominal, slack),
      Err(TxRefusal::OutsideSatelliteService)
    );
    assert_eq!(
      guard_uplink(tx(435_800_010, 0.0), nominal, slack),
      Ok(145_999_990)
    );
    // 最坏远离：下行低端起约 3.85 kHz 内被拒，3.85 kHz 之外能写（独立脚本：第一个可写点 435 803 850）。
    assert_eq!(
      guard_uplink(tx(435_800_000, 7.9), nominal, slack),
      Err(TxRefusal::OutsideSatelliteService)
    );
    assert_eq!(
      guard_uplink(tx(435_803_840, 7.9), nominal, slack),
      Err(TxRefusal::OutsideSatelliteService)
    );
    assert!(guard_uplink(tx(435_803_850, 7.9), nominal, slack).is_ok());
    // 靠近：发得更低，整个通带都在界内。
    assert!(guard_uplink(tx(435_800_000, -7.9), nominal, slack).is_ok());
    // 通带中心从来不受影响。
    for rr in [-7.9, 0.0, 7.9] {
      let c = tx(t.downlink_center(), rr);
      assert_eq!(guard_uplink(c, nominal, slack), Ok(c));
    }
  }

  /// AO-7 模式 B 的上行（432.125–432.175 MHz）在 430–435 MHz：是 70 cm 业余波段，但 ITU 脚注 5.282 给卫星业余业务的
  /// 70 cm 段只有 435–438 MHz。护栏按项目的法规数据保守地拒写，并说清原因；是否允许在该频段发射卫星上行是主管部门的事，
  /// 不由本站替用户判断。
  #[test]
  fn ao7_mode_b_uplink_is_refused_because_it_is_outside_the_satellite_service_segment() {
    let t = by_name("AO-7 · B");
    let nominal = t.uplink_lo + t.width() / 2;
    for dl in [t.downlink_lo, t.downlink_center(), t.downlink_hi] {
      let tx = pair(t, dl, 0.0).unwrap().tx_hz.round() as u64;
      assert_eq!(
        guard_uplink(tx, nominal, t.width() / 2),
        Err(TxRefusal::OutsideSatelliteService),
        "{tx}"
      );
    }
    // 同一颗星的模式 A 上行（145.85–145.95 MHz）在 144–146 MHz 卫星段内，照常通过。
    let a = by_name("AO-7 · A");
    let tx = pair(a, a.downlink_center(), 0.0).unwrap().tx_hz.round() as u64;
    assert_eq!(
      guard_uplink(tx, a.uplink_lo + a.width() / 2, a.width() / 2),
      Ok(tx)
    );
  }

  #[test]
  fn the_guard_lets_the_fm_presets_through_and_refuses_a_wrong_digit() {
    for p in crate::sat_doppler::PRESETS {
      for rr in [-7.9, 0.0, 7.9] {
        let tx = tx_frequency_hz(p.uplink_hz as f64, rr).unwrap().round() as u64;
        assert_eq!(guard_uplink(tx, p.uplink_hz, 0), Ok(tx), "{} {rr}", p.name);
      }
    }
    // ISS 上行 145.990：填成 145.099 / 146.990 / 14.599 都被挡。
    assert_eq!(
      guard_uplink(145_099_000, 145_990_000, 0),
      Err(TxRefusal::FarFromNominal)
    );
    assert_eq!(
      guard_uplink(146_990_000, 145_990_000, 0),
      Err(TxRefusal::OutsideSatelliteService)
    );
    assert_eq!(
      guard_uplink(14_599_000, 145_990_000, 0),
      Err(TxRefusal::OutsideAmateurBand)
    );
  }

  #[test]
  fn the_guard_checks_band_then_service_then_distance_in_that_order() {
    // 不在任何业余波段：电视 / 航空频段。
    assert_eq!(
      guard_uplink(121_500_000, 121_500_000, 0),
      Err(TxRefusal::OutsideAmateurBand)
    );
    assert_eq!(
      guard_uplink(162_550_000, 162_550_000, 0),
      Err(TxRefusal::OutsideAmateurBand)
    );
    // 在 2 m 业余波段里（146–148），但不在卫星业余业务段（144–146）。
    assert_eq!(
      guard_uplink(146_520_000, 146_520_000, 0),
      Err(TxRefusal::OutsideSatelliteService)
    );
    // 70 cm 里 430–435 是业余波段、不是卫星业务；435–438 才是。
    assert_eq!(
      guard_uplink(433_500_000, 433_500_000, 0),
      Err(TxRefusal::OutsideSatelliteService)
    );
    assert_eq!(guard_uplink(436_000_000, 436_000_000, 0), Ok(436_000_000));
    // 438–440 又是业余波段、不是卫星业务。
    assert_eq!(
      guard_uplink(439_000_000, 439_000_000, 0),
      Err(TxRefusal::OutsideSatelliteService)
    );
    // 1.2 GHz 的卫星业务段在项目的业余波段表之外（表只到 70 cm）：被挡，这是有意的保守。
    assert_eq!(
      guard_uplink(1_265_000_000, 1_265_000_000, 0),
      Err(TxRefusal::OutsideAmateurBand)
    );
  }

  #[test]
  fn the_distance_limit_is_the_doppler_ceiling_plus_slack() {
    // 145.990 MHz 的 40 ppm = 5 839.6 Hz，向上取整 5 840。
    let nominal = 145_990_000;
    assert_eq!(
      guard_uplink(nominal + 5_840, nominal, 0),
      Ok(nominal + 5_840)
    );
    assert_eq!(
      guard_uplink(nominal - 5_840, nominal, 0),
      Ok(nominal - 5_840)
    );
    assert_eq!(
      guard_uplink(nominal + 5_841, nominal, 0),
      Err(TxRefusal::FarFromNominal)
    );
    assert_eq!(
      guard_uplink(nominal - 5_841, nominal, 0),
      Err(TxRefusal::FarFromNominal)
    );
    // 余量加在上面。
    assert_eq!(
      guard_uplink(nominal + 5_841, nominal, 1),
      Ok(nominal + 5_841)
    );
    // 对应的径向速度上限是 12 km/s。
    assert!((max_range_rate_km_s() - 11.99).abs() < 0.01);
  }

  #[test]
  fn the_guard_rejects_zero_without_panicking() {
    assert_eq!(guard_uplink(0, 145_990_000, 0), Err(TxRefusal::NotValid));
    assert_eq!(guard_uplink(145_990_000, 0, 0), Err(TxRefusal::NotValid));
    assert_eq!(
      guard_uplink(u64::MAX, u64::MAX, u64::MAX),
      Err(TxRefusal::OutsideAmateurBand)
    );
  }
}
