//! 天线 DIY：几种经典自制天线的材料与尺寸估算。

/// (天线, 材料, 尺寸估算, 说明, 可加载的 `.nec` 模板 id)
///
/// 最后一列指向 [`crate::nec_templates`] 里的模型；空串表示暂无可加载的模型。
/// 这些尺寸与模板里的几何**出自同一组公式**（`143/f`、`71.5/f`），
/// 有单测同时钉住两边，不会出现「文章说 10.14 m、加载出来是别的长度」。
pub const DIY_ANTENNAS: &[(&str, &str, &str, &str, &str)] = &[
  (
    "半波偶极",
    "铜导线 / 振子管",
    "总长 143/f(MHz) 米，中点馈电",
    "最易制作的基础天线，架设水平或倒 V。",
    "dipole-14mhz",
  ),
  (
    "四分之一波长 GP",
    "金属杆 + 地网",
    "振子长 71.5/f(MHz) 米（与 143/f 同源），配 3–4 根地网",
    "VHF/UHF 基地台全向天线。",
    "vertical-gp-2m",
  ),
  (
    "J 型天线",
    "铜管或导线",
    "总长约 3λ/4，短臂 1λ/4",
    "无需地网，适合 2m/70cm。",
    "j-pole-2m",
  ),
  (
    "2m 八木",
    "铝管",
    "反射器约 0.52λ（须比半波长长 3%–5%）、振子约 0.47λ、引向器约 0.44λ",
    "定向高增益，卫星与远距离通联。",
    "yagi3-2m",
  ),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn antenna_diy_data_populated() {
    assert!(DIY_ANTENNAS.len() >= 3);
    for (name, material, size, desc, template) in DIY_ANTENNAS {
      assert!(!name.is_empty());
      assert!(!material.is_empty());
      assert!(!size.is_empty());
      assert!(!desc.is_empty());
      // 模板 id 要么为空，要么指向真实存在的模型。
      if !template.is_empty() {
        assert!(
          crate::nec_templates::nec_template(template).is_some(),
          "{name} 指向了不存在的模板 {template}"
        );
      }
    }
  }
}
