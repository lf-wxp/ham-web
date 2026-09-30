//! 无线电传播与电离层速查：电离层分层、传播模式与关键概念。

/// 电离层各层。
pub const LAYERS: &[(&str, &str, &str)] = &[
  (
    "D 层",
    "约 60–90 km",
    "仅白天存在，主要吸收 LF/MF 信号，日落消失。",
  ),
  (
    "E 层",
    "约 90–130 km",
    "可反射较低频 HF；偶发 Es 层能反射 VHF 信号。",
  ),
  ("F1 层", "约 180–250 km", "仅白天存在，夜间与 F2 层合并。"),
  (
    "F2 层",
    "约 250–400 km",
    "HF 远距离通信的主要反射层，昼夜都存在。",
  ),
];

/// 主要传播方式。
pub const PROPAGATION_MODES: &[(&str, &str)] = &[
  ("地波（表面波）", "沿地表传播，用于 LF/MF 及近距离通信。"),
  ("天波", "经电离层反射，是 HF 远距离通信的主要方式。"),
  ("直线波（空间波）", "视距传播，用于 VHF/UHF 及以上频段。"),
  ("散射传播", "对流层/电离层散射、流星余迹等特殊方式。"),
];

/// 关键概念。
pub const CONCEPTS: &[(&str, &str)] = &[
  (
    "太阳黑子数 SSN",
    "反映太阳活动，黑子多时 HF 高波段传播条件改善。",
  ),
  ("MUF 最高可用频率", "某时刻两点间能被电离层反射的最高频率。"),
  ("LUF 最低可用频率", "某时刻两点间能被反射的最低频率。"),
  ("静区（跳越区）", "地波与天波都覆盖不到的环形区域。"),
  ("衰落", "信号因多径干涉等因素出现的强弱起伏。"),
];

/// 低频段（160m/80m）DX 要点。
pub const LOW_BAND_TIPS: &[&str] = &[
  "160m/80m 冬季大气噪声低，是远距离 DX 的黄金季节。",
  "灰线（日出日落）时刻是低频 DX 的最佳通联窗口。",
  "低波段受地波与电离层吸收影响大，需要好天线并耐心守听。",
  "使用窄带接收、降低本底噪声，弱信号更容易分辨。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn propagation_data_populated() {
    assert!(!LAYERS.is_empty());
    assert!(!PROPAGATION_MODES.is_empty());
    assert!(!CONCEPTS.is_empty());
    assert!(!LOW_BAND_TIPS.is_empty());
    for &(n, r, d) in LAYERS {
      assert!(!n.is_empty() && !r.is_empty() && !d.is_empty());
    }
  }
}
