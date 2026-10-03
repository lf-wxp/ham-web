//! 巴伦（Balun）与不平衡变压器（Unun）：平衡-不平衡转换、阻抗变换与共模抑制。

/// 核心概念。
pub const BALUN_CONCEPTS: &[(&str, &str)] = &[
  (
    "巴伦 Balun",
    "Balanced–Unbalanced，平衡-不平衡转换器，连接偶极等平衡天线与同轴等不平衡馈线。",
  ),
  (
    "Unun",
    "Unbalanced–Unbalanced，不平衡-不平衡变压器，仅做阻抗变换（如长线匹配）。",
  ),
  (
    "电流巴伦",
    "扼流型，用共模扼流圈强制电流平衡、抑制馈线外皮共模电流，损耗低、带宽大。",
  ),
  (
    "电压巴伦",
    "变压器型，靠绕组变压比做平衡转换与阻抗变换，结构简单但可能磁芯饱和。",
  ),
  (
    "共模扼流圈",
    "把同轴在磁环上绕多匝，让差模信号通过、抑制共模（外皮）电流，减少射频干扰。",
  ),
];

/// 阻抗比与典型用途。
pub const BALUN_RATIOS: &[(&str, &str, &str)] = &[
  (
    "1:1",
    "偶极 / 倒 V ↔ 同轴",
    "最常用：平衡-不平衡转换 + 共模抑制，不改变阻抗。",
  ),
  (
    "4:1",
    "折叠偶极 / Windom ↔ 50Ω",
    "约 200Ω 平衡天线匹配到 50Ω，兼作平衡转换。",
  ),
  (
    "9:1",
    "随机长线 / 端馈 ↔ 50Ω",
    "高阻长线匹配到 50Ω，常用于多波段长线天线。",
  ),
  (
    "49:1 / 64:1",
    "端馈半波 EFHW ↔ 50Ω",
    "端馈半波天线阻抗约 2.5–3.2 kΩ，匹配到 50Ω。",
  ),
];

/// 绕制与选型要点。
pub const BALUN_TIPS: &[&str] = &[
  "磁环优先选 43 / 31 型铁氧体（FT-240-43 等），HF 段共模抑制效果好。",
  "同轴绕环做共模扼流圈：RG-58 / RG-8X 在磁环上绕 8–12 匝，覆盖 3.5–30 MHz。",
  "大功率时用大磁环并留散热空间，避免磁芯饱和导致发热与失真。",
  "1:1 电流巴伦是天线馈电点最实用的选择，优先于电压巴伦。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn balun_data_populated() {
    assert!(BALUN_CONCEPTS.len() >= 4);
    assert_eq!(BALUN_RATIOS.len(), 4);
    assert!(!BALUN_TIPS.is_empty());
    for (ratio, use_, desc) in BALUN_RATIOS {
      assert!(!ratio.is_empty());
      assert!(!use_.is_empty());
      assert!(!desc.is_empty());
    }
  }
}
