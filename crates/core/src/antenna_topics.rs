//! 天线补充专题：垂直天线与地网、陷阱偶极子、对数周期天线（LPDA）、dBi 与 dBd 的增益口径。
//!
//! 阅读型专题。**能算的数字都有出处**：
//! - 带「necpp」字样的数字来自 `crates/nec/reference/antenna_topics_necpp.py`（PyNEC 2.4.0，14.2 MHz、
//!   1 mm 半径细导线），与本站求解器无关；
//! - LPDA 的设计式是 Carrel（1961）的步骤，经 Cebik 整理进 ARRL 的天线书；本模块的 [`lpda_design`]
//!   把它写成函数，页面上的算例由单测对着函数核对；
//! - FCC 的地网要求来自 47 CFR §73.189(b)(4)；陷阱的定性结论来自 ARRL《QST》1996 年 11 月 Lab Notes
//!   与 ARRL Handbook 补充文件；dBi / dBd / 实现增益的定义来自 IEEE Std 145-2013。
//!
//! 没有可核对来源的说法（地网根数与效率的具体百分比、LPDA 的增益数值）一律写成定性的，不编数。

/// 垂直天线与地网。
pub const VERTICAL_CONCEPTS: &[(&str, &str)] = &[
  (
    "镜像原理",
    "¼ 波长垂直振子靠地面（或地网）充当另一半：理想大地上它等价于「振子 + 镜像」组成的半波偶极子，馈点电阻是偶极子的一半，增益多 3 dB。核对（necpp，14.2 MHz）：偶极子 72.1 Ω / 2.14 dBi，理想大地上的垂直振子 36.0 Ω / 5.15 dBi，电阻之比 0.500、增益差 +3.01 dB。",
  ),
  (
    "地网在做什么",
    "为振子底端的回流电流提供低电阻通路。地网不足时电流改走大地，大地电阻与辐射电阻串联、白白发热：效率 = 辐射电阻 ÷（辐射电阻 + 地损电阻）。地损电阻与辐射电阻同为 36 Ω 时效率只剩 50%（−3 dB）。",
  ),
  (
    "效率看馈点电阻，不看驻波比",
    "地网越差，馈点电阻越高、驻波比反而「变好」：36 Ω 的辐射电阻串上 14 Ω 地损正好 50 Ω、驻波比 1.00，效率却只有 72%。驻波比好不等于效率高；判断地网要看馈点电阻比 36 Ω 高出多少，或对比实际信号强度。",
  ),
  (
    "下垂地网与 50 Ω",
    "自由空间里 4 根 ¼ 波长地网水平伸出时，馈点电阻只有约 23 Ω；让地网向下倾斜电阻就升高：30° 约 43 Ω、45° 约 53 Ω、60° 约 60 Ω（necpp，垂直振子已调到谐振）。所以下垂约 45° 的地网可以直接接 50 Ω 同轴。这是自由空间里的结果，近地架设时数值会随大地变化。",
  ),
  (
    "贴地与埋地地网",
    "地网贴地或埋在浅土里时靠根数取胜。美国 FCC 对调幅广播的要求是：至少 90 根、至少 ¼ 波长、等间隔的埋地径向线，120 根、0.35–0.4 波长算「极好」（47 CFR §73.189(b)(4)）。业余场合到不了这个规模，但方向一致：同样的线材，一般认为分给更多根比拉长少数几根更划算。",
  ),
  (
    "抬高地网",
    "地网抬离地面、按谐振长度剪，只要少量几根就能把大地损耗压得很低；代价是要有支撑，而且每个波段各剪一组。长度按 ¼ 波长算，约 71.5 ÷ 频率（MHz）米（与「公式速查」里 ¼ 波长天线的公式同一口径），再按谐振修剪。",
  ),
  (
    "共模电流",
    "地网不足时同轴电缆的外皮会充当地网：馈线上出现共模电流，方向图变形，接收时还会把噪声引进来。在馈点加共模扼流圈，并补地网。",
  ),
  (
    "缩短垂直天线",
    "高度小于 ¼ 波长的垂直振子，辐射电阻随（高度 ÷ 波长）² 急剧下降，要用加载线圈抵消容抗。线圈放在振子中段比放在底部好，顶部加容性帽更好；加载线圈的估算见「工具」里的电短天线加载线圈。",
  ),
];

