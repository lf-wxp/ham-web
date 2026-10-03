//! 极光通信（Aurora Propagation）：高纬地区利用极光区电离增强完成 VHF/UHF 通联。

/// 核心概念。
pub const AURORA_CONCEPTS: &[(&str, &str)] = &[
  (
    "极光传播",
    "太阳带电粒子沿地磁线注入极区高层大气，形成不规则电离体，可反射 VHF/UHF 电波。",
  ),
  (
    "极光椭圆",
    "极光活动集中在地磁纬度约 65°～70° 的环带；强地磁暴时环带向低纬扩展，中纬度也可能开通。",
  ),
  (
    "Kp 指数",
    "全球地磁活动指数（0～9），Kp ≥ 5 即地磁暴。数值越大，极光区越靠南、传播越强。",
  ),
  (
    "音色特征",
    "极光反射伴随强烈多径起伏，CW 听起来沙哑颤动、语音近似「低语」，是典型识别特征。",
  ),
  (
    "适用波段",
    "以 6m（50 MHz）与 2m（144 MHz）为主，70cm 偶有开通，更高频段极少见。",
  ),
  (
    "天线与极化",
    "天线需指向极光区（高纬的北 / 南方天空）；极光会使极化面持续旋转，圆极化天线占优。",
  ),
];

/// 常用波段（波段，频率，说明）。
pub const AURORA_BANDS: &[(&str, &str, &str)] = &[
  (
    "6m",
    "50 MHz",
    "极光通联最常用的波段，单次开通可达上千公里。",
  ),
  ("2m", "144 MHz", "次常用波段，信号较弱但同样可行。"),
  ("70cm", "432 MHz", "仅在强地磁暴时偶尔开通，属进阶尝试。"),
  ("1.25m", "222 MHz", "北美等地区可用，开通条件与 2m 相近。"),
];

/// 预测与监测（指标，含义）。
pub const AURORA_FORECAST: &[(&str, &str)] = &[
  (
    "Kp 指数",
    "NOAA SWPC 每 3 小时发布；Kp ≥ 5 时高纬地区极光传播概率明显上升。",
  ),
  (
    "太阳风速度",
    "持续高于约 500 km/s 时地磁扰动增强，是极光活动的上游驱动。",
  ),
  (
    "Bz（行星际磁场分量）",
    "南向（负值）且持续时能量更易注入极区，是极光活动的关键先兆。",
  ),
  (
    "极光预报",
    "NOAA 极光预报给出可见范围与概率，可作为判断开通时间与纬度的粗略参考。",
  ),
];

/// 操作要点。
pub const AURORA_TIPS: &[&str] = &[
  "盯住 Kp 与太阳风参数：地磁暴刚起、Kp 由 4 升到 5～7 时，往往是开通最猛的时段。",
  "优先用 6m 配合 CW 或数字模式（MSK144 / FT8）尝试，极光信号起伏快，短促交换更可靠。",
  "天线指向极光区，尽量使用圆极化或将水平天线转到最佳方位；极光会让极化面不断旋转。",
  "沙哑、颤动的音色属正常现象，不要误判为设备或馈线故障。",
  "信号含混时收窄带宽、放慢语音或改用 CW，可显著提高抄收率。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn aurora_data_populated() {
    assert!(AURORA_CONCEPTS.len() >= 4);
    assert!(AURORA_BANDS.len() >= 3);
    assert!(AURORA_FORECAST.len() >= 3);
    assert!(!AURORA_TIPS.is_empty());
    for (name, desc) in AURORA_CONCEPTS.iter().chain(AURORA_FORECAST) {
      assert!(!name.is_empty());
      assert!(!desc.is_empty());
    }
    for (band, freq, desc) in AURORA_BANDS {
      assert!(!band.is_empty());
      assert!(!freq.is_empty());
      assert!(!desc.is_empty());
    }
  }
}
