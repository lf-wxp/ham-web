//! 点对点 HF 传播预测：基于 foF2 经验模型与太阳黑子数的简化实现。
//!
//! 参考 VOACAP / ITU-R P.533 的思路，给出两点间各业余波段的可用性估计。
//! 为教学与快速判断用途，采用简化公式；结果仅供参考，精确规划请使用专业预测软件。
//!
//! foF2 的**基础公式统一复用 [`crate::muf::estimate_fof2`]**（以 SFI 为输入，经
//! [`crate::muf::sfi_from_ssn`] 换算），本模块只在其上叠加季节与昼夜修正。这样
//! 「MUF 速查」与「点对点预测」两处不会给出互相矛盾的临界频率。

use serde::Serialize;

use crate::grid::{distance_bearing, lat_lon_from_grid};
use crate::muf::{NIGHT_FOF2_RATIO, diurnal_factor, local_hour, sfi_from_ssn};

/// 最佳工作频率（FOT / OWF）与 MUF 之比。
const OWF_RATIO: f64 = 0.85;

/// 最低可用频率（LUF）下限（MHz）：夜间 D 层消失，此时受大气噪声与人为噪声限制。
const LUF_NIGHT_MHZ: f64 = 1.8;

/// 最低可用频率上限（MHz）：正午 D 层吸收最强时（吸收大致正比于 1/f²）。
const LUF_DAY_MHZ: f64 = 6.8;

/// 业余 HF 波段及其参考频率（MHz）。
pub const HF_BANDS: &[(&str, f64)] = &[
  ("160m", 1.8),
  ("80m", 3.6),
  ("60m", 5.3),
  ("40m", 7.1),
  ("30m", 10.1),
  ("20m", 14.1),
  ("17m", 18.1),
  ("15m", 21.1),
  ("12m", 24.9),
  ("10m", 28.4),
];

/// 单个波段的预测结果。
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct BandResult {
  /// 波段名，如 `20m`。
  pub band: &'static str,
  /// 参考频率（MHz）。
  pub freq_mhz: f64,
  /// 是否可用（参考频率不超过 MUF）。
  pub usable: bool,
  /// 相对可靠度（0–1）：越接近最佳工作频率（OWF）越高。
  pub reliability: f64,
}

/// 一次点对点预测。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Prediction {
  /// 两点大圆距离（km）。
  pub distance_km: f64,
  /// 初始方位角（度，正北为 0）。
  pub bearing_deg: f64,
  /// 估算的 foF2（MHz）。
  pub fo_f2: f64,
  /// 估算的 MUF（MHz）。
  pub muf: f64,
  /// 各波段结果。
  pub bands: Vec<BandResult>,
  /// 预测所用 UTC 时刻（0–24）；`None` 表示未指定时刻（按路径日照最佳情况估算）。
  pub hour_utc: Option<f64>,
  /// 路径中点经度（度），地方时由此换算。
  pub mid_lon: f64,
  /// 路径中点的地方时（0–24）；`None` 表示未指定时刻，此时 [`Self::diurnal`] 恒为 1，
  /// 没有「真实地方时」可言 —— 用 `Option` 而不是填一个假值，调用方才不会拿它去展示。
  pub local_hour: Option<f64>,
  /// 实际施加的昼夜衰减因子（[`crate::muf::diurnal_factor`] 的值域，
  /// 下限约为 [`crate::muf::NIGHT_FOF2_RATIO`]）；未指定时刻时为 1。
  pub diurnal: f64,
}

/// 季节项：春秋分略强于冬夏。
///
/// 春秋分是两个峰，故周期取 **6 个月**；拟合峰值落在 4 月与 10 月，与节气春秋分（3/9 月）相差约 1 个月。
/// 用 12 个月周期的话，10 月会落到谷底（0.92），把「秋分强」写成「秋分最弱」。
fn seasonal_factor(month: u32) -> f64 {
  1.0 + 0.08 * (2.0 * std::f64::consts::PI * (month as f64 - 4.0) / 6.0).cos()
}

/// F2 层等效反射高度（km）。
const REFLECTION_HEIGHT_KM: f64 = 300.0;

/// 地球平均半径（km）。
const EARTH_RADIUS_KM: f64 = 6371.0;

/// F2 层单跳最大地面距离（km）；更长的路径必须分成多跳。
const MAX_HOP_KM: f64 = 4000.0;