/// 垂直天线要点。
pub const VERTICAL_TIPS: &[&str] = &[
  "先量馈点电阻：把天线调到谐振后用天线分析仪读 R。接近 36 Ω 说明地网基本合格；明显更高而驻波比很漂亮，多半是地损在作怪。",
  "垂直极化的低仰角优势依赖地面：海水边与导电好的土壤最好，城市里的硬化地面最差。",
  "每个波段都要有对应长度的地网：只给 20 米配的地网，用到 40 米上等于地网不足。",
  "屋顶、车载这类没法铺地的场合，用多根抬高的谐振地网，或把天线改成端馈并在馈点加共模扼流圈。",
];

/// 天线陷阱（Trap）。
pub const TRAP_CONCEPTS: &[(&str, &str)] = &[
  (
    "并联谐振 LC",
    "陷阱是并联的电感和电容，谐振频率 f₀ = 1/(2π√(LC))。在 f₀ 处阻抗很高，相当于一个「随频率变化的绝缘子」，把外段隔开；低于 f₀ 呈感性，高于 f₀ 呈容性。",
  ),
  (
    "双波段偶极子怎么工作",
    "内段取高波段的半波；高波段上陷阱谐振，外段被隔离。低波段上陷阱呈感性，相当于给天线加载，所以内外两段的总长可以比同频的直导线短（ARRL《QST》1996 年 11 月 Lab Notes）。「工具」里的陷阱偶极子计算器按此算外段长度，并用 NEC 做自证。",
  ),
  (
    "代价",
    "线圈电阻带来损耗，带宽变窄，重量和风阻增加，造价与维护成本上升。陷阱让天线变短、省事，但不会让它变得更好（同一篇 Lab Notes 的结论）。",
  ),
  (
    "杂散电容",
    "线圈匝间电容、外壳与导线之间的电容都与陷阱并联，会把谐振点往低处拉；设计时 C 要填含杂散在内的总电容。",
  ),
  (
    "非谐振陷阱",
    "也可以让陷阱谐振在两个波段之间：低波段用它的感抗、高波段用它的容抗，各自凑出一个谐振长度（ARRL Handbook 补充文件里的两波段加载偶极子就是这个思路）。陷阱的损耗因此更小，但整副天线的带宽并不会变宽。",
  ),
  (
    "替代方案",
    "扇形偶极子：几根不同长度的导线并联在同一个馈点，每根对应一个波段；谐振的那根阻抗接近 50–75 Ω，失谐的几根阻抗高得多，合起来仍接近单根谐振导线。跳线偶极子：逐波段短接绝缘子，损耗最低、带宽不打折，但换波段要走到天线旁。再就是天调加开线馈线。",
  ),
];

/// 陷阱要点。
pub const TRAP_TIPS: &[&str] = &[
  "先调陷阱、后调天线：用天线分析仪或栅流表单独找出陷阱的并联谐振点，再装上去修外段。",
  "外段先留长，从最高波段开始往低波段调：每改一段只看那一个波段的谐振。",
  "陷阱谐振时两端电压很高：电容耐压与线圈温升要按功率选，不要只看标称电容值。",
  "NEC 建模时，单个频率上的并联 LC 就是一个纯电抗，可以换成该频率下电抗相同的串联元件（本站的陷阱计算器就是这样自证的）。",
];

