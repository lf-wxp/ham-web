//! LoRa 链路计算内核：空中时间、接收灵敏度、链路预算与最大距离。
//!
//! 公式依据 Semtech 应用笔记 **AN1200.13「LoRa 调制基础」**（其符号时间、前导码、有效载荷符号数与
//! 接收灵敏度在开源实现与社区计算器中被广泛转述；本模块的数值对 calc.engineer 的 `lora-airtime`
//! 逐例一致，见单测）。**未取到 AN1200.13 官方 PDF 原文**（Semtech 站返回的是跳转页而非文档），
//! 因此「代码与社区口径一致」不等于「与规范原文一致」，页面里写明。
//!
//! # 空中时间（Time on Air）
//!
//! ```text
//! T_sym        = 2^SF / BW                                   （符号周期，BW 单位 Hz）
//! T_preamble   = (N_preamble + 4.25) · T_sym                 （4.25 是同步/报头开销）
//! payloadSymbNb = 8 + max(0, ⌈ (8·PL − 4·SF + 28 + 16·CRC − 20·H) / (4·(SF − 2·DE)) ⌉ · (CR + 4))
//! T_payload    = payloadSymbNb · T_sym
//! ToA          = T_preamble + T_payload
//! ```
//!
//! - `PL` 有效载荷字节数（1–255）；`CRC` 启用=1；`H` 隐式报头=1、显式=0；`DE` 低速率优化启用=1；
//! - `CR` 编码率 1–4，对应 4/5、4/6、4/7、4/8；
//! - `DE` 的「自动」判据是**符号时间 > 16 ms**（见 [`low_data_rate_optimize_auto`]），与 SF 和带宽都有关：
//!   125 kHz 下是 SF11 / SF12；62.5 kHz 还包括 SF10；250 kHz 只有 SF12；500 kHz 没有。公式本身照 `DE`
//!   的 0 / 1 代入，手动强制开关时不做物理限制。
//!
//! # 接收灵敏度
//!
//! ```text
//! S(dBm) = −174 + 10·lg(BW_Hz) + NF + SNR_limit(SF)
//! ```
//!
//! `−174 dBm/Hz` 是 290 K 的热噪声密度；`NF` 是接收机噪声系数（硬件相关，典型 4–7 dB）。
//! `SNR_limit` 取自 Semtech 数据手册的解调门限表（也是 AN1200.13 引用的那张）：
//!
//! | SF | 7 | 8 | 9 | 10 | 11 | 12 |
//! |--:|--:|--:|--:|---:|---:|---:|
//! | SNR(dB) | −7.5 | −10 | −12.5 | −15 | −17.5 | −20 |
//!
//! SF6 需要隐式报头、payload 公式另有差异，本模块不支持（LoRaWAN 与业余应用的主流也是 SF7–SF12）。
//!
//! # 链路预算
//!
//! 复用 [`crate::link_budget`] 的自由空间损耗（Friis 的 dB 形式），算出给定距离的接收功率、链路裕量，
//! 以及令裕量归零的最大距离。**这是自由空间理想值**：不考虑地球曲率、地形、遮挡与多径，实际可达距离
//! 受视距几何与传播环境限制，远小于理想值（见页面说明）。

use crate::link_budget::received_dbm;

/// 支持的扩频因子范围。
pub const SF_RANGE: std::ops::RangeInclusive<u8> = 7..=12;
/// 编码率范围（1=4/5 … 4=4/8）。
pub const CR_RANGE: std::ops::RangeInclusive<u8> = 1..=4;
/// 有效载荷字节数范围。
pub const PL_RANGE: std::ops::RangeInclusive<u16> = 1..=255;
/// 前导码符号数范围（LoRa 最小 6）。
pub const PREAMBLE_RANGE: std::ops::RangeInclusive<u16> = 6..=65535;
/// 常见带宽（Hz），LoRaWAN 与业余设备主流。
///
/// 最窄的一档是 **62.5 kHz**（SX127x 的 7.8–500 kHz 档位里的 62.5），写成 62 kHz 会让符号时间与
/// 灵敏度都偏差约 0.8%，所以用 Hz 存、避免取整。
pub const BW_HZ_CHOICES: &[u32] = &[62_500, 125_000, 250_000, 500_000];
/// 低速率优化（LDRO）自动判据：符号时间超过 16 ms（Semtech 数据手册 / AN1200.13 的建议）。
const LDRO_SYMBOL_TIME_S: f64 = 0.016;

