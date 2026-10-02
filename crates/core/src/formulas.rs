//! 高频公式速查：把考试必背的公式集中起来，每条关联到 `/tools` 页的对应计算器锚点，
//! 供「公式速查」页（`/formulas`）展示并一键跳转代入计算。

/// 一条公式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Formula {
  /// 公式名。
  pub name: &'static str,
  /// 公式表达式（用 `·`、`√`、`²` 等符号排版）。
  pub expr: &'static str,
  /// 一句话说明。
  pub desc: &'static str,
  /// `/tools` 页对应计算器的锚点 id（如 `ohms-law`）。
  pub tool: &'static str,
}

/// 一组公式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormulaGroup {
  pub name: &'static str,
  pub items: &'static [Formula],
}

const fn f(
  name: &'static str,
  expr: &'static str,
  desc: &'static str,
  tool: &'static str,
) -> Formula {
  Formula {
    name,
    expr,
    desc,
    tool,
  }
}

/// 全部公式分组（按考试大纲归类）。
pub const FORMULA_GROUPS: &[FormulaGroup] = &[
  FormulaGroup {
    name: "基础电路",
    items: &[
      f("欧姆定律", "U = I · R", "电压 = 电流 × 电阻", "ohms-law"),
      f(
        "电功率",
        "P = U · I = I²R = U²/R",
        "功率与电压、电流、电阻的换算",
        "ohms-law",
      ),
      f(
        "电阻串联",
        "R = R₁ + R₂ + …",
        "串联电阻直接相加",
        "resistor",
      ),
      f(
        "电阻并联",
        "1/R = 1/R₁ + 1/R₂ + …",
        "并联电阻取倒数之和的倒数",
        "resistor",
      ),
      f(
        "正弦有效值",
        "Vrms = Vpeak / √2",
        "有效值 = 峰值 ÷ √2，峰-峰值 = 2 × 峰值",
        "dbm-power",
      ),
    ],
  },
  FormulaGroup {
    name: "频率与谐振",
    items: &[
      f(
        "频率 ↔ 波长",
        "λ = 300 / f",
        "波长（米）= 300 ÷ 频率（MHz）",
        "freq-wavelength",
      ),
      f(
        "LC 谐振频率",
        "f = 1 / (2π√(LC))",
        "谐振时容抗与感抗相互抵消",
        "lc-resonance",
      ),
      f("容抗", "Xc = 1 / (2πfC)", "频率越高容抗越小", "reactance"),
      f("感抗", "XL = 2πfL", "频率越高感抗越大", "reactance"),
      f(
        "CW 必要带宽",
        "Bn = B × K",
        "B = WPM / 1.2，K 取 5（衰落）或 3（非衰落）",
        "cw-bandwidth",
      ),
    ],
  },
  FormulaGroup {
    name: "天线与传输线",
    items: &[
      f(
        "半波偶极长度",
        "L = 143 / f",
        "长度（米）= 143 ÷ 频率（MHz）",
        "antenna-length",
      ),
      f(
        "1/4 波长天线",
        "L = 71.5 / f",
        "长度（米）= 71.5 ÷ 频率（MHz）",
        "antenna-length",
      ),
      f(
        "驻波比",
        "SWR = (1+|Γ|)/(1-|Γ|)",
        "Γ = (ZL − Z0)/(ZL + Z0)，工程上宜 ≤ 1.5:1",
        "swr",
      ),
      f(
        "馈线损耗",
        "总损耗 = 每百米损耗 × 长度/100",
        "功率损耗 = 1 − 10^(−dB/10)",
        "feedline-loss",
      ),
    ],
  },
  FormulaGroup {
    name: "增益与功率",
    items: &[
      f(
        "功率增益（dB）",
        "dB = 10·lg(P₁/P₀)",
        "功率比的分贝表示",
        "decibel-gain",
      ),
      f(
        "电压增益（dB）",
        "dB = 20·lg(V₁/V₀)",
        "电压比的分贝表示",
        "decibel-gain",
      ),
      f("dBm 定义", "0 dBm = 1 mW", "30 dBm = 1 W", "dbm-power"),
      f(
        "dBm ↔ dBμV",
        "dBμV = dBm + 107",
        "50Ω 阻抗下的换算",
        "dbm-dbuv",
      ),
      f(
        "级联增益",
        "G 总 = G₁ + G₂ + …",
        "各级增益 dB 直接相加",
        "cascade-gain",
      ),
      f(
        "天线增益换算",
        "dBi = dBd + 2.15",
        "dBi 相对各向同性，dBd 相对半波偶极",
        "gain-conversion",
      ),
    ],
  },
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn formulas_are_populated_and_well_formed() {
    assert!(FORMULA_GROUPS.len() >= 3);
    let mut tools = std::collections::HashSet::new();
    for g in FORMULA_GROUPS {
      assert!(!g.items.is_empty());
      for f in g.items {
        assert!(
          !f.name.is_empty() && !f.expr.is_empty() && !f.desc.is_empty() && !f.tool.is_empty()
        );
        tools.insert(f.tool);
      }
    }
    assert!(tools.len() >= 10, "公式应覆盖至少 10 个计算器");
  }
}
