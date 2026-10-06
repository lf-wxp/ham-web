//! CTCSS / DCS 亚音静噪码表与中继台频差。

/// 标准 CTCSS 亚音频率（Hz，EIA/TIA-603，50 组）。
pub const CTCSS_TONES: &[f64] = &[
  67.0, 69.3, 71.9, 74.4, 77.0, 79.7, 82.5, 85.4, 88.5, 91.5, 94.8, 97.4, 100.0, 103.5, 107.2,
  110.9, 114.8, 118.8, 123.0, 127.3, 131.8, 136.5, 141.3, 146.2, 151.4, 156.7, 159.8, 162.2, 165.5,
  167.9, 171.3, 173.8, 177.3, 179.9, 183.5, 186.2, 189.9, 192.8, 196.6, 199.5, 203.5, 206.5, 210.7,
  218.1, 225.7, 229.1, 233.6, 241.8, 250.3, 254.1,
];

/// 标准 DCS 数字静噪码（八进制，按 Rust 八进制字面量存储，UI 以 `{:03o}` 展示）。
pub const DCS_CODES: &[u16] = &[
  0o023, 0o025, 0o026, 0o031, 0o032, 0o036, 0o043, 0o047, 0o051, 0o053, 0o054, 0o065, 0o071, 0o072,
  0o073, 0o074, 0o114, 0o115, 0o116, 0o122, 0o125, 0o131, 0o132, 0o134, 0o143, 0o145, 0o152, 0o155,
  0o156, 0o162, 0o165, 0o172, 0o174, 0o205, 0o212, 0o223, 0o225, 0o226, 0o243, 0o244, 0o245, 0o246,
  0o251, 0o252, 0o255, 0o261, 0o263, 0o265, 0o266, 0o271, 0o274, 0o306, 0o311, 0o315, 0o325, 0o331,
  0o332, 0o343, 0o346, 0o351, 0o356, 0o364, 0o365, 0o371, 0o411, 0o412, 0o413, 0o423, 0o431, 0o432,
  0o445, 0o446, 0o452, 0o454, 0o455, 0o462, 0o464, 0o465, 0o466, 0o503, 0o506, 0o516, 0o523, 0o526,
  0o532, 0o546, 0o565, 0o606, 0o612, 0o624, 0o627, 0o631, 0o632, 0o654, 0o662, 0o664, 0o703, 0o712,
  0o723, 0o731, 0o732, 0o734, 0o743, 0o754,
];

/// 中继台频差表：`(波段, 频段范围, 标准频差 MHz, 说明)`。
pub const REPEATER_OFFSETS: &[(&str, &str, f64, &str)] = &[
  ("10 m", "29.5–29.7 MHz", 0.1, "常用 -100 kHz，依地区约定"),
  (
    "6 m",
    "50–54 MHz",
    0.5,
    "常用 -500 kHz 或 -1 MHz，依地区约定",
  ),
  (
    "2 m",
    "144–148 MHz",
    0.6,
    "多用 -600 kHz（三区），依地区约定",
  ),
  (
    "1.25 m",
    "222–225 MHz",
    1.6,
    "美国 222–225 MHz 波段常用 -1.6 MHz（我国未开放该波段）",
  ),
  (
    "70 cm",
    "430–440 MHz",
    5.0,
    "多用 -5 MHz（我国），依地区约定",
  ),
  (
    "33 cm",
    "902–928 MHz",
    12.0,
    "美国 902–928 MHz 波段常用 +12 MHz（我国未开放该波段）",
  ),
  (
    "23 cm",
    "1240–1300 MHz",
    20.0,
    "依地区约定（我国业余为 1240–1260 MHz）",
  ),
];

/// 由波段名称查标准频差（MHz）。
#[must_use]
pub fn offset_for_band(band: &str) -> Option<f64> {
  REPEATER_OFFSETS
    .iter()
    .find(|(name, _, _, _)| *name == band)
    .map(|(_, _, offset, _)| *offset)
}

/// 由接收频率与频差计算发射频率（MHz）。`positive` 表示「发射 = 接收 + 频差」（正偏移）。
#[must_use]
pub fn transmit_frequency(rx_mhz: f64, offset_mhz: f64, positive: bool) -> f64 {
  if positive {
    rx_mhz + offset_mhz
  } else {
    rx_mhz - offset_mhz
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ctcss_has_50_tones() {
    assert_eq!(CTCSS_TONES.len(), 50);
    // 递增且覆盖常见 88.5 / 100.0。
    assert!(CTCSS_TONES.windows(2).all(|w| w[0] < w[1]));
    assert!(CTCSS_TONES.contains(&88.5));
    assert!(CTCSS_TONES.contains(&100.0));
  }

  #[test]
  fn dcs_codes_are_three_digit_octal() {
    for &code in DCS_CODES {
      assert!((0o023..=0o754).contains(&code), "{code:03o} 超出范围");
    }
    assert!(DCS_CODES.contains(&0o023));
    assert!(DCS_CODES.contains(&0o754));
  }

  #[test]
  fn offset_lookup() {
    assert_eq!(offset_for_band("2 m"), Some(0.6));
    assert_eq!(offset_for_band("70 cm"), Some(5.0));
    assert_eq!(offset_for_band("未知"), None);
  }

  #[test]
  fn transmit_frequency_direction() {
    assert!((transmit_frequency(145.0, 0.6, false) - 144.4).abs() < 1e-9);
    assert!((transmit_frequency(439.0, 5.0, true) - 444.0).abs() < 1e-9);
  }
}
