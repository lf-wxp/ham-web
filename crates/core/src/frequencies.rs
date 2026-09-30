//! 常用频率速查：遇险应急、国际信标、呼叫频率、APRS 与数字模式常用频率。

/// 一组频率。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreqGroup {
  pub category: &'static str,
  /// (频率, 用途) 列表。
  pub freqs: &'static [(&'static str, &'static str)],
}

/// 常用频率分组。
pub const FREQ_GROUPS: &[FreqGroup] = &[
  FreqGroup {
    category: "遇险与应急",
    freqs: &[
      ("2182 kHz", "海上遇险与呼叫（话音）"),
      ("121.5 MHz", "航空遇险（VHF）"),
      ("243.0 MHz", "航空遇险（军用）"),
      ("406.0–406.1 MHz", "卫星应急示位信标 EPIRB（COSPAS-SARSAT）"),
    ],
  },
  FreqGroup {
    category: "国际信标网络（NCDXF/IARU）",
    freqs: &[
      ("14.100 MHz", "20m 信标"),
      ("18.110 MHz", "17m 信标"),
      ("21.150 MHz", "15m 信标"),
      ("24.930 MHz", "12m 信标"),
      ("28.200 MHz", "10m 信标"),
    ],
  },
  FreqGroup {
    category: "DX / SSB 呼叫",
    freqs: &[
      ("50.110 MHz", "6m DX 呼叫（SSB/CW）"),
      ("144.200 MHz", "2m SSB 呼叫"),
      ("432.100 MHz", "70cm SSB 呼叫"),
      ("29.600 MHz", "10m FM 呼叫"),
    ],
  },
  FreqGroup {
    category: "FM 呼叫（中国常用）",
    freqs: &[
      ("145.000 / 435.000 MHz", "VHF / UHF 常用呼叫"),
      ("145.500 MHz", "2m FM 呼叫"),
      ("438.500 MHz", "70cm FM 呼叫"),
    ],
  },
  FreqGroup {
    category: "APRS",
    freqs: &[
      ("144.390 MHz", "北美 APRS 常用频率"),
      ("144.640 MHz", "中国 APRS 常用频率"),
    ],
  },
  FreqGroup {
    category: "数字模式常用（FT8 / RTTY / SSTV / PSK）",
    freqs: &[
      ("7.074 MHz", "40m FT8"),
      ("14.074 MHz", "20m FT8"),
      ("21.074 MHz", "15m FT8"),
      ("28.074 MHz", "10m FT8"),
      ("50.313 MHz", "6m FT8"),
      ("144.174 MHz", "2m FT8"),
      ("14.080 MHz", "20m RTTY"),
      ("14.071 MHz", "20m PSK31"),
      ("14.230 MHz", "20m SSTV"),
    ],
  },
];

/// 根据频率（MHz）推断业余波段名称，无法归入常用业余波段时返回「其他」。
#[must_use]
pub fn band_of(freq_mhz: f64) -> &'static str {
  if (1.8..2.0).contains(&freq_mhz) {
    "160m"
  } else if (3.5..4.0).contains(&freq_mhz) {
    "80m"
  } else if (5.3515..5.3666).contains(&freq_mhz) {
    "60m"
  } else if (7.0..7.3).contains(&freq_mhz) {
    "40m"
  } else if (10.1..10.15).contains(&freq_mhz) {
    "30m"
  } else if (14.0..14.35).contains(&freq_mhz) {
    "20m"
  } else if (18.0..18.168).contains(&freq_mhz) {
    "17m"
  } else if (21.0..21.45).contains(&freq_mhz) {
    "15m"
  } else if (24.89..24.99).contains(&freq_mhz) {
    "12m"
  } else if (28.0..29.7).contains(&freq_mhz) {
    "10m"
  } else if (50.0..54.0).contains(&freq_mhz) {
    "6m"
  } else if (144.0..148.0).contains(&freq_mhz) {
    "2m"
  } else if (430.0..440.0).contains(&freq_mhz) {
    "70cm"
  } else {
    "其他"
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn frequencies_grouped() {
    let total: usize = FREQ_GROUPS.iter().map(|g| g.freqs.len()).sum();
    assert!(total >= 20, "应有足够多的频率条目，实际 {total}");
    for g in FREQ_GROUPS {
      assert!(!g.category.is_empty());
      for (f, u) in g.freqs {
        assert!(!f.is_empty());
        assert!(!u.is_empty());
      }
    }
  }

  #[test]
  fn band_lookup() {
    assert_eq!(band_of(14.074), "20m");
    assert_eq!(band_of(7.074), "40m");
    assert_eq!(band_of(144.5), "2m");
    assert_eq!(band_of(433.0), "70cm");
    assert_eq!(band_of(1.2), "其他");
  }
}