/// 扩频因子的解调 SNR 门限（dB），按 SF7–SF12 顺序。见模块文档的表。
const SNR_LIMIT_DB: [f64; 6] = [-7.5, -10.0, -12.5, -15.0, -17.5, -20.0];

/// 给定扩频因子的解调 SNR 门限（dB）；SF 超出 [`SF_RANGE`] 为 `None`。
#[must_use]
pub fn snr_limit_db(sf: u8) -> Option<f64> {
  SF_RANGE
    .contains(&sf)
    .then(|| SNR_LIMIT_DB[(sf - 7) as usize])
}

/// 编码率的分子分母：CR 1–4 → `(4/5, 4/6, 4/7, 4/8)`；非法为 `None`。
#[must_use]
pub fn coding_rate(rate: u8) -> Option<(u8, u8)> {
  CR_RANGE.contains(&rate).then(|| (4, rate + 4))
}

/// 符号周期（秒）：`2^SF / BW_Hz`。SF 或带宽非法为 `None`。
#[must_use]
pub fn symbol_time_s(sf: u8, bw_hz: f64) -> Option<f64> {
  (SF_RANGE.contains(&sf) && bw_hz > 0.0 && bw_hz.is_finite())
    .then(|| f64::from(1u32 << sf) / bw_hz)
}

/// 低速率优化的「自动」取值：符号时间 > 16 ms 时应启用。SF 或带宽非法为 `None`。
///
/// 只看 SF 是错的：SF11 在 250 kHz 下符号时间仅 8.2 ms 不需要，而 SF10 在 62.5 kHz 下是
/// 16.4 ms 就需要。
#[must_use]
pub fn low_data_rate_optimize_auto(sf: u8, bw_hz: f64) -> Option<bool> {
  symbol_time_s(sf, bw_hz).map(|tsym| tsym > LDRO_SYMBOL_TIME_S)
}

/// 把输入框里的数值收成 `u16`：必须是 0–65535 的**整数**，否则 `None`。
///
/// 有效载荷字节数与前导码符号数都是整数。输入 `12.5` 时不能悄悄四舍五入去算 ——
/// 界面上还显示着 `12.5`，结果却是按 13 算的，两边对不上。
#[must_use]
pub fn whole_u16(v: f64) -> Option<u16> {
  // `as` 对越界值是饱和转换、不会 panic；是否采用由前面的条件决定。
  (v.is_finite() && v.fract() == 0.0 && (0.0..=f64::from(u16::MAX)).contains(&v))
    .then_some(v as u16)
}

/// 前导码时间（秒）：`(N_preamble + 4.25) · T_sym`。前导码符号数非法为 `None`。
#[must_use]
pub fn preamble_time_s(npreamble: u16, symbol_time_s: f64) -> Option<f64> {
  (PREAMBLE_RANGE.contains(&npreamble) && symbol_time_s.is_finite() && symbol_time_s > 0.0)
    .then(|| (f64::from(npreamble) + 4.25) * symbol_time_s)
}

