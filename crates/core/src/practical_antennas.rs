//! 实用天线专题：EFHW 端馈半波、磁环小环、接收天线与倒 V 的选型与要点。

/// 核心概念。
pub const PRACTICAL_ANTENNAS_CONCEPTS: &[(&str, &str)] = &[
  (
    "EFHW 端馈半波",
    concat!(
      "一根半波长导线，一端馈电约 2.5–3.2 kΩ，需经 49:1 或 64:1 不平衡变压器（unun）匹配到 50Ω。",
      "严格说它不是巴伦（balun）而是 unun；实际还常需馈线侧共模扼流圈与短线地网 / counterpoise，",
      "否则高频电流会沿馈线外皮回流。",
    ),
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
    concat!(
      "偶极子两臂下垂（或上扬）成夹角，减少架设空间。",
      "阻抗随夹角减小而下降：约 120° 时约 50Ω、90° 时约 42Ω，明显低于水平偶极的 73Ω —— ",
      "这正是倒 V 可以直接用 50Ω 同轴馈电的原因。方向图也由「8」字趋于近全向。",
    ),
  ),
];

/// 天线对比表：`(方案, 优势, 适用场景, 可加载的 `.nec` 模板 id)`。
///
/// 最后一列为空表示**目前没有可加载的模型** —— 小环、Beverage、K9AY 这类
/// 对外部元件（调谐电容、终端电阻、匹配变压器）的依赖很强，用当前这套
/// 「理想细线 + 反射系数地面」的模型给不出有意义的数字，宁可不给。
pub const PRACTICAL_ANTENNAS_TABLE: &[(&str, &str, &str, &str)] = &[
  (
    "EFHW 端馈半波",
    "单端馈电、架设简单、多波段",
    "便携、SOTA/POTA、空间受限",
    "efhw-40m",
  ),
  (
    "磁环小环",
    "体积极小、低噪、带宽极窄需频繁调谐",
    "阳台/室内、强干扰市区接收",
    "",
  ),
  (
    "Beverage",
    "超长行波天线、方向性强、低噪",
    "160/80m 低波段 DX 接收",
    "",
  ),
  (
    "K9AY loop",
    "小型环形、可切换方向",
    "低波段接收、空间有限",
    "",
  ),
  (
    "倒 V",
    "单支撑杆、半波偶极变体",
    "野外架设、应急通信",
    "inverted-v-14mhz",
  ),
];

/// 制作与使用要点。
pub const PRACTICAL_ANTENNAS_TIPS: &[&str] = &[
  "EFHW 需 49:1 或 64:1 不平衡变压器，长线对端应远离人体与导电物。",
  concat!(
    "磁环天线调谐电容两端电压极高：100W 时可达数千伏（kV 量级），",
    "数百伏只适用于 QRP 几瓦的场景。必须按功率留足电容耐压与间距，调谐时勿触碰。",
  ),
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

  #[test]
  fn table_models_point_at_real_templates() {
    // 表里挂的模板 id 必须真的存在；没有模型的方案用空串明确表达
    // （小环 / Beverage / K9AY 对外部元件依赖太强，当前模型给不出有意义的数字）。
    let with_model = PRACTICAL_ANTENNAS_TABLE
      .iter()
      .filter(|(_, _, _, t)| !t.is_empty())
      .count();
    assert!(with_model >= 2, "至少倒 V 与 EFHW 应当能加载模型");
    for (name, _, _, template) in PRACTICAL_ANTENNAS_TABLE {
      if !template.is_empty() {
        assert!(
          crate::nec_templates::nec_template(template).is_some(),
          "{name} 指向了不存在的模板 {template}"
        );
      }
    }
  }
}
