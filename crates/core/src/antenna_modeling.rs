//! 天线建模软件：EZNEC、MMANA-GAL、4NEC2 等仿真工具。

/// 常用软件。
pub const MODELING_SOFTWARE: &[(&str, &str, &str)] = &[
  (
    "EZNEC",
    "免费",
    "基于矩量法（NEC 内核），界面友好，经典首选；作者 2022 年退休后已免费发布。",
  ),
  ("MMANA-GAL", "免费", "免费天线建模与优化，界面简洁。"),
  ("4NEC2", "免费", "NEC2 内核的免费建模工具，功能全面。"),
  (
    "NEC2 内核",
    "算法",
    "数值电磁学代码，计算天线的方向图与阻抗。",
  ),
];

/// 建模流程。
pub const MODELING_STEPS: &[(&str, &str)] = &[
  ("建模", "输入振子坐标、线径、材料，定义天线几何结构。"),
  ("设频段", "指定仿真频率范围，设置地面类型与架设高度。"),
  ("求解", "运行求解器，得到阻抗、驻波比与方向图。"),
  ("优化", "调整尺寸观察驻波比与增益变化，迭代到满意。"),
];

/// 使用要点。
pub const MODELING_TIPS: &[&str] = &[
  "先建模后动手，仿真能大幅减少天线调试的试错成本。",
  "仿真结果受地面与周围环境影响，实际架设后仍需微调。",
  "注意线径与分段数设置，分段过少会导致结果不准。",
  "免费工具（MMANA-GAL / 4NEC2）足以满足绝大多数业余需求。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn antenna_modeling_data_populated() {
    assert!(MODELING_SOFTWARE.len() >= 4);
    assert!(MODELING_STEPS.len() >= 4);
    assert!(!MODELING_TIPS.is_empty());
  }
}
