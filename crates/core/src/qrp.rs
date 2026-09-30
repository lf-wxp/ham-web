//! QRP 低功率操作：理念、设备与技巧。

/// 核心概念。
pub const QRP_CONCEPTS: &[(&str, &str)] = &[
  (
    "QRP",
    "低功率操作，一般指 CW/SSB 输出不超过 5W（部分国家定为 10W）。",
  ),
  ("QRPp", "极小功率操作，通常 1W 以下，挑战传播极限。"),
  (
    "QRP 精神",
    "以最小功率完成最远通联，重技巧与传播而非堆功率。",
  ),
  (
    "弱信号模式",
    "FT8 / WSPR / JS8 等弱信号模式让 QRP 也能完成远距离通联。",
  ),
  (
    "QRP 聚集频率",
    "各波段低端为 QRP 聚集区，如 40m 7.030、20m 14.060。",
  ),
];

/// 设备建议。
pub const QRP_RIGS: &[(&str, &str)] = &[
  (
    "QRP 收发信机",
    "Yaesu FT-818、ICOM IC-705、Xiegu G90 等内置电池的便携机。",
  ),
  ("DIY 套件", "QRP Labs QCX 等低成本 CW 套件，焊接入门首选。"),
  (
    "天线",
    "高效谐振天线比功率更重要：全尺寸偶极、垂直或便携线天线。",
  ),
  ("电源", "磷酸铁锂 / 聚合物电池，轻便且放电特性好。"),
];

/// 操作要点。
pub const QRP_TIPS: &[&str] = &[
  "QRP 靠天吃饭，选对传播时间与波段比盲目发射更重要。",
  "优先 CW 与弱信号数字模式，SSB 小功率很难被抄收。",
  "主动报告 QRP 功率，许多电台会特别关照小功率信号。",
  "参加 QRP 竞赛与奖状（如 QRP DXCC、QRP ARCI），累积成就感。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn qrp_data_populated() {
    assert!(QRP_CONCEPTS.len() >= 4);
    assert!(QRP_RIGS.len() >= 3);
    assert!(!QRP_TIPS.is_empty());
  }
}
