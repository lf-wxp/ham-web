//! APRS：自动位置报告系统。

/// 核心概念。
pub const APRS_CONCEPTS: &[(&str, &str)] = &[
  (
    "APRS",
    "Automatic Packet Reporting System，用分组无线电自动广播位置、气象、短消息等信息。",
  ),
  (
    "定位",
    "通过 GPS 获取坐标，用 AX.25 分组经 VHF 发射，由数字中继（digipeater）转发。",
  ),
  (
    "IGate",
    "互联网网关，把收到的 APRS 分组转发到互联网，实现全球跟踪。",
  ),
  (
    "数字中继",
    "digipeater，接收并转发 APRS 分组，扩大覆盖范围。",
  ),
  ("信标", "台站按设定周期自动发送位置或状态分组。"),
];

/// 常用频率。
pub const APRS_FREQS: &[(&str, &str)] = &[
  ("144.390 MHz", "北美及多数地区 APRS 标准频率（VHF）。"),
  ("144.640 MHz", "中国 APRS 常用频率。"),
  ("144.800 MHz", "部分欧洲地区 APRS 频率。"),
];

/// 主要应用。
pub const APRS_USES: &[(&str, &str)] = &[
  ("位置追踪", "车辆、徒步、气球、船舶实时位置显示在地图上。"),
  ("气象站", "自动上传温湿度、气压、风速等数据。"),
  ("短消息", "台站之间发送文本消息与问候。"),
  ("应急定位", "灾害中用于人员与物资定位，是应急通信重要手段。"),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn aprs_data_populated() {
    assert!(!APRS_CONCEPTS.is_empty());
    assert!(!APRS_FREQS.is_empty());
    assert!(!APRS_USES.is_empty());
  }
}
