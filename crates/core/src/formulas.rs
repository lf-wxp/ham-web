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
        "行波系数",
        "TWR = 1 / SWR",
        "行波系数 = (1−|Γ|)/(1+|Γ|)，与驻波比互为倒数",
        "swr",
      ),
      f(
        "天线增益",
        "G = η · D",
        "增益 = 效率 × 方向性系数；分贝形式为 G(dB) = η(dB) + D(dB)",
        "gain-conversion",
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

/// 每条公式的 KaTeX 源（按公式名索引）。
///
/// 界面启用 KaTeX 时用它渲染，未启用（或没写）时回退到 `expr` 的 Unicode 排版 ——
/// 两者是同一公式的两种写法，本表只补「KaTeX 能排得更好看」的那部分，全部 22 条都写全了。
pub const FORMULA_TEX: &[(&str, &str)] = &[
  ("欧姆定律", "U = I \\cdot R"),
  ("电功率", "P = U \\cdot I = I^2 R = \\frac{U^2}{R}"),
  ("电阻串联", "R = R_1 + R_2 + \\dots"),
  (
    "电阻并联",
    "\\frac{1}{R} = \\frac{1}{R_1} + \\frac{1}{R_2} + \\dots",
  ),
  (
    "正弦有效值",
    "V_{\\mathrm{rms}} = \\frac{V_{\\mathrm{peak}}}{\\sqrt{2}}",
  ),
  ("频率 ↔ 波长", "\\lambda = \\frac{300}{f}"),
  ("LC 谐振频率", "f = \\frac{1}{2\\pi\\sqrt{LC}}"),
  ("容抗", "X_C = \\frac{1}{2\\pi f C}"),
  ("感抗", "X_L = 2\\pi f L"),
  ("CW 必要带宽", "B_n = B \\times K"),
  ("半波偶极长度", "L = \\frac{143}{f}"),
  ("1/4 波长天线", "L = \\frac{71.5}{f}"),
  ("驻波比", "\\mathrm{SWR} = \\frac{1+|\\Gamma|}{1-|\\Gamma|}"),
  ("行波系数", "\\mathrm{TWR} = \\frac{1}{\\mathrm{SWR}}"),
  ("天线增益", "G = \\eta \\cdot D"),
  ("馈线损耗", "A = a \\cdot \\frac{L}{100}"),
  ("功率增益（dB）", "\\mathrm{dB} = 10 \\lg\\frac{P_1}{P_0}"),
  ("电压增益（dB）", "\\mathrm{dB} = 20 \\lg\\frac{V_1}{V_0}"),
  ("dBm 定义", "0\\ \\mathrm{dBm} = 1\\ \\mathrm{mW}"),
  ("dBm ↔ dBμV", "\\mathrm{dB\\mu V} = \\mathrm{dBm} + 107"),
  ("级联增益", "G_{\\text{总}} = G_1 + G_2 + \\dots"),
  ("天线增益换算", "\\mathrm{dBi} = \\mathrm{dBd} + 2.15"),
];

/// 按公式名取 KaTeX 源。
#[must_use]
pub fn tex_of(name: &str) -> Option<&'static str> {
  FORMULA_TEX
    .iter()
    .find(|(n, _)| *n == name)
    .map(|(_, t)| *t)
}

#[cfg(test)]
mod tests {

  #[test]
  fn every_tex_entry_matches_a_formula_and_is_ascii_math() {
    for &(name, tex) in FORMULA_TEX {
      assert!(
        FORMULA_GROUPS
          .iter()
          .any(|g| g.items.iter().any(|f| f.name == name)),
        "FORMULA_TEX 里的「{name}」在公式表里不存在"
      );
      // KaTeX 要的是 ASCII 数学源：混进 Unicode 排版符号（·、²、√、₁）会渲染失败。
      for bad in ['·', '²', '√', '₁', '₂', 'Γ', 'λ', 'μ'] {
        assert!(!tex.contains(bad), "「{name}」的 tex 里混进了 {bad}");
      }
    }
    assert_eq!(tex_of("欧姆定律"), Some("U = I \\cdot R"));
    assert!(tex_of("不存在的公式").is_none());
  }

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
