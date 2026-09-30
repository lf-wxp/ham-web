//! 特殊传播方式：EME、流星余迹、极光、对流层散射等。

/// (方式, 原理, 特点)
pub const SPECIAL_MODES: &[(&str, &str, &str)] = &[
  (
    "EME 月面反射",
    "利用月球反射电波，实现地球任意两地通联",
    "路径损耗极大（约 250dB+），需高增益天线阵列与大功率。",
  ),
  (
    "流星余迹散射 MS",
    "利用流星电离余迹反射 VHF 电波",
    "短暂增强，常配合高速数字模式（如 MSK144）完成交换。",
  ),
  (
    "极光传播",
    "高纬极光区电离增强反射电波",
    "信号有独特的「嘶嘶」声，用于 VHF/UHF。",
  ),
  (
    "对流层散射",
    "对流层不均匀体散射电波",
    "可实现数百公里 VHF/UHF 稳定通信。",
  ),
  (
    "波导传播",
    "逆温层形成大气波导，超视距传播",
    "UHF 可远达上千公里，多出现在海上与沿海。",
  ),
  ("卫星转发", "经业余卫星中继转发", "见「业余卫星」专题。"),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn special_prop_data_populated() {
    assert!(SPECIAL_MODES.len() >= 5);
    for (name, principle, feature) in SPECIAL_MODES {
      assert!(!name.is_empty());
      assert!(!principle.is_empty());
      assert!(!feature.is_empty());
    }
  }
}
