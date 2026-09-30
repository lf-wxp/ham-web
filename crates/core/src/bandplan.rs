//! IARU 业余波段规划（三区，中国大陆口径）：各波段内的模式子段分配。

/// 一个波段规划。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BandPlan {
  pub band: &'static str,
  pub freq_range: &'static str,
  /// (频率子段, 模式) 列表。
  pub segments: &'static [(&'static str, &'static str)],
}

/// 主要业余波段的模式子段分配。
pub const BAND_PLANS: &[BandPlan] = &[
  BandPlan {
    band: "160m",
    freq_range: "1.8–2.0 MHz",
    segments: &[
      ("1.800–1.830", "CW"),
      ("1.830–1.840", "窄带数据"),
      ("1.840–2.000", "话音（SSB）"),
    ],
  },
  BandPlan {
    band: "80m",
    freq_range: "3.5–3.9 MHz",
    segments: &[
      ("3.500–3.600", "CW"),
      ("3.600–3.700", "数据"),
      ("3.700–3.900", "话音（SSB）"),
    ],
  },
  BandPlan {
    band: "40m",
    freq_range: "7.0–7.2 MHz",
    segments: &[("7.000–7.100", "CW / 数据"), ("7.100–7.200", "话音（SSB）")],
  },
  BandPlan {
    band: "30m",
    freq_range: "10.10–10.15 MHz",
    segments: &[("10.100–10.150", "仅 CW / 窄带数据（WARC）")],
  },
  BandPlan {
    band: "20m",
    freq_range: "14.0–14.35 MHz",
    segments: &[("14.000–14.150", "CW"), ("14.150–14.350", "话音（SSB）")],
  },
  BandPlan {
    band: "17m",
    freq_range: "18.068–18.168 MHz",
    segments: &[("18.068–18.110", "CW"), ("18.110–18.168", "话音（WARC）")],
  },
  BandPlan {
    band: "15m",
    freq_range: "21.0–21.45 MHz",
    segments: &[("21.000–21.150", "CW"), ("21.150–21.450", "话音（SSB）")],
  },
  BandPlan {
    band: "12m",
    freq_range: "24.89–24.99 MHz",
    segments: &[("24.890–24.930", "CW"), ("24.930–24.990", "话音（WARC）")],
  },
  BandPlan {
    band: "10m",
    freq_range: "28.0–29.7 MHz",
    segments: &[("28.000–28.200", "CW"), ("28.200–29.700", "话音 / 信标")],
  },
  BandPlan {
    band: "6m",
    freq_range: "50–54 MHz",
    segments: &[
      ("50.0–50.1", "CW / 信标"),
      ("50.1–50.3", "SSB"),
      ("50.3–54.0", "FM / 数据"),
    ],
  },
  BandPlan {
    band: "2m",
    freq_range: "144–148 MHz",
    segments: &[
      ("144.000–144.100", "CW / 数据"),
      ("144.100–144.400", "SSB"),
      ("144.400–145.800", "FM / 中继"),
      ("145.800–146.000", "业余卫星"),
    ],
  },
  BandPlan {
    band: "70cm",
    freq_range: "430–440 MHz",
    segments: &[
      ("430.000–432.000", "各种模式"),
      ("432.000–438.000", "弱信号 / 卫星"),
      ("438.000–440.000", "FM / 中继"),
    ],
  },
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn band_plans_are_populated() {
    assert!(BAND_PLANS.len() >= 10);
    for b in BAND_PLANS {
      assert!(!b.band.is_empty());
      assert!(!b.segments.is_empty());
    }
  }
}