/// 单跳 MUF 因子：正割定律 `MUF = foF2 × sec(i)`。
///
/// `hop_km` 为**单跳**的地面距离。仰角越低（单跳跨距越远）→ 入射角 i 越大 →
/// 因子**越大**：垂直入射（hop→0）时因子为 1，3000 km 约 3.3，4000 km 约 3.4。
///
/// 注意这与「跳数越多因子越小」的直觉不同。长路径的 MUF 实测偏低另有原因：
/// 路径会跨越电离层条件最差的一段（晨昏线、夜侧或低纬），瓶颈在最差的那一段，
/// 而不是仰角本身。
#[must_use]
pub fn muf_factor_for_hop(hop_km: f64) -> f64 {
  if hop_km <= 0.0 {
    return 1.0;
  }
  let hop = hop_km.min(MAX_HOP_KM);
  // 半跳对应的地心角。
  let beta = (hop / 2.0) / EARTH_RADIUS_KM;
  let (sin_b, cos_b) = beta.sin_cos();
  let denom = EARTH_RADIUS_KM + REFLECTION_HEIGHT_KM - EARTH_RADIUS_KM * cos_b;
  let tan_i = EARTH_RADIUS_KM * sin_b / denom;
  // sec(i) = √(1 + tan²i)
  (1.0 + tan_i * tan_i).sqrt()
}

/// 路径 MUF 因子：先按 [`MAX_HOP_KM`] 分跳，再按单跳跨距取正割因子。
fn muf_factor(distance_km: f64) -> f64 {
  let hops = (distance_km / MAX_HOP_KM).ceil().max(1.0);
  muf_factor_for_hop(distance_km / hops)
}

/// 估算 foF2（MHz）：太阳黑子数越高电离层越强；春秋分略强于冬夏。
///
/// 基础公式复用 [`crate::muf::estimate_fof2`]，此处只叠加季节项。
#[must_use]
pub fn estimate_fof2(ssn: f64, month: u32) -> f64 {
  crate::muf::estimate_fof2(sfi_from_ssn(ssn)) * seasonal_factor(month)
}

/// 估算指定时刻、指定经度处的 foF2（MHz）：在 [`estimate_fof2`] 上叠加昼夜衰减。
#[must_use]
pub fn estimate_fof2_at(ssn: f64, month: u32, utc_hour: f64, lon_deg: f64) -> f64 {
  estimate_fof2(ssn, month) * diurnal_factor(local_hour(utc_hour, lon_deg))
}