/// 对数周期天线（LPDA）。
pub const LPDA_CONCEPTS: &[(&str, &str)] = &[
  (
    "几何级数",
    "振子长度与间距按同一个比例缩放：τ = L(n+1) ÷ L(n) < 1 是相邻振子的长度比，σ = d(n) ÷ (2·L(n)) 是相对间距（d 为相邻振子的间距）。所有振子的端点落在同一个楔形的两条边上，楔形半顶角 α 满足 tan α = (1 − τ) ÷ (4σ)。",
  ),
  (
    "活动区",
    "任一频率上只有长度接近半波长的那几根振子在有效辐射，其余起引向或反射作用。换频率时活动区沿天线移动，所以增益与阻抗在很宽的频带内变化不大。",
  ),
  (
    "馈电",
    "相邻振子交叉馈电（沿馈线每过一个振子交换一次接法，相邻振子相位差 180°）；波束指向最短振子所在的一端（顶点）。",
  ),
  (
    "设计带宽",
    "活动区自己占一段带宽：B_ar = 1.1 + 7.7(1 − τ)²·cot α（Carrel 1961 的拟合式）。结构（最长到最短振子）要覆盖 B_s = B × B_ar，B 是最高频率 ÷ 最低频率；振子数 N = 1 + ln(B_s) ÷ ln(1/τ)，小数部分超过约 0.3 就进位（Carrel / Cebik 的步骤）。",
  ),
  (
    "算例：14–28 MHz",
    "取 τ = 0.90、σ = 0.16，B = 2：cot α = 4σ ÷ (1 − τ) = 6.40，α = 8.9°；B_ar = 1.59，B_s = 3.19；N = 12.0，取 12 根；最长振子约为 14 MHz 的半波长 10.7 m；臂长 L = (λ_max ÷ 4)(1 − 1/B_s)·cot α = 23.5 m。σ 缩小到 0.05 时 cot α = 2.0，臂长缩到 6.4 m，N = 9.7，取 10 根。",
  ),
  (
    "τ 与 σ 的取舍",
    "τ 大：振子多、增益与阻抗更平坦；σ 大：臂长变长，增益随之升高，直到最佳 σ 为止。Cebik 给的取值范围是 0.8 ≤ τ ≤ 0.98、0.03 ≤ σ ≤ σ_opt；业余常取偏小的 σ，拿一些增益换更短的臂。",
  ),
  (
    "与八木比较",
    "LPDA 的单位长度增益低于八木，换来整个频带内增益与驻波比都比较平坦：一副按 14–28 MHz 设计的 LPDA 能覆盖 20 / 17 / 15 / 12 米四个波段，以及 10 米波段 28 MHz 以下的部分（设计上限取 29.7 MHz 才能盖满 10 米）；八木在窄带内的增益与前后比更好。",
  ),
];

/// 对数周期天线要点。
pub const LPDA_TIPS: &[&str] = &[
  "最长振子取低端频率的半波长（实际略长），最短振子要比高端频率再留余量 —— 结构带宽 B_s 多出的部分就是这份余量。",
  "相邻振子的接线接反（没有交叉）会让方向图和驻波比明显变坏；调试时先用天线分析仪扫整个频带，驻波比应平稳。",
  "本站求解器不支持交叉线 TL（见 NEC 页的说明），所以这里不提供 LPDA 的 NEC 模型。",
];

