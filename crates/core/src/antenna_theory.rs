//! 天线理论深化：近场/远场边界、天线 Q 与带宽、噪声温度与 G/T、互耦。
//!
//! 每条公式同时带 `expr`（Unicode 排版，无 JS 时直接用）与 `tex`（KaTeX 源），
//! 界面在 KaTeX 可用时用后者渲染，不可用时回退前者 —— 两条都是同一公式的两种写法。

/// 一条理论公式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TheoryFormula {
  /// 名称。
  pub name: &'static str,
  /// Unicode 表达式（无 KaTeX 时的回退显示）。
  pub expr: &'static str,
  /// KaTeX 源（界面启用 KaTeX 时用）。
  pub tex: &'static str,
  /// 一句话说明。
  pub desc: &'static str,
}

const fn f(
  name: &'static str,
  expr: &'static str,
  tex: &'static str,
  desc: &'static str,
) -> TheoryFormula {
  TheoryFormula {
    name,
    expr,
    tex,
    desc,
  }
}

/// 近场 / 远场边界。
pub const NEAR_FAR: &[TheoryFormula] = &[
  f(
    "远场边界（夫琅禾费距离）",
    "r = 2D² / λ",
    r"r = \frac{2D^2}{\lambda}",
    "D 为天线最大尺寸，λ 为波长。超过这个距离才谈得上方向图与增益；天线越大、频率越高，远场越远。",
  ),
  f(
    "电抗近场边界",
    "r = λ / 2π",
    r"r = \frac{\lambda}{2\pi}",
    "更近的距离内储能场占主导，测量与辐射模式都失真 —— 场地测量要在这条线之外。",
  ),
  f(
    "实例 · 2m 波段八木",
    "D ≈ λ → r ≈ 2λ ≈ 4 m",
    r"D \approx \lambda \Rightarrow r \approx 2\lambda \approx 4\,\mathrm{m}",
    "2m 波段一根约 2 米的八木（D≈λ），远场只有 4 米左右；同样 D≈λ 的八木搬到 20m 波段（λ=20m，天线约 20 米长），远场要到 40m。",
  ),
];

/// 天线 Q 与带宽。
pub const Q_BANDWIDTH: &[TheoryFormula] = &[
  f(
    "Q 的定义",
    "Q = 2π · 储能 / 每周损耗",
    r"Q = \frac{2\pi W_s}{W_l}",
    "天线与匹配电路的「尖锐程度」：Q 越高，谐振越尖锐。",
  ),
  f(
    "带宽与 Q",
    "BW ≈ f₀ / Q",
    r"\mathrm{BW} \approx \frac{f_0}{Q}",
    "带宽与 Q 成反比：谐振点相同，Q 翻倍则带宽减半。电小天线（短于 λ/10）天然高 Q、窄带。",
  ),
  f(
    "实例 · 磁环小环",
    "Q ≈ 100 → BW ≈ 1%",
    r"Q \approx 100 \Rightarrow \mathrm{BW} \approx 1\%",
    "7 MHz 谐振的小环天线带宽只有几十 kHz —— 这就是小环「换频段要重调」的数学原因。",
  ),
];

/// 噪声温度与 G/T。
pub const G_T: &[TheoryFormula] = &[
  f(
    "噪声温度",
    "T = 噪声功率 / (k·B)",
    r"T = \frac{P_n}{k\,B}",
    "把噪声功率折算成等效温度：室温约 290 K，对应约 −174 dBm/Hz 的热噪声底（kTB 取 1 Hz）。",
  ),
  f(
    "系统噪声温度",
    "T系统 = T天线 + T接收机",
    r"T_\mathrm{sys} = T_\mathrm{ant} + T_\mathrm{rx}",
    "天线指向冷空时 T天线 很低（几十 K），指向地面/热源时接近室温。",
  ),
  f(
    "品质因数 G/T",
    "G/T = G(dBi) − 10·lg(T/K)",
    r"G/T = G_{\mathrm{dBi}} - 10\lg(T)",
    "弱信号（卫星 / EME）看的是增益与噪声的比：增益高但噪声大，弱信号照样出不来。",
  ),
];

/// 互耦。
pub const MUTUAL_COUPLING: &[TheoryFormula] = &[
  f(
    "互耦是什么",
    "Z₁ = Z₁₁ + Z₁₂ · I₂ / I₁",
    r"Z_1 = Z_{11} + Z_{12}\frac{I_2}{I_1}",
    "两根天线靠得太近会互相「看得到」：输入阻抗漂移、方向图畸变、谐振点移动。",
  ),
  f(
    "工程惯例",
    "间距 ≥ λ/2 起步",
    r"d \gtrsim \lambda/2",
    "多单元阵列与多天线架设的起步间距；阵列天线的单元间距正是有意利用互耦来成形。",
  ),
];

/// 全部小节：`(节标题, 公式列表)`。
pub const THEORY_SECTIONS: &[(&str, &[TheoryFormula])] = &[
  ("近场 / 远场边界", NEAR_FAR),
  ("天线 Q 与带宽", Q_BANDWIDTH),
  ("噪声温度与 G/T", G_T),
  ("互耦", MUTUAL_COUPLING),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn all_sections_are_populated_and_well_formed() {
    assert_eq!(THEORY_SECTIONS.len(), 4, "四个小节");
    for (title, formulas) in THEORY_SECTIONS {
      assert!(!title.is_empty());
      assert!(!formulas.is_empty(), "{title} 没有公式");
      for f in formulas.iter() {
        assert!(
          !f.name.is_empty() && !f.expr.is_empty() && !f.tex.is_empty(),
          "{title}"
        );
        // tex 里不能出现 Unicode 排版符号（KaTeX 要的是 ASCII 数学源）。
        for bad in ['·', '²', '√', '−'] {
          assert!(
            !f.tex.contains(bad),
            "{title}：{} 的 tex 里混进了 {bad}",
            f.name
          );
        }
      }
    }
  }

  #[test]
  fn the_key_formulas_are_what_the_exam_expects() {
    // 远场边界与带宽公式是考试口径，钉住写法。
    assert_eq!(NEAR_FAR[0].expr, "r = 2D² / λ");
    assert_eq!(Q_BANDWIDTH[1].expr, "BW ≈ f₀ / Q");
  }
}
