//! 应急通信 EmComm：组织、频率与操作要点。

/// 核心概念与组织。
pub const EMCOMM_CONCEPTS: &[(&str, &str)] = &[
  (
    "ARES",
    "Amateur Radio Emergency Service，美国业余无线电应急服务组织。",
  ),
  (
    "RACES",
    "Radio Amateur Civil Emergency Service，美国业余无线电民间应急服务。",
  ),
  (
    "EmComm",
    "Emergency Communications，泛指利用业余无线电在灾害中提供通信保障。",
  ),
  (
    "应急台站",
    "灾害现场与指挥中心之间的通信节点，通常电池供电、便携架设。",
  ),
  ("中继保障", "应急中继台提供大范围覆盖，是现场指挥的关键。"),
];

/// 应急常用频率。
pub const EMCOMM_FREQS: &[(&str, &str)] = &[
  ("145.000 MHz", "VHF 应急呼叫常用（中国）。"),
  ("435.000 MHz", "UHF 应急呼叫常用（中国）。"),
  ("7.050 MHz / 7.060 MHz", "HF 应急通信常用（中国 LSB）。"),
  ("3.760 MHz", "HF 应急通信常用（中国 LSB）。"),
];

/// 操作要点。
pub const EMCOMM_TIPS: &[&str] = &[
  "应急通信必须服从统一指挥，先听后发，避免占用与干扰。",
  "优先保障生命财产安全相关的信息传递，用语简明准确。",
  "准备独立于市电的电源：电池、发电机、太阳能。",
  "平时多参加应急演练，熟悉预案与频点。",
  "记录完整通信日志，重要信息复诵确认。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn emcomm_data_populated() {
    assert!(!EMCOMM_CONCEPTS.is_empty());
    assert!(!EMCOMM_FREQS.is_empty());
    assert!(!EMCOMM_TIPS.is_empty());
  }
}