/// dBi 与 dBd：增益的口径。
pub const GAIN_CONCEPTS: &[(&str, &str)] = &[
  (
    "两个参照",
    "dBi 以各向同性辐射器（理想点源）为参照，dBd 以自由空间中的半波偶极子为参照；半波偶极子本身是 2.15 dBi，所以 dBi = dBd + 2.15（necpp：细导线偶极子最大增益 2.14 dBi）。同一副天线，dBi 的数字总比 dBd 大 2.15。",
  ),
  (
    "换算与功率口径",
    "EIRP 以点源为参照、ERP 以半波偶极子为参照：EIRP = ERP × 1.64（+2.15 dB）。例：标称 8.15 dBi 的天线，相对点源是 6.5 倍功率增益，相对半波偶极子是 6 dBd = 4.0 倍。",
  ),
  (
    "增益不是放大",
    "天线是无源器件，增益只是把能量集中到某些方向：增益 = 方向性 × 效率（IEEE Std 145：无耗天线的增益等于方向性）。8 dBi 的方向天线，在别的方向一定比点源弱。",
  ),
  (
    "地面反射增益",
    "HF 水平天线的标称增益常常「含地面反射」。同一副偶极子架在理想大地上方 0.125–1 波长（7 个高度）时，最大增益比自由空间高 4.8–6.9 dB；换成平均大地（εr = 13、σ = 5 mS/m，反射系数近似）是 3.2–5.6 dB（necpp，14.2 MHz）。厂商只写「x dBi」而不说明含不含地面时，这个数字不能与自由空间的值直接比。",
  ),
  (
    "增益与实现增益",
    "IEEE Std 145 定义的增益不含阻抗失配与极化失配；「实现增益」把阻抗失配也算进去，与接在什么系统上有关。驻波比很差的天线，增益可以很高，实现增益却很低。",
  ),
  (
    "替代法（比较法）",
    "用已知增益的参考天线（标准增益天线，或校准过的半波偶极子）与被测天线轮流接收同一个信号源，同样的距离、同样的发射功率：读数差加上参考增益就是被测增益；参考是半波偶极子时，结果直接是 dBd。",
  ),
  (
    "三天线法",
    "手里没有已知增益的天线时，用三副天线两两互测（固定距离，共三次），解三个方程得出三副天线各自的增益；收发角色可以互换。",
  ),
  (
    "远场距离",
    "测量距离要在远场：R > 2D² ÷ λ（D 为天线最大尺寸）。方向性强的天线、要得到准确数字时，距离还要更远，并要核对邻近耦合的影响。",
  ),
  (
    "误差来源",
    "阻抗失配（先测 S11、做失配修正）、馈线损耗与电缆晃动、极化不匹配（线极化对圆极化固有 3 dB）、地面反射与周围物体、读数电平的量化。口径与「测量实验室」一致：每一项都估一个范围，最后合成不确定度。",
  ),
];

/// 增益口径要点。
pub const GAIN_TIPS: &[&str] = &[
  "看增益数字先问参照：dBi 还是 dBd？含不含地面？是最大值还是水平面的值？",
  "比较两副天线先统一参照：dBd 加 2.15 再与 dBi 比。",
  "自己测对比增益时，用同高度、同距离、同极化，先后换天线做 A/B 比较，重复几次看起伏；别在近场里测。",
];

/// Carrel 步骤的设计结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LpdaDesign {
  /// 楔形半顶角（度）。
  pub half_angle_deg: f64,
  /// 活动区带宽 `B_ar`。
  pub active_bandwidth: f64,
  /// 结构带宽 `B_s = B × B_ar`。
  pub structure_bandwidth: f64,
  /// 振子数的原始值（未取整）。
  pub elements_raw: f64,
  /// 取整后的振子数：小数部分超过 0.3 就进位。
  pub elements: u32,
  /// 最长振子（低端频率的自由空间半波长，米）。
  pub longest_m: f64,
  /// 臂长（最长到最短振子的距离，米）。
  pub boom_m: f64,
}

/// 按 Carrel（1961）/ Cebik 的步骤设计 LPDA。
///
/// 页面不直接调用它：它的存在是为了让页面文字里的算例有可对拍的出处（见本文件的单测），
/// 所以别当成死代码删掉；改了算例的文字就要改单测，反之亦然。
///
/// `tau`、`sigma` 取 `0 < τ < 1`、`σ > 0`；`f_low_mhz < f_high_mhz`；任何一项越界返回 `None`。
/// 这是教学近似：`B_ar` 是对 Carrel 曲线的拟合式，最长振子取半波长（实际略长），不含终端短路线与馈线阻抗。
#[must_use]
pub fn lpda_design(tau: f64, sigma: f64, f_low_mhz: f64, f_high_mhz: f64) -> Option<LpdaDesign> {
  let ok = |x: f64| x.is_finite() && x > 0.0;
  if !(ok(tau)
    && tau < 1.0
    && ok(sigma)
    && ok(f_low_mhz)
    && ok(f_high_mhz)
    && f_high_mhz > f_low_mhz)
  {
    return None;
  }
  let cot_alpha = 4.0 * sigma / (1.0 - tau);
  let b = f_high_mhz / f_low_mhz;
  let active = 1.1 + 7.7 * (1.0 - tau).powi(2) * cot_alpha;
  let structure = b * active;
  let raw = 1.0 + structure.ln() / (1.0 / tau).ln();
  let elements = round_element_count(raw);
  let lambda_max = 299.792_458 / f_low_mhz;
  Some(LpdaDesign {
    half_angle_deg: (1.0 / cot_alpha).atan().to_degrees(),
    active_bandwidth: active,
    structure_bandwidth: structure,
    elements_raw: raw,
    elements,
    longest_m: lambda_max / 2.0,
    boom_m: lambda_max / 4.0 * (1.0 - 1.0 / structure) * cot_alpha,
  })
}