/// 大圆路径中点（向量平均，自动正确处理跨日期变更线）。
///
/// 近似对跖的两点（相距接近 20000 km）向量和趋近零，`atan2(0, 0)` 会返回 0 而把中点
/// 扔到几内亚湾（0°, 0°），地方时与昼夜因子随之全错。这种路径在业余通信里虽罕见但
/// 确实存在，因此退化时退回发射端坐标 —— 至少不会凭空造出一个错误的中点。
fn midpoint(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> (f64, f64) {
  let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
  let (l1, l2) = (lon1.to_radians(), lon2.to_radians());
  let x = p1.cos() * l1.cos() + p2.cos() * l2.cos();
  let y = p1.cos() * l1.sin() + p2.cos() * l2.sin();
  let z = p1.sin() + p2.sin();
  if x.hypot(y) < 1e-9 && z.abs() < 1e-9 {
    return (lat1, lon1);
  }
  (z.atan2(x.hypot(y)).to_degrees(), y.atan2(x).to_degrees())
}

/// 点对点传播预测（不指定时刻，按路径日照最佳情况估算）。网格码非法时返回 `None`。
///
/// 需要判断「今晚 20m 能不能通」请用 [`predict_at`] 传入 UTC 时刻。
#[must_use]
pub fn predict(tx_grid: &str, rx_grid: &str, month: u32, ssn: f64) -> Option<Prediction> {
  predict_at(tx_grid, rx_grid, month, ssn, None)
}

/// 带上 UTC 时刻的点对点传播预测。
///
/// 时刻会换算成**路径中点**的地方时后施加昼夜衰减 —— 决定 MUF 的是路径中途
/// 电离层的日照情况，而不是发射端当地时间。网格码非法时返回 `None`。
#[must_use]
pub fn predict_at(
  tx_grid: &str,
  rx_grid: &str,
  month: u32,
  ssn: f64,
  utc_hour: Option<f64>,
) -> Option<Prediction> {
  let (tx_lat, tx_lon) = lat_lon_from_grid(tx_grid)?;
  let (rx_lat, rx_lon) = lat_lon_from_grid(rx_grid)?;
  let (distance_km, bearing_deg) = distance_bearing(tx_lat, tx_lon, rx_lat, rx_lon);

  let (_mid_lat, mid_lon) = midpoint(tx_lat, tx_lon, rx_lat, rx_lon);
  // 未指定时刻时不换算地方时：那时算出来的数只是「假设 UTC 12 点」的产物，
  // 拿去展示会被误读成真实时刻。
  let path_local_hour = utc_hour.map(|h| local_hour(h, mid_lon));
  let diurnal = path_local_hour.map_or(1.0, diurnal_factor);

  let fo_f2 = estimate_fof2(ssn, month) * diurnal;
  // 先按 F2 单跳上限分跳，再按单跳跨距取正割因子。
  let muf = fo_f2 * muf_factor(distance_km);
  let owf = muf * OWF_RATIO;
  // 最低可用频率：由 D 层吸收（∝1/f²，白天最强）与噪声共同决定，与 MUF 不是
  // 固定比例关系。这里用「夜间底噪 → 白天吸收」随日照插值作粗略估计。
  let daytime = ((diurnal - NIGHT_FOF2_RATIO) / (1.0 - NIGHT_FOF2_RATIO)).clamp(0.0, 1.0);
  let luf = (LUF_NIGHT_MHZ + (LUF_DAY_MHZ - LUF_NIGHT_MHZ) * daytime).min(muf * 0.9);

  let bands = HF_BANDS
    .iter()
    .map(|&(band, freq_mhz)| {
      let usable = freq_mhz <= muf && freq_mhz >= luf;
      // 以 OWF 为峰的连续曲线：低于 LUF 为 0，LUF→OWF 上升，OWF→MUF 下降。
      let reliability = if freq_mhz <= luf || freq_mhz >= muf {
        0.0
      } else if freq_mhz <= owf {
        (freq_mhz - luf) / (owf - luf)
      } else {
        (muf - freq_mhz) / (muf - owf)
      };
      BandResult {
        band,
        freq_mhz,
        usable,
        reliability,
      }
    })
    .collect();

  Some(Prediction {
    distance_km,
    bearing_deg,
    fo_f2,
    muf,
    bands,
    hour_utc: utc_hour,
    mid_lon,
    local_hour: path_local_hour,
    diurnal,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn fof2_scales_with_ssn() {
    assert!(estimate_fof2(200.0, 10) > estimate_fof2(0.0, 10));
  }

  #[test]
  fn fof2_reuses_muf_baseline() {
    // 统一口径：voacap 的 foF2 必须等于 muf 模块基础公式 × 季节项，
    // 否则「MUF 速查」与「点对点预测」会给出互相矛盾的临界频率。
    let expected = crate::muf::estimate_fof2(crate::muf::sfi_from_ssn(100.0)) * seasonal_factor(10);
    assert!((estimate_fof2(100.0, 10) - expected).abs() < 1e-9);
    // 数量级合理：中等太阳活动下正午 foF2 约 10 MHz。
    let f = estimate_fof2(100.0, 10);
    assert!((8.0..=13.0).contains(&f), "foF2 {f}");
  }

  #[test]
  fn predict_between_grids() {
    // 北京（OM89ew 附近）→ 伦敦（IO91wm 附近）
    let p = predict("OM89", "IO91", 10, 100.0).expect("valid grids");
    assert!(p.distance_km > 7000.0, "dist {}", p.distance_km);
    assert!(
      (p.bearing_deg - 320.0).abs() < 40.0,
      "bearing {}",
      p.bearing_deg
    );
    // 未指定时刻时按路径日照最佳情况估算（全路径受照）。按正割定律，8150 km 分
    // 3 跳、每跳约 2717 km，单跳因子约 3.2，foF2 约 10MHz → MUF 约 32MHz。
    assert!(
      (29.7..=45.0).contains(&p.muf),
      "best-case MUF {} 应高于 10m 波段上沿 29.7MHz",
      p.muf
    );
    let forty = p.bands.iter().find(|b| b.band == "40m").unwrap();
    let twenty = p.bands.iter().find(|b| b.band == "20m").unwrap();
    let ten = p.bands.iter().find(|b| b.band == "10m").unwrap();
    assert!(forty.usable);
    assert!(twenty.usable);
    // 全路径受照的最佳情况下 10m 进入可用范围（旧模型按「跳数越多因子越小」
    // 算出 MUF≈27MHz 会误判 10m 不可用）。
    assert!(ten.usable);
    // 可靠度以 OWF（0.85×MUF≈27MHz）为峰，10m 比 20m 更接近峰值。
    assert!(ten.reliability > twenty.reliability);
    assert_eq!(p.hour_utc, None);
  }

  #[test]
  fn muf_factor_follows_secant_law() {
    // 正割定律：跳距越远（仰角越低）因子越大，垂直入射时为 1。
    assert!((muf_factor_for_hop(0.0) - 1.0).abs() < 1e-9);
    let f500 = muf_factor_for_hop(500.0);
    let f1000 = muf_factor_for_hop(1000.0);
    let f3000 = muf_factor_for_hop(3000.0);
    assert!(f500 < f1000 && f1000 < f3000, "{f500} {f1000} {f3000}");
    // 3000 km 一跳的常用值约 3.0–3.4。
    assert!((3.0..=3.4).contains(&f3000), "M(3000) = {f3000}");
    // 短路径不能被当成 3.0 —— 这是历史上把 500 km 路径算成 MUF≈30MHz 的根源。
    assert!(f500 < 1.5, "500km 因子应约 1.3，实际 {f500}");
    // 分段：8150 km 按 4000 km 上限分 3 跳，每跳更短 → 因子略低于 3000 km 一跳。
    let f_long = muf_factor(8150.0);
    assert!(f_long < f3000, "{f_long}");
    assert!(f_long > 3.0, "{f_long}");
  }

  #[test]
  fn seasonal_factor_peaks_at_equinoxes() {
    // 春分（4 月）与秋分（10 月）都应是峰，冬夏为谷。
    let apr = seasonal_factor(4);
    let oct = seasonal_factor(10);
    let jan = seasonal_factor(1);
    let jul = seasonal_factor(7);
    assert!((apr - 1.08).abs() < 1e-9, "4 月 {apr}");
    assert!((oct - 1.08).abs() < 1e-9, "10 月 {oct}");
    assert!(jan < apr && jul < apr, "冬夏应低于春秋分：{jan} {jul}");
  }

  #[test]
  fn reliability_peaks_at_owf_and_vanishes_below_luf() {
    let p = predict_at("OM89", "IO91", 10, 100.0, Some(12.0)).expect("valid grids");
    let at = |b: &str| p.bands.iter().find(|x| x.band == b).unwrap().reliability;
    // 曲线必须连续且以 OWF 为峰：不能出现「OWF 左侧 1.0、右侧 0.5」的断崖，
    // 也不能让低于 LUF 的波段仍被判为「较可靠」。
    for b in p.bands.iter() {
      assert!(b.reliability >= 0.0 && b.reliability <= 1.0, "{:?}", b);
    }
    let owf = p.muf * OWF_RATIO;
    // 20m（14.1MHz）低于 OWF，10m（28.4MHz）高于 OWF 且接近 MUF → 可靠度应更低。
    assert!(owf > 14.1, "OWF {owf}");
    assert!(at("20m") > 0.0);
    assert!(at("10m") < at("20m"), "接近 MUF 的波段可靠度应下降");
  }

  #[test]
  fn night_drops_high_bands_and_keeps_low_ones() {
    let day = predict_at("OM89", "IO91", 10, 100.0, Some(12.0)).expect("valid grids");
    let night = predict_at("OM89", "IO91", 10, 100.0, Some(0.0)).expect("valid grids");

    // 同一条路径，凌晨的 MUF 应显著低于白天。
    assert!(
      night.muf < day.muf * 0.6,
      "day {} night {}",
      day.muf,
      night.muf
    );
    assert!(night.diurnal < day.diurnal);

    let band = |p: &Prediction, b: &str| p.bands.iter().find(|x| x.band == b).unwrap().usable;
    // 白天 20m 可用；夜里只剩 40m 以下的低波段。
    assert!(band(&day, "20m"));
    assert!(!band(&night, "20m"));
    assert!(band(&night, "80m"));
    assert!(band(&night, "40m"));
  }

  #[test]
  fn antipodal_path_does_not_collapse_to_null_island() {
    // 赤道上的正对跖点：向量和恰为零。退化时必须退回一个真实端点坐标，
    // 而不是把中点算成 (0°, 0°) —— 那会让地方时与昼夜因子全错。
    let (lat, lon) = midpoint(0.0, 0.0, 0.0, 180.0);
    assert!(lat.abs() < 1e-6, "lat {lat}");
    assert!(lon.abs() < 1e-6 || (lon - 180.0).abs() < 1e-6, "lon {lon}");
  }

  #[test]
  fn midpoint_handles_date_line() {
    // 东京（约 140°E）与檀香山（约 -158°W）跨日期变更线，中点应落在北太平洋（约 171°E）。
    let (lat, lon) = midpoint(35.7, 139.7, 21.3, -157.8);
    assert!(lat > 25.0 && lat < 45.0, "lat {lat}");
    assert!(lon.abs() > 150.0, "lon {lon}");
  }

  #[test]
  fn invalid_grid_returns_none() {
    assert_eq!(predict("XX", "IO91", 10, 100.0), None);
    assert_eq!(predict("OM89", "bad", 10, 100.0), None);
    assert_eq!(predict_at("XX", "IO91", 10, 100.0, Some(12.0)), None);
  }
}
