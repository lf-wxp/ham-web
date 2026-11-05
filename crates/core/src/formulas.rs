//! 高频公式速查：把考试必背的公式集中起来，每条尽量关联到 `/tools` 页的对应计算器锚点，
//! 供「公式速查」页（`/formulas`）展示并一键跳转代入计算；没有对应计算器的公式只展示、不跳转。

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
  ///
  /// 只在计算器真正算这个量（或其中必需的一步）时才挂；没有就是 `None`，
  /// 页面上该公式只展示、不跳转 —— 硬挂最近的计算器会让「计算」入口名不副实。
  pub tool: Option<&'static str>,
}

/// 一组公式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormulaGroup {
  pub name: &'static str,
  pub items: &'static [Formula],
}

/// 有对应计算器的公式。
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
    tool: Some(tool),
  }
}

/// 没有对应计算器的公式。
const fn n(name: &'static str, expr: &'static str, desc: &'static str) -> Formula {
  Formula {
    name,
    expr,
    desc,
    tool: None,
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
      n(
        "波形有效值",
        "Vrms = Vp/√2（正弦），Vp（方波），Vp/√3（三角波）",
        "Vp 为交流分量的峰值；含直流分量时 Vrms² = Vdc² + Vac²（Vac 为交流分量有效值）。例：0 至 100 V 的方波真值 70.7 V、三角波 57.7 V；题库有一道 0 至 100 V 方波题只计交流幅度，答 50 V",
      ),
      n(
        "波形平均值",
        "Vavg = (Vmax + Vmin) / 2",
        "取一个周期的平均，即直流分量；适用于正弦、三角波和占空比 50% 的方波，对称于 0 的波形为 0，最小值为 0 时是峰峰值的一半；全波整流后的正弦为 0.637 × Vp",
      ),
      n(
        "RC / RL 时间常数",
        "τ = R · C = L / R",
        "通电 1 个 τ 后：RC 中电容电压升到 63%、电阻电压降到 37%；RL 中电阻电压升到 63%、电感电压降到 37%",
      ),
      f(
        "理想变压器",
        "Us/Up = Ns/Np，Zp/Zs = (Np/Ns)²",
        "电压比等于匝数比，阻抗比等于匝数比的平方，电流比与匝数比相反；例：450 Ω 变 50 Ω 需 3:1，3 匝:6 匝可作 1:4 阻抗变换",
        "transformer",
      ),
      n(
        "电源效率与电流",
        "I = Pout / (η · U)，W = Pout · t / η",
        "Pout 为射频输出功率，η 为电源效率；例：η = 80%、13.8 V 时 I ≈ 0.091 × Pout（A），220 V 时 ≈ 0.0057 × Pout；发射 10 s 耗电 ≈ 3.5×10⁻⁶ × Pout（kWh）",
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
        "dBμV ↔ μV",
        "dBμV = 20·lg(μV)",
        "反算 μV = 10^(dBμV/20)；0 dBμV = 1 μV，60 dBμV = 1 mV，与阻抗无关",
        "dbm-dbuv",
      ),
      f(
        "μV ↔ dBm",
        "dBm = 20·lg(μV) − 107",
        "50Ω 阻抗下由电压换算功率电平",
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
  FormulaGroup {
    name: "链路与传播",
    items: &[
      f(
        "自由空间路径损耗",
        "L = 32.4 + 20·lg d + 20·lg f",
        "d 为距离（km），f 为频率（MHz），L 为损耗（dB）；距离或频率每增大 10 倍多损耗 20 dB，加倍约多 6 dB（常数精确值为 32.45）",
        "fspl",
      ),
      f(
        "EIRP 与 ERP",
        "EIRP = P · Gi，ERP = P · Gd",
        "Gi、Gd 为功率倍数，dBi = dBd + 2.15，所以 EIRP 比 ERP 大 2.15 dB；例：10 W、3.85 dBd = 6 dBi（×4）→ EIRP 40 W；10 W、5.15 dBi = 3 dBd（×2）→ ERP 20 W",
        "eirp",
      ),
      f(
        "S 表与功率",
        "ΔS = ΔdB / 6",
        "S 表每格 6 dB：功率变为 1/4（电压减半）少 1 格；距离加倍路径损耗多 6 dB，也约少 1 格；例：100 W → 25 W 少 1 格",
        "decibel-gain",
      ),
    ],
  },
  FormulaGroup {
    name: "接收与调制",
    items: &[
      f(
        "镜像频率",
        "fimage = fsig ± 2·fIF",
        "本振高于信号时镜频 = 信号 + 2×中频，低于时 = 信号 − 2×中频；例：145.00 MHz、中频 45.05 MHz → 235.10 或 54.90 MHz",
        "intermod",
      ),
      n(
        "FM / FSK 必要带宽",
        "Bn ≈ 2(M + D)",
        "D 为峰值频偏；M 为最高调制频率（FM 话音）或键控速率（FSK）。FSK 的 D 题库直接取频移（RTTY 的 170 Hz 是两个频点的间隔，工程上的峰值偏移只有它的一半，即 85 Hz），按题库口径算：RTTY 频移 170 Hz、50 Bd → 2×(170+50) = 440 Hz，考题取不大于 450 Hz",
      ),
      n(
        "符号速率与数据速率",
        "Rb = Rs · log₂M",
        "Rs 为符号速率（波特），M 为每个符号的状态数；例：MFSK16 为 15.625 Bd × log₂16 = 62.5 bit/s；RTTY 只有 2 个状态，波特与 bit/s 数值相等",
      ),
    ],
  },
];

/// 每条公式的 KaTeX 源（按公式名索引）。
///
/// 界面启用 KaTeX 时用它渲染，未启用（或没写）时回退到 `expr` 的 Unicode 排版 ——
/// 两者是同一公式的两种写法，本表只补「KaTeX 能排得更好看」的那部分，每条公式都应有对应项
/// （条数以 `FORMULA_GROUPS` 为准，不在注释里写死）。
///
/// 源码按原样交给 KaTeX，不经过译文词典，所以别在里面写中文标签。
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
  (
    "波形有效值",
    "V_{\\mathrm{rms}} = \\frac{V_p}{\\sqrt{2}}\\ (\\text{sine}),\\ V_p\\ (\\text{square}),\\ \\frac{V_p}{\\sqrt{3}}\\ (\\text{triangle})",
  ),
  (
    "波形平均值",
    "V_{\\mathrm{avg}} = \\frac{V_{\\max} + V_{\\min}}{2}",
  ),
  ("RC / RL 时间常数", "\\tau = RC = \\frac{L}{R}"),
  (
    "理想变压器",
    "\\frac{U_s}{U_p} = \\frac{N_s}{N_p},\\quad \\frac{Z_p}{Z_s} = \\left(\\frac{N_p}{N_s}\\right)^2",
  ),
  (
    "电源效率与电流",
    "I = \\frac{P_{\\mathrm{out}}}{\\eta U},\\quad W = \\frac{P_{\\mathrm{out}}\\,t}{\\eta}",
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
  ("dBμV ↔ μV", "\\mathrm{dB\\mu V} = 20 \\lg(\\mu\\mathrm{V})"),
  ("μV ↔ dBm", "\\mathrm{dBm} = 20 \\lg(\\mu\\mathrm{V}) - 107"),
  ("级联增益", "G_{\\text{总}} = G_1 + G_2 + \\dots"),
  ("天线增益换算", "\\mathrm{dBi} = \\mathrm{dBd} + 2.15"),
  ("自由空间路径损耗", "L = 32.4 + 20 \\lg d + 20 \\lg f"),
  (
    "EIRP 与 ERP",
    "\\mathrm{EIRP} = P \\cdot G_i,\\quad \\mathrm{ERP} = P \\cdot G_d",
  ),
  ("S 表与功率", "\\Delta S = \\frac{\\Delta\\mathrm{dB}}{6}"),
  (
    "镜像频率",
    "f_{\\mathrm{image}} = f_{\\mathrm{sig}} \\pm 2 f_{\\mathrm{IF}}",
  ),
  ("FM / FSK 必要带宽", "B_n \\approx 2(M + D)"),
  ("符号速率与数据速率", "R_b = R_s \\log_2 M"),
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
        assert!(!f.name.is_empty() && !f.expr.is_empty() && !f.desc.is_empty());
        assert!(f.tool.is_none_or(|t| !t.is_empty()));
        tools.extend(f.tool);
      }
    }
    assert!(tools.len() >= 10, "公式应覆盖至少 10 个计算器");
  }

  #[test]
  fn every_formula_has_a_tex_source_and_a_unique_name() {
    let mut seen = std::collections::HashSet::new();
    for f in FORMULA_GROUPS.iter().flat_map(|g| g.items) {
      assert!(tex_of(f.name).is_some(), "「{}」缺 KaTeX 源", f.name);
      assert!(seen.insert(f.name), "公式名重复：{}", f.name);
    }
  }

  /// 题库反查出来的缺口：名字在、挂靠对。
  #[test]
  fn exam_gap_formulas_are_present() {
    // （公式名，对应计算器锚点；没有对应计算器的是 None）
    const GAPS: &[(&str, Option<&str>)] = &[
      ("自由空间路径损耗", Some("fspl")),
      ("EIRP 与 ERP", Some("eirp")),
      ("理想变压器", Some("transformer")),
      ("RC / RL 时间常数", None),
      ("镜像频率", Some("intermod")),
      ("电源效率与电流", None),
      ("S 表与功率", Some("decibel-gain")),
      ("FM / FSK 必要带宽", None),
      ("符号速率与数据速率", None),
      ("波形有效值", None),
      ("波形平均值", None),
    ];
    for &(name, tool) in GAPS {
      let hit = FORMULA_GROUPS
        .iter()
        .flat_map(|g| g.items)
        .find(|f| f.name == name);
      let Some(f) = hit else {
        panic!("缺公式「{name}」");
      };
      assert_eq!(f.tool, tool, "「{name}」挂靠的计算器不对");
    }
  }

  /// 说明文字里写出的算例要真算得出来：改了数却忘了改说明（或反过来）会在这里红。
  #[test]
  fn documented_examples_hold_numerically() {
    let near = |a: f64, b: f64, tol: f64| (a - b).abs() < tol;
    let db_to_ratio = |db: f64| 10f64.powf(db / 10.0);

    // 自由空间路径损耗：距离 ×5 / ×10、频率 ×3，以及题库常数 32.4 与精确值 32.45 的差。
    let fspl = |c: f64, d: f64, f: f64| c + 20.0 * d.log10() + 20.0 * f.log10();
    assert!(near(
      fspl(32.4, 500.0, 145.0) - fspl(32.4, 100.0, 145.0),
      14.0,
      0.03
    ));
    assert!(near(
      fspl(32.4, 1000.0, 145.0) - fspl(32.4, 100.0, 145.0),
      20.0,
      1e-9
    ));
    assert!(near(
      fspl(32.4, 100.0, 435.0) - fspl(32.4, 100.0, 145.0),
      9.5,
      0.05
    ));
    assert!(near(
      fspl(32.4, 200.0, 145.0) - fspl(32.4, 100.0, 145.0),
      6.0,
      0.03
    ));
    assert!((fspl(32.45, 100.0, 145.0) - fspl(32.4, 100.0, 145.0)).abs() < 0.1);

    // EIRP / ERP：10 W，3.85 dBd → 40 W；5.15 dBi → 20 W。
    assert!(near(10.0 * db_to_ratio(3.85 + 2.15), 40.0, 0.5));
    assert!(near(10.0 * db_to_ratio(5.15 - 2.15), 20.0, 0.1));

    // S 表：功率 1/4（6 dB）少 1 格，100 W → 25 W。
    assert!(near(10.0 * (100.0f64 / 25.0).log10() / 6.0, 1.0, 0.01));

    // 理想变压器：450 Ω → 50 Ω 匝数比 3:1；1100:110 匝、220 V → 22 V；3 匝:6 匝阻抗 1:4。
    assert!(near((450.0f64 / 50.0).sqrt(), 3.0, 1e-12));
    assert!(near(220.0 * 110.0 / 1100.0, 22.0, 1e-12));
    assert!(near((3.0f64 / 6.0).powi(2), 0.25, 1e-12));

    // 时间常数：1 个 τ 后 63% / 37%。
    assert!(near(1.0 - (-1.0f64).exp(), 0.63, 0.005));
    assert!(near((-1.0f64).exp(), 0.37, 0.005));

    // 电源效率：η = 80%。
    assert!(near(1.0 / (0.8 * 13.8), 0.091, 0.0005));
    assert!(near(1.0 / (0.8 * 220.0), 0.0057, 0.00005));
    assert!(near(10.0 / 0.8 / 3.6e6, 3.5e-6, 5e-8));

    // 镜像频率：145.00 MHz，中频 45.05 MHz。
    assert!(near(145.0 + 2.0 * 45.05, 235.10, 1e-9));
    assert!(near(145.0 - 2.0 * 45.05, 54.90, 1e-9));

    // FSK 必要带宽与数据速率。
    assert!(near(2.0 * (170.0 + 50.0), 440.0, 1e-9));
    assert!(near(15.625 * 16f64.log2(), 62.5, 1e-9));

    // 波形：0 至 100 V 的方波 / 三角波真有效值，平均值，整流正弦平均值，峰峰值 100 V 正弦。
    assert!(near((50.0f64 * 50.0 + 50.0 * 50.0).sqrt(), 70.7, 0.05));
    assert!(near(
      (50.0f64 * 50.0 + 50.0 * 50.0 / 3.0).sqrt(),
      57.7,
      0.05
    ));
    assert!(near((100.0 + 0.0) / 2.0, 50.0, 1e-12));
    assert!(near(2.0 / std::f64::consts::PI, 0.637, 0.0005));
    assert!(near(50.0 / 2f64.sqrt(), 35.4, 0.05));
  }
}