/// 振子数取整（Cebik 的规则）：小数部分超过 0.3 就进位，否则舍去。
fn round_element_count(raw: f64) -> u32 {
  (if raw.fract() > 0.3 {
    raw.ceil()
  } else {
    raw.floor()
  }) as u32
}

/// 垂直天线的地网效率：`辐射电阻 ÷ (辐射电阻 + 地损电阻)`，电阻单位欧姆。
///
/// 与 [`lpda_design`] 同理：页面不直接调用，只用来核对文字里的效率算例。
///
/// 任一项为负或非有限、或二者之和为 0 时返回 `None`。
#[must_use]
pub fn ground_efficiency(radiation_ohm: f64, ground_loss_ohm: f64) -> Option<f64> {
  let ok = radiation_ohm.is_finite() && ground_loss_ohm.is_finite();
  let total = radiation_ohm + ground_loss_ohm;
  (ok && radiation_ohm >= 0.0 && ground_loss_ohm >= 0.0 && total > 0.0)
    .then(|| radiation_ohm / total)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn all_text() -> Vec<&'static str> {
    let pairs = [
      VERTICAL_CONCEPTS,
      TRAP_CONCEPTS,
      LPDA_CONCEPTS,
      GAIN_CONCEPTS,
    ];
    let tips = [VERTICAL_TIPS, TRAP_TIPS, LPDA_TIPS, GAIN_TIPS];
    pairs
      .iter()
      .flat_map(|p| p.iter().flat_map(|&(a, b)| [a, b]))
      .chain(tips.iter().flat_map(|t| t.iter().copied()))
      .collect()
  }

  fn find(items: &'static [(&'static str, &'static str)], name: &str) -> &'static str {
    items
      .iter()
      .find(|(k, _)| *k == name)
      .unwrap_or_else(|| panic!("缺条目 {name}"))
      .1
  }

  #[test]
  fn every_section_is_populated_and_has_no_duplicate_terms() {
    for items in [
      VERTICAL_CONCEPTS,
      TRAP_CONCEPTS,
      LPDA_CONCEPTS,
      GAIN_CONCEPTS,
    ] {
      assert!(items.len() >= 6);
      let mut names: Vec<_> = items.iter().map(|p| p.0).collect();
      names.sort_unstable();
      names.dedup();
      assert_eq!(names.len(), items.len(), "术语重复");
    }
    for tips in [VERTICAL_TIPS, TRAP_TIPS, LPDA_TIPS, GAIN_TIPS] {
      assert!(tips.len() >= 3);
    }
    // 译文词典以中文原文为 key：同一句话出现两次会让词条互相覆盖。
    let mut all = all_text();
    all.sort_unstable();
    let n = all.len();
    all.dedup();
    assert_eq!(all.len(), n, "有重复的中文原文");
  }

  #[test]
  fn lpda_worked_examples_match_the_function() {
    // 页面「算例：14–28 MHz」里的每个数字都来自这里。
    let d = lpda_design(0.9, 0.16, 14.0, 28.0).unwrap();
    assert!(
      (d.half_angle_deg - 8.88).abs() < 0.01,
      "{}",
      d.half_angle_deg
    );
    assert!((d.active_bandwidth - 1.5928).abs() < 1e-9);
    assert!((d.structure_bandwidth - 3.1856).abs() < 1e-9);
    assert!(
      (d.elements_raw - 11.997).abs() < 0.001,
      "{}",
      d.elements_raw
    );
    assert_eq!(d.elements, 12);
    assert!((d.longest_m - 10.707).abs() < 0.001);
    assert!((d.boom_m - 23.507).abs() < 0.001, "{}", d.boom_m);
    let small = lpda_design(0.9, 0.05, 14.0, 28.0).unwrap();
    assert!((small.boom_m - 6.438).abs() < 0.001, "{}", small.boom_m);
    assert!((small.elements_raw - 9.727).abs() < 0.001);
    assert_eq!(small.elements, 10);
    let text = find(LPDA_CONCEPTS, "算例：14–28 MHz");
    for needle in [
      "cot α = 4σ ÷ (1 − τ) = 6.40",
      &format!("α = {:.1}°", d.half_angle_deg),
      &format!("B_ar = {:.2}", d.active_bandwidth),
      &format!("B_s = {:.2}", d.structure_bandwidth),
      &format!("N = {:.1}，取 {} 根", d.elements_raw, d.elements),
      &format!("{:.1} m", d.longest_m),
      &format!("L = (λ_max ÷ 4)(1 − 1/B_s)·cot α = {:.1} m", d.boom_m),
      "σ 缩小到 0.05 时 cot α = 2.0",
      &format!("臂长缩到 {:.1} m", small.boom_m),
      &format!("N = {:.1}，取 {} 根", small.elements_raw, small.elements),
    ] {
      assert!(text.contains(needle), "算例缺 {needle:?}：{text}");
    }
  }

  #[test]
  fn lpda_geometry_is_consistent_with_its_definitions() {
    // 按定义造一副天线：振子半长 = R_n·tan α，R_(n+1) = τ·R_n；算回来的 τ、σ 必须等于输入。
    let (tau, sigma): (f64, f64) = (0.92, 0.12);
    let tan_alpha = (1.0 - tau) / (4.0 * sigma);
    let r = |n: i32| tau.powi(n);
    let len = |n: i32| 2.0 * r(n) * tan_alpha;
    assert!((len(3) / len(2) - tau).abs() < 1e-12);
    let sigma_back = (r(2) - r(3)) / (2.0 * len(2));
    assert!((sigma_back - sigma).abs() < 1e-12);
    // Carrel 1961 的原算例（τ = 0.92、σ = 0.12）：α ≈ 9.5°。
    let d = lpda_design(tau, sigma, 100.0, 200.0).unwrap();
    assert!(
      (d.half_angle_deg - 9.46).abs() < 0.01,
      "{}",
      d.half_angle_deg
    );
  }

  #[test]
  fn lpda_rejects_out_of_range_inputs() {
    for (t, s, lo, hi) in [
      (1.0, 0.1, 14.0, 28.0),
      (0.0, 0.1, 14.0, 28.0),
      (0.9, 0.0, 14.0, 28.0),
      (0.9, 0.1, 28.0, 14.0),
      (0.9, 0.1, 0.0, 28.0),
      (f64::NAN, 0.1, 14.0, 28.0),
      (0.9, f64::INFINITY, 14.0, 28.0),
    ] {
      assert!(
        lpda_design(t, s, lo, hi).is_none(),
        "({t}, {s}, {lo}, {hi})"
      );
    }
  }

  #[test]
  fn element_count_rounds_up_only_past_point_three() {
    // 取整规则（Cebik）：小数部分 ≤ 0.3 舍去，> 0.3 进位。边界两侧各钉一个，
    // 不钉 9.3 本身：浮点里 9.3 的小数部分是 0.3000000000000007。
    for (raw, want) in [
      (9.0, 9),
      (9.29, 9),
      (9.31, 10),
      (9.727, 10),
      (11.997, 12),
      (12.0, 12),
    ] {
      assert_eq!(round_element_count(raw), want, "raw = {raw}");
    }
  }

  #[test]
  fn lpda_design_rounds_down_when_the_fraction_is_small() {
    // 页面上的两个算例都落在「进位」一侧；这两组落在「舍去」一侧（14–28 MHz）。
    let d = lpda_design(0.9, 0.12, 14.0, 28.0).unwrap();
    assert!(
      (d.elements_raw - 11.233).abs() < 0.001,
      "{}",
      d.elements_raw
    );
    assert_eq!(d.elements, 11);
    let d = lpda_design(0.88, 0.14, 14.0, 28.0).unwrap();
    assert!(
      (d.elements_raw - 10.184).abs() < 0.001,
      "{}",
      d.elements_raw
    );
    assert_eq!(d.elements, 10);
    // 进位一侧：与页面算例一致。
    assert_eq!(lpda_design(0.9, 0.05, 14.0, 28.0).unwrap().elements, 10);
  }

  #[test]
  fn ground_efficiency_matches_the_page_examples() {
    assert_eq!(ground_efficiency(36.0, 36.0), Some(0.5));
    // 36 Ω 的辐射电阻串 14 Ω 地损：50 Ω、驻波比 1.00、效率 72%。
    let e = ground_efficiency(36.0, 14.0).unwrap();
    assert!((e - 0.72).abs() < 1e-12);
    assert!(((36.0f64 + 14.0) / 50.0 - 1.0).abs() < 1e-12);
    assert_eq!(ground_efficiency(0.0, 5.0), Some(0.0));
    for (r, g) in [(-1.0, 5.0), (5.0, -1.0), (0.0, 0.0), (f64::NAN, 1.0)] {
      assert!(ground_efficiency(r, g).is_none(), "({r}, {g})");
    }
    let text = find(VERTICAL_CONCEPTS, "地网在做什么");
    assert!(text.contains("效率只剩 50%（−3 dB）"));
    assert!((10.0 * 0.5f64.log10() + 3.0103).abs() < 1e-3);
    assert!(find(VERTICAL_CONCEPTS, "效率看馈点电阻，不看驻波比").contains("效率却只有 72%"));
  }

  #[test]
  fn gain_arithmetic_in_the_text_is_right() {
    let ratio = |db: f64| 10f64.powf(db / 10.0);
    // EIRP = ERP × 1.64。
    assert!((ratio(2.15) - 1.64).abs() < 0.005);
    // 8.15 dBi：相对点源 6.5 倍，相对偶极子（−2.15 dB = 6 dBd）4.0 倍。
    assert!((ratio(8.15) - 6.5).abs() < 0.05);
    assert!((ratio(8.15 - 2.15) - 4.0).abs() < 0.05);
    let text = find(GAIN_CONCEPTS, "换算与功率口径");
    for needle in [
      "EIRP = ERP × 1.64（+2.15 dB）",
      "8.15 dBi",
      "6.5 倍",
      "6 dBd = 4.0 倍",
    ] {
      assert!(text.contains(needle), "缺 {needle:?}");
    }
    // 理想大地上垂直振子：馈点电阻是偶极子的一半，增益多 3 dB（数字来自 necpp）。
    let mirror = find(VERTICAL_CONCEPTS, "镜像原理");
    for needle in [
      "72.1 Ω / 2.14 dBi",
      "36.0 Ω / 5.15 dBi",
      "0.500",
      "+3.01 dB",
    ] {
      assert!(mirror.contains(needle), "缺 {needle:?}");
    }
  }

  #[test]
  fn necpp_numbers_in_the_text_are_the_ones_the_script_prints() {
    // 抄进文字的 necpp 数字（`antenna_topics_necpp.py` 的输出，2026-11-12）。脚本改了要同步改这里与文字。
    let droop = find(VERTICAL_CONCEPTS, "下垂地网与 50 Ω");
    for needle in ["约 23 Ω", "30° 约 43 Ω", "45° 约 53 Ω", "60° 约 60 Ω"] {
      assert!(droop.contains(needle), "缺 {needle:?}");
    }
    let reflect = find(GAIN_CONCEPTS, "地面反射增益");
    for needle in ["4.8–6.9 dB", "3.2–5.6 dB", "εr = 13、σ = 5 mS/m"] {
      assert!(reflect.contains(needle), "缺 {needle:?}");
    }
  }

  #[test]
  fn fcc_ground_requirement_is_quoted_exactly() {
    // 47 CFR §73.189(b)(4)：至少 ¼ 波长、不少于 90 根；120 根、0.35–0.4 波长算极好。
    let text = find(VERTICAL_CONCEPTS, "贴地与埋地地网");
    for needle in [
      "至少 90 根",
      "¼ 波长",
      "120 根",
      "0.35–0.4 波长",
      "47 CFR §73.189(b)(4)",
    ] {
      assert!(text.contains(needle), "缺 {needle:?}");
    }
  }
}
