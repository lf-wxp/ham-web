//! NCDXF/IARU 国际信标网络（International Beacon Project）：
//! 18 个信标台在 5 个 HF 波段轮流发射，供判断传播路径开通情况。

/// 信标波段（波段名，频率 MHz）。
pub const BEACON_BANDS: &[(&str, f64)] = &[
  ("20m", 14.100),
  ("17m", 18.110),
  ("15m", 21.150),
  ("12m", 24.930),
  ("10m", 28.200),
];

/// 信标台（呼号，位置，洲），按轮询顺序排列。
pub const BEACONS: &[(&str, &str, &str)] = &[
  ("4U1UN", "联合国 · 美国纽约", "北美洲"),
  ("VE8AT", "加拿大", "北美洲"),
  ("W6WX", "美国", "北美洲"),
  ("KH6WO", "夏威夷", "太平洋"),
  ("ZL6B", "新西兰", "大洋洲"),
  ("VK6RBP", "澳大利亚", "大洋洲"),
  ("JA2IGY", "日本", "亚洲"),
  ("RR9O", "俄罗斯 · 亚洲", "亚洲"),
  ("VR2B", "香港", "亚洲"),
  ("4S7B", "斯里兰卡", "亚洲"),
  ("ZS6DN", "南非", "非洲"),
  ("5Z4B", "肯尼亚", "非洲"),
  ("4X6TU", "以色列", "亚洲"),
  ("OH2B", "芬兰", "欧洲"),
  ("CS3B", "马德拉", "欧洲"),
  ("LU4AA", "阿根廷", "南美洲"),
  ("OA4B", "秘鲁", "南美洲"),
  ("YV5B", "委内瑞拉", "南美洲"),
];

/// 递减功率 dash：每级下降 10 dB，用于估算路径损耗余量。
pub const DASH_POWERS: &[(&str, f64)] = &[
  ("第 1 划", 100.0),
  ("第 2 划", 10.0),
  ("第 3 划", 1.0),
  ("第 4 划", 0.1),
];

/// 一轮时长（秒）：18 台 × 10 秒。
pub const CYCLE_SECS: i64 = 180;
/// 每台信标发射时长（秒）。
pub const SLOT_SECS: i64 = 10;

/// 由 UTC epoch 秒计算「当前信标下标」与「进入该台的秒数（0..9）」。
///
/// 轮询以 UTC 整 3 分钟边界对齐（epoch 0 = 1970-01-01T00:00:00Z 恰为周期起点）。
pub fn beacon_slot(utc_epoch_secs: i64) -> (usize, i64) {
  let cycle = utc_epoch_secs.rem_euclid(CYCLE_SECS); // 0..179
  ((cycle / SLOT_SECS) as usize, cycle % SLOT_SECS)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn beacons_populated() {
    assert_eq!(BEACONS.len(), 18);
    assert_eq!(BEACON_BANDS.len(), 5);
    for (callsign, loc, region) in BEACONS {
      assert!(!callsign.is_empty());
      assert!(!loc.is_empty());
      assert!(!region.is_empty());
    }
  }

  #[test]
  fn dash_powers_descend_by_10db() {
    assert_eq!(DASH_POWERS.len(), 4);
    for i in 1..DASH_POWERS.len() {
      assert!((DASH_POWERS[i - 1].1 / DASH_POWERS[i].1 - 10.0).abs() < 1e-6);
    }
  }

  #[test]
  fn slot_cycle_boundaries() {
    assert_eq!(beacon_slot(0), (0, 0));
    assert_eq!(beacon_slot(9), (0, 9));
    assert_eq!(beacon_slot(10), (1, 0));
    assert_eq!(beacon_slot(179), (17, 9));
    assert_eq!(beacon_slot(180), (0, 0));
  }
}
