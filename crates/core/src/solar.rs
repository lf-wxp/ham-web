//! 太阳活动与短波传播科普：太阳活动指数及其对 HF 传播的影响。

/// 太阳活动指数（名称、取值范围、含义）。
pub const SOLAR_INDICES: &[(&str, &str, &str)] = &[
  (
    "SFI 太阳通量",
    "10.7cm 射电流量，典型 60–300",
    "数值越高，HF 高波段传播越好。",
  ),
  (
    "SSN 太阳黑子数",
    "太阳黑子相对数",
    "黑子越多，电离层电离越强、MUF 越高。",
  ),
  (
    "A 指数",
    "地磁活动日指数，0–400",
    "数值越高，传播条件越差（磁暴）。",
  ),
  (
    "K 指数",
    "地磁活动 3 小时指数，0–9",
    "K≥5 为磁暴，HF 传播明显恶化。",
  ),
  (
    "X 射线耀斑",
    "A/B/C/M/X 五个等级",
    "M/X 级会引发短波中断（黑子活动强时）。",
  ),
];

/// 传播条件分级。
pub const CONDITIONS: &[(&str, &str)] = &[
  ("优秀", "SFI 高、A/K 低，HF 各波段普遍开放。"),
  ("良好", "SFI 中等，HF 中低波段稳定可用。"),
  ("一般", "SFI 偏低，仅较低波段可用。"),
  ("差", "磁暴或耀斑爆发，短波中断。"),
];

/// 太阳活动周期科普要点。
pub const CYCLE_NOTES: &[&str] = &[
  "太阳活动约 11 年一个周期，峰值年份 HF 高波段（10m/12m/15m）传播最佳。",
  "低谷年份则依赖较低波段（40m/80m/160m）进行远距离通信。",
  "地磁暴会使极区吸收增强、MUF 下降，同时可能带来极光散射通信机会。",
];

/// 传播条件等级。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropagationLevel {
  /// 优秀。
  Excellent,
  /// 良好。
  Good,
  /// 一般。
  Fair,
  /// 差。
  Poor,
}

impl PropagationLevel {
  /// 中文标签。
  #[must_use]
  pub const fn label(self) -> &'static str {
    match self {
      Self::Excellent => "优秀",
      Self::Good => "良好",
      Self::Fair => "一般",
      Self::Poor => "差",
    }
  }
}

/// 根据 Kp 指数、太阳黑子数与太阳通量判断传播条件。
///
/// 规则：Kp≥5 视为磁暴（差）；Kp≥4 视为地磁活跃（一般）；
/// 其余按太阳活动强弱分为优秀/良好/一般。
#[must_use]
pub fn propagation_level(kp: f64, ssn: f64, f107: f64) -> PropagationLevel {
  if kp >= 5.0 {
    PropagationLevel::Poor
  } else if kp >= 4.0 {
    PropagationLevel::Fair
  } else if f107 >= 130.0 && ssn >= 80.0 {
    PropagationLevel::Excellent
  } else if f107 >= 100.0 {
    PropagationLevel::Good
  } else {
    PropagationLevel::Fair
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn solar_data_populated() {
    assert!(!SOLAR_INDICES.is_empty());
    assert!(!CONDITIONS.is_empty());
    assert!(!CYCLE_NOTES.is_empty());
    for &(n, r, d) in SOLAR_INDICES {
      assert!(!n.is_empty() && !r.is_empty() && !d.is_empty());
    }
  }

  #[test]
  fn propagation_levels() {
    assert_eq!(propagation_level(6.0, 100.0, 150.0), PropagationLevel::Poor);
    assert_eq!(propagation_level(4.5, 100.0, 150.0), PropagationLevel::Fair);
    assert_eq!(
      propagation_level(2.0, 120.0, 140.0),
      PropagationLevel::Excellent
    );
    assert_eq!(propagation_level(2.0, 50.0, 110.0), PropagationLevel::Good);
    assert_eq!(propagation_level(2.0, 30.0, 80.0), PropagationLevel::Fair);
  }
}