/// 有效载荷符号数（向上取整，是整数）：见模块文档的 `payloadSymbNb` 公式。
///
/// `crc_enabled` 与 `implicit_header` 只影响公式里的两个常数项；`low_data_rate_opt` 改的是分母里的
/// `2·DE`（自动取值见 [`low_data_rate_optimize_auto`]）。
/// 参数非法（SF / CR / PL 超出范围）为 `None`。
#[must_use]
pub fn payload_symbols(
  sf: u8,
  payload_bytes: u16,
  rate: u8,
  crc_enabled: bool,
  implicit_header: bool,
  low_data_rate_opt: bool,
) -> Option<u64> {
  if !SF_RANGE.contains(&sf) || !CR_RANGE.contains(&rate) || !PL_RANGE.contains(&payload_bytes) {
    return None;
  }
  let numerator = 8.0 * f64::from(payload_bytes) - 4.0 * f64::from(sf)
    + 28.0
    + if crc_enabled { 16.0 } else { 0.0 }
    - if implicit_header { 20.0 } else { 0.0 };
  let denominator = 4.0 * (f64::from(sf) - if low_data_rate_opt { 2.0 } else { 0.0 });
  // 分母在 SF≥7 且 DE 任意组合下都 ≥ 20，恒正，不会除零。
  let symbols = 8.0 + (numerator / denominator).ceil().max(0.0) * f64::from(rate + 4);
  // symbols 恒 ≥ 8（见上式），`as u64` 安全。
  Some(symbols.round() as u64)
}

/// 一次空中时间计算的完整参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PacketParams {
  pub sf: u8,
  pub bw_hz: f64,
  pub npreamble: u16,
  pub payload_bytes: u16,
  pub rate: u8,
  pub crc_enabled: bool,
  pub implicit_header: bool,
  pub low_data_rate_opt: bool,
}

/// 完整空中时间（秒）：前导码 + 有效载荷。参数非法为 `None`。
#[must_use]
pub fn time_on_air_s(p: &PacketParams) -> Option<f64> {
  let tsym = symbol_time_s(p.sf, p.bw_hz)?;
  let payload = f64::from(
    u32::try_from(payload_symbols(
      p.sf,
      p.payload_bytes,
      p.rate,
      p.crc_enabled,
      p.implicit_header,
      p.low_data_rate_opt,
    )?)
    .ok()?,
  );
  let preamble = preamble_time_s(p.npreamble, tsym)?;
  Some(preamble + payload * tsym)
}

/// 接收灵敏度（dBm）：`−174 + 10·lg(BW_Hz) + NF + SNR_limit(SF)`。SF / 带宽 / 噪声系数非法为 `None`。
#[must_use]
pub fn sensitivity_dbm(sf: u8, bw_hz: f64, noise_figure_db: f64) -> Option<f64> {
  if !SF_RANGE.contains(&sf) || !(bw_hz > 0.0 && bw_hz.is_finite()) || !noise_figure_db.is_finite()
  {
    return None;
  }
  Some(-174.0 + 10.0 * bw_hz.log10() + noise_figure_db + snr_limit_db(sf)?)
}

/// 最大距离（km）：令「接收功率 = 灵敏度」反解自由空间损耗。
///
/// 输入含义与 [`received_dbm`] 一致（发射功率 dBm、收发天线增益 dBi、频率 MHz、附加损耗 dB）。
/// 这是自由空间理想值，不含地球曲率 / 地形 / 遮挡。任一参数非法为 `None`。
#[must_use]
pub fn max_distance_km(
  tx_dbm: f64,
  tx_gain_dbi: f64,
  rx_gain_dbi: f64,
  freq_mhz: f64,
  extra_loss_db: f64,
  sensitivity: f64,
) -> Option<f64> {
  if ![tx_dbm, tx_gain_dbi, rx_gain_dbi, extra_loss_db, sensitivity]
    .iter()
    .all(|v| v.is_finite())
    || !(freq_mhz > 0.0 && freq_mhz.is_finite())
  {
    return None;
  }
  // 最大允许路径损耗 = 全部增益 − 灵敏度 − 附加损耗。
  let fspl_max = tx_dbm + tx_gain_dbi + rx_gain_dbi - sensitivity - extra_loss_db;
  // FSPL = 20·lg(f) + 20·lg(d) + 32.45 → d = 10^((FSPL − 32.45 − 20·lg(f)) / 20)。
  let d = 10f64.powf((fspl_max - 32.45 - 20.0 * freq_mhz.log10()) / 20.0);
  (d > 0.0 && d.is_finite()).then_some(d)
}

