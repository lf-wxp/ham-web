//! 天线 DIY：几种经典自制天线的材料与尺寸估算。

/// (天线, 材料, 尺寸估算, 说明)
pub const DIY_ANTENNAS: &[(&str, &str, &str, &str)] = &[
  (
    "半波偶极",
    "铜导线 / 振子管",
    "总长 143/f(MHz) 米，中点馈电",
    "最易制作的基础天线，架设水平或倒 V。",
  ),
  (
    "四分之一波长 GP",
    "金属杆 + 地网",
    "振子长 71/f(MHz) 米，配 3–4 根地网",
    "VHF/UHF 基地台全向天线。",
  ),
  (
    "J 型天线",
    "铜管或导线",
    "总长约 3λ/4，短臂 1λ/4",
    "无需地网，适合 2m/70cm。",
  ),
  (
    "2m 八木",
    "铝管",
    "反射器约 0.5λ、振子约 0.47λ、引向器约 0.44λ",
    "定向高增益，卫星与远距离通联。",
  ),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn antenna_diy_data_populated() {
    assert!(DIY_ANTENNAS.len() >= 3);
    for (name, material, size, desc) in DIY_ANTENNAS {
      assert!(!name.is_empty());
      assert!(!material.is_empty());
      assert!(!size.is_empty());
      assert!(!desc.is_empty());
    }
  }
}
