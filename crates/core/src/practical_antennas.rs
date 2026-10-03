//! 实用天线专题：EFHW 端馈半波、磁环小环、接收天线与倒 V 的选型与要点。

/// 核心概念。
pub const PRACTICAL_ANTENNAS_CONCEPTS: &[(&str, &str)] = &[
  (
    "EFHW 端馈半波",
    "一根半波长导线，一端直接接发射机（经 49:1 或 64:1 匹配），无需巴伦与复杂馈线。",
  ),
  (
    "磁环天线 Mag Loop",
    "小直径环形 + 高压可调电容，体积小、Q 值高，适合阳台/楼顶与低噪声接收。",
  ),
  (
    "接收天线",
    "Beverage、K9AY loop 等专为低波段 DX 设计的低噪声接收天线。",
  ),
  (
    "倒 V / 正 V",
    "偶极子倒挂或正挂，减少架设空间，阻抗与方向图接近水平偶极。",
  ),
];

/// 天线对比表。
pub const PRACTICAL_ANTENNAS_TABLE: &[(&str, &str, &str)] = &[
  (
    "EFHW 端馈半波",
    "单端馈电、架设简单、多波段",
    "便携、SOTA/POTA、空间受限",
  ),
  (
    "磁环小环",
    "体积极小、低噪、带宽极窄需频繁调谐",
    "阳台/室内、强干扰市区接收",
  ),
  (
    "Beverage",
    "超长行波天线、方向性强、低噪",
    "160/80m 低波段 DX 接收",
  ),
  ("K9AY loop", "小型环形、可切换方向", "低波段接收、空间有限"),
  ("倒 V", "单支撑杆、半波偶极变体", "野外架设、应急通信"),
];

/// 制作与使用要点。
pub const PRACTICAL_ANTENNAS_TIPS: &[&str] = &[
  "EFHW 需 49:1 或 64:1 不平衡变压器，长线对端应远离人体与导电物。",
  "磁环天线电容两端电压极高（数百伏），大功率下需耐压足够高的可变电容。",
  "接收天线不承担发射，可与发射天线分离，重点优化信噪比而非驻波比。",
  "低波段（160/80m）DX 收听优先上接收天线，效果常优于发射天线加前置放大。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn practical_antennas_populated() {
    assert!(!PRACTICAL_ANTENNAS_CONCEPTS.is_empty());
    assert!(!PRACTICAL_ANTENNAS_TABLE.is_empty());
    assert!(!PRACTICAL_ANTENNAS_TIPS.is_empty());
  }
}