/// 链路裕量（dB）：给定距离的接收功率 − 灵敏度。正值为有富余，负值为不够。
#[must_use]
pub fn link_margin_db(
  tx_dbm: f64,
  tx_gain_dbi: f64,
  rx_gain_dbi: f64,
  freq_mhz: f64,
  dist_km: f64,
  extra_loss_db: f64,
  sensitivity: f64,
) -> Option<f64> {
  received_dbm(
    tx_dbm,
    tx_gain_dbi,
    rx_gain_dbi,
    freq_mhz,
    dist_km,
    extra_loss_db,
  )
  .and_then(|rx| (rx - sensitivity).is_finite().then_some(rx - sensitivity))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() < tol
  }

  fn s12() -> f64 {
    sensitivity_dbm(12, 125_000.0, 6.0).unwrap()
  }

  /// 数值来自独立 Python 脚本（照 AN1200.13 公式，不复用本模块代码），并与 calc.engineer 的
  /// `lora-airtime` 对拍：`SF7 / 125 kHz / CR1 / PL10 / 显式报头 / CRC / 无低速率优化 → ToA 41.216 ms`。
  #[test]
  fn symbol_time_matches_the_reference() {
    assert!(near(symbol_time_s(7, 125_000.0).unwrap(), 1.024e-3, 1e-12));
    assert!(near(symbol_time_s(12, 500_000.0).unwrap(), 8.192e-3, 1e-12));
    assert_eq!(symbol_time_s(6, 125_000.0), None);
    assert_eq!(symbol_time_s(13, 125_000.0), None);
    assert_eq!(symbol_time_s(7, 0.0), None);
  }

  #[test]
  fn payload_symbol_count_matches_the_independent_script() {
    // (sf, pl, cr, crc, ih, de, 期望)
    for (sf, pl, cr, crc, ih, de, want) in [
      (7, 10, 1, true, false, false, 28),
      (7, 20, 1, true, false, false, 43),
      (7, 10, 4, true, false, false, 40),
      (12, 10, 1, true, false, true, 18),
      (9, 50, 2, true, false, false, 80),
      (7, 10, 1, true, true, false, 23),   // 隐式报头：−20 项生效
      (7, 10, 1, false, false, false, 23), // 关 CRC：少 +16，ceil(80/28)=3 → 15 → 23 个符号
    ] {
      assert_eq!(
        payload_symbols(sf, pl, cr, crc, ih, de),
        Some(want),
        "sf={sf} pl={pl} cr={cr}"
      );
    }
  }

  #[test]
  fn time_on_air_matches_the_reference_examples() {
    // 每条 (np, sf, bw, pl, cr, crc, ih, de, ToA ms)
    for (np, sf, bw, pl, cr, crc, ih, de, ms) in [
      (8, 7, 125_000, 10, 1, true, false, false, 41.216),
      (8, 7, 125_000, 20, 1, true, false, false, 56.576),
      (8, 7, 125_000, 10, 4, true, false, false, 53.504),
      (8, 12, 125_000, 10, 1, true, false, true, 991.232),
      (8, 12, 500_000, 10, 1, true, false, true, 247.808),
      (8, 9, 250_000, 50, 2, true, false, false, 188.928),
      (8, 7, 125_000, 10, 1, true, true, false, 36.096),
    ] {
      let toa = time_on_air_s(&PacketParams {
        sf,
        bw_hz: bw as f64,
        npreamble: np,
        payload_bytes: pl,
        rate: cr,
        crc_enabled: crc,
        implicit_header: ih,
        low_data_rate_opt: de,
      })
      .unwrap();
      assert!(
        near(toa, ms / 1e3, 0.0006),
        "sf={sf} bw={bw} pl={pl}: {toa}"
      );
    }
  }

  #[test]
  fn sensitivity_matches_the_semtech_table_at_nf_6() {
    // calc.engineer 与 Semtech 数据手册口径：NF=6、BW=125 kHz。
    for (sf, want) in [
      (7, -124.53),
      (8, -127.03),
      (9, -129.53),
      (10, -132.03),
      (11, -134.53),
      (12, -137.03),
    ] {
      assert!(
        near(sensitivity_dbm(sf, 125_000.0, 6.0).unwrap(), want, 0.005),
        "sf={sf}"
      );
    }
    // 带宽 / 噪声系数变化。
    assert!(near(
      sensitivity_dbm(7, 250_000.0, 6.0).unwrap(),
      -121.52,
      0.01
    ));
    assert!(near(
      sensitivity_dbm(12, 125_000.0, 4.0).unwrap(),
      -139.03,
      0.01
    ));
    assert!(near(
      sensitivity_dbm(10, 500_000.0, 7.0).unwrap(),
      -125.01,
      0.01
    ));
  }

  #[test]
  fn snr_limit_covers_exactly_sf7_to_sf12() {
    assert_eq!(snr_limit_db(7), Some(-7.5));
    assert_eq!(snr_limit_db(12), Some(-20.0));
    assert_eq!(snr_limit_db(6), None);
    assert_eq!(snr_limit_db(13), None);
  }

  #[test]
  fn coding_rate_maps_to_the_four_fractions() {
    assert_eq!(coding_rate(1), Some((4, 5)));
    assert_eq!(coding_rate(2), Some((4, 6)));
    assert_eq!(coding_rate(3), Some((4, 7)));
    assert_eq!(coding_rate(4), Some((4, 8)));
    assert_eq!(coding_rate(0), None);
    assert_eq!(coding_rate(5), None);
  }

  #[test]
  fn max_distance_matches_the_independent_script() {
    let s12 = sensitivity_dbm(12, 125_000.0, 6.0).unwrap(); // −137.03
    // 14 dBm、各 3 dBi、470 MHz → 3605 km（自由空间理想值）。
    assert!(near(
      max_distance_km(14.0, 3.0, 3.0, 470.0, 0.0, s12).unwrap(),
      3605.35,
      0.05
    ));
    // 20 dBm → 7193 km。
    assert!(near(
      max_distance_km(20.0, 3.0, 3.0, 470.0, 0.0, s12).unwrap(),
      7193.61,
      0.05
    ));
    // 433 MHz、SF7、偶极子 2.15 dBi → 763 km。
    let s7 = sensitivity_dbm(7, 125_000.0, 6.0).unwrap();
    assert!(near(
      max_distance_km(14.0, 2.15, 2.15, 433.0, 0.0, s7).unwrap(),
      763.06,
      0.05
    ));
  }

  #[test]
  fn link_margin_matches_the_independent_script() {
    let s12 = sensitivity_dbm(12, 125_000.0, 6.0).unwrap();
    // 500 km、470 MHz、14 dBm、各 3 dBi → 接收 −119.87，裕量 17.16 dB。
    let m = link_margin_db(14.0, 3.0, 3.0, 470.0, 500.0, 0.0, s12).unwrap();
    assert!(near(m, 17.16, 0.02), "{m}");
    // 距离大到超过自由空间理想值：裕量为负。
    assert!(link_margin_db(14.0, 3.0, 3.0, 470.0, 5000.0, 0.0, s12).unwrap() < 0.0);
  }

  /// LDRO 自动档看的是符号时间 > 16 ms，不是「SF ≥ 11」：同一个 SF 换带宽，结论会变。
  #[test]
  fn ldro_auto_follows_symbol_time_not_just_sf() {
    // 125 kHz：SF10 = 8.192 ms 不需要；SF11 = 16.384 ms、SF12 = 32.768 ms 需要。
    assert_eq!(low_data_rate_optimize_auto(10, 125_000.0), Some(false));
    assert_eq!(low_data_rate_optimize_auto(11, 125_000.0), Some(true));
    assert_eq!(low_data_rate_optimize_auto(12, 125_000.0), Some(true));
    // 250 kHz：SF11 只有 8.192 ms 不需要；SF12 = 16.384 ms 需要。
    assert_eq!(low_data_rate_optimize_auto(11, 250_000.0), Some(false));
    assert_eq!(low_data_rate_optimize_auto(12, 250_000.0), Some(true));
    // 500 kHz：SF12 = 8.192 ms，不需要。
    assert_eq!(low_data_rate_optimize_auto(12, 500_000.0), Some(false));
    // 62.5 kHz：SF10 = 16.384 ms 就需要，SF9 = 8.192 ms 不需要。
    assert_eq!(low_data_rate_optimize_auto(10, 62_500.0), Some(true));
    assert_eq!(low_data_rate_optimize_auto(9, 62_500.0), Some(false));
    assert_eq!(low_data_rate_optimize_auto(6, 125_000.0), None);
    assert_eq!(low_data_rate_optimize_auto(7, 0.0), None);
  }

  /// 最窄一档是 62.5 kHz，不是 62 kHz；符号时间与灵敏度都按 62500 Hz 算。
  #[test]
  fn narrowest_bandwidth_is_62_5_khz() {
    assert_eq!(BW_HZ_CHOICES, &[62_500, 125_000, 250_000, 500_000]);
    assert!(near(symbol_time_s(7, 62_500.0).unwrap(), 2.048e-3, 1e-12));
    assert!(near(
      sensitivity_dbm(7, 62_500.0, 6.0).unwrap(),
      -127.54,
      0.01
    ));
  }

  /// 整数字段只收整数：`12.5` 不能悄悄四舍五入去算。
  #[test]
  fn whole_u16_accepts_only_integers_in_range() {
    assert_eq!(whole_u16(12.0), Some(12));
    assert_eq!(whole_u16(0.0), Some(0));
    assert_eq!(whole_u16(65_535.0), Some(65_535));
    assert_eq!(whole_u16(12.5), None);
    assert_eq!(whole_u16(-1.0), None);
    assert_eq!(whole_u16(65_536.0), None);
    assert_eq!(whole_u16(f64::NAN), None);
    assert_eq!(whole_u16(f64::INFINITY), None);
  }

  #[test]
  fn invalid_inputs_are_refused() {
    let p = |sf, bw, np, pl, cr| PacketParams {
      sf,
      bw_hz: bw,
      npreamble: np,
      payload_bytes: pl,
      rate: cr,
      crc_enabled: true,
      implicit_header: false,
      low_data_rate_opt: false,
    };
    assert_eq!(time_on_air_s(&p(6, 125_000.0, 8, 10, 1)), None);
    assert_eq!(time_on_air_s(&p(13, 125_000.0, 8, 10, 1)), None);
    assert_eq!(time_on_air_s(&p(7, 125_000.0, 8, 0, 1)), None);
    assert_eq!(time_on_air_s(&p(7, 125_000.0, 8, 256, 1)), None);
    assert_eq!(time_on_air_s(&p(7, 125_000.0, 8, 10, 0)), None);
    assert_eq!(time_on_air_s(&p(7, 125_000.0, 5, 10, 1)), None);
    assert_eq!(sensitivity_dbm(7, 0.0, 6.0), None);
    assert_eq!(sensitivity_dbm(7, 125_000.0, f64::NAN), None);
    assert_eq!(max_distance_km(14.0, 3.0, 3.0, 0.0, 0.0, s12()), None);
    assert_eq!(max_distance_km(14.0, 3.0, 3.0, 470.0, 0.0, f64::NAN), None);
  }
}
