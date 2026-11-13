//! 磁环（环形磁芯）绕线计算：由目标电感反推匝数，或由匝数算电感，并给出绕线所需的铜线长度、
//! 窗口占用与自谐振的粗略提示。
//!
//! 基本公式（`AL` 的单位 nH / 匝²）：
//!
//! ```text
//! L(µH)  = AL · N² / 1000
//! N      = √( 1000 · L(µH) / AL )          （向上取整到整匝）
//! ```
//!
//! **数据边界（先读这个）**：[`CORES`] 里只收录**已逐个对着厂商官方页面核对过**的磁环，
//! 两条来源：Fair-Rite 官方零件页（料号是 10 位料号，AL 在 10 kHz 下测得）与 Micrometals 官方
//! [product finder](https://www.micrometals.com/products/product-finder/)（料号是 T-xx-材料 的
//! 通用型号，同一张表给出 AL / Ae / Ie / 尺寸）。每一条都带料号与核对日期。
//!
//! 厂商对这些数值**没有给出数据使用条款**（两家的条款只覆盖购买与退货，不含目录数据授权），
//! AL、尺寸、Ae、Ie 是事实性规格数据，本项目按「标注来源 + 料号 + 核对日期」的方式引用，
//! 不宣称获得授权，也不复制厂商的曲线图与说明书正文。
//!
//! 粉芯（Micrometals 的 -2 / -6）的 AL 是**分布气隙**的结果，不能用 `µ0·µi·Ae/le` 反推 ——
//! 那条自洽检查只对铁氧体成立（见 [`catalogue_geometry_is_consistent`] 的分支）。
//! 页面也提供「自己输入 AL」的入口，用来覆盖没收录的磁环。
//!
//! 精度边界：
//! - AL 有出厂公差（铁氧体 ±20% / ±25%，Micrometals 的 -2 是 ±5%、-6 是 ±10%），且随频率、
//!   磁通与温度变化（铁氧体的 µ 随频率下降）；这里算出的是**低频（10 kHz 口径）的标称值**，
//!   不是工作频率下的有效电感；
//! - 不考虑绕线之间的分布电容、引线电感与自谐振，只给一个几何上的窗口占用率提醒；
//! - 共模扼流圈的阻抗取决于材料在工作频段的复磁导率，不等于这里的低频电感，本模块不替它下结论；
//!   要看 HF 各波段的阻抗与插入损耗，用 [`crate::common_mode_choke`]（那里只有带官方复磁导率表的
//!   铁氧体材料；Micrometals 的粉芯不进那张表）。

use crate::math::positive;

/// 一个磁环的厂商数据。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Core {
  /// 厂商零件号。
  pub part: &'static str,
  /// 材料牌号（Fair-Rite 的 43 / 61 等；Micrometals 铁粉芯记成 `-2` / `-6`，见 [`Core::is_powder`]）。
  pub material: &'static str,
  /// 外径 / 内径 / 高度（mm，标称值）。
  pub od_mm: f64,
  pub id_mm: f64,
  pub height_mm: f64,
  /// 电感系数（nH / 匝²，10 kHz 下测得）。
  pub al_nh: f64,
  /// AL 的出厂公差（±%）。
  pub tolerance_pct: f64,
  /// 材料的初始磁导率 µi。
  pub mu_i: f64,
  /// 平均磁路长度 le（cm）与有效截面积 Ae（cm²），厂商页面给出。
  pub le_cm: f64,
  pub ae_cm2: f64,
  /// 数据核对日期（对照厂商官方零件页）。
  pub verified: &'static str,
}

impl Core {
  /// 是不是 Micrometals 的铁粉芯（材料记号以 `-` 开头）。
  ///
  /// 粉芯与铁氧体在两处要分开对待：AL 是分布气隙的结果（不能用 `µ0·µi·Ae/le` 反推），
  /// 也没有复磁导率表（不进共模扼流圈）。判别只放在这一处，别在别处再写一遍 `starts_with('-')`。
  #[must_use]
  pub fn is_powder(&self) -> bool {
    self.material.starts_with('-')
  }
}

/// 已核对的磁环（见模块文档的数据边界）。
pub const CORES: &[Core] = &[
  Core {
    part: "5943000301",
    material: "43",
    od_mm: 12.7,
    id_mm: 7.15,
    height_mm: 4.9,
    al_nh: 440.0,
    tolerance_pct: 20.0,
    mu_i: 800.0,
    le_cm: 2.95,
    ae_cm2: 0.129,
    verified: "2026-10-14",
  },
  Core {
    part: "5943001101",
    material: "43",
    od_mm: 12.7,
    id_mm: 7.9,
    height_mm: 6.35,
    al_nh: 480.0,
    tolerance_pct: 20.0,
    mu_i: 800.0,
    le_cm: 3.12,
    ae_cm2: 0.15,
    verified: "2026-10-14",
  },
  Core {
    part: "5943003801",
    material: "43",
    od_mm: 61.0,
    id_mm: 35.55,
    height_mm: 12.7,
    al_nh: 1075.0,
    tolerance_pct: 20.0,
    mu_i: 800.0,
    le_cm: 14.5,
    ae_cm2: 1.58,
    verified: "2026-10-14",
  },
  Core {
    part: "5961003801",
    material: "61",
    od_mm: 61.0,
    id_mm: 35.55,
    height_mm: 12.7,
    al_nh: 170.0,
    tolerance_pct: 25.0,
    mu_i: 125.0,
    le_cm: 14.5,
    ae_cm2: 1.58,
    verified: "2026-10-14",
  },
  // ── Micrometals 铁粉芯（product finder，2026-11-12 核对）─────────────────────────
  // 公差：-2 为 AL +5%/−5%，-6 为 +10%/−10%（RF 目录第 4 页的 MAGNETIC TOLERANCE 表）；
  // 这里统一记成 ±值。Ae / Ie / 尺寸与 AL 同表给出（英寸已折算成 mm）。
  Core {
    part: "T50-2",
    material: "-2",
    od_mm: 12.7,
    id_mm: 7.7,
    height_mm: 4.83,
    al_nh: 4.9,
    tolerance_pct: 5.0,
    mu_i: 10.0,
    le_cm: 3.19,
    ae_cm2: 0.112,
    verified: "2026-11-12",
  },
  Core {
    part: "T68-2",
    material: "-2",
    od_mm: 17.5,
    id_mm: 9.4,
    height_mm: 4.83,
    al_nh: 5.7,
    tolerance_pct: 5.0,
    mu_i: 10.0,
    le_cm: 4.23,
    ae_cm2: 0.179,
    verified: "2026-11-12",
  },
  Core {
    part: "T106-2",
    material: "-2",
    od_mm: 26.9,
    id_mm: 14.5,
    height_mm: 11.1,
    al_nh: 13.5,
    tolerance_pct: 5.0,
    mu_i: 10.0,
    le_cm: 6.49,
    ae_cm2: 0.659,
    verified: "2026-11-12",
  },
  Core {
    part: "T130-2",
    material: "-2",
    od_mm: 33.0,
    id_mm: 19.8,
    height_mm: 11.1,
    al_nh: 11.0,
    tolerance_pct: 5.0,
    mu_i: 10.0,
    le_cm: 8.28,
    ae_cm2: 0.698,
    verified: "2026-11-12",
  },
  Core {
    part: "T200-2",
    material: "-2",
    od_mm: 50.8,
    id_mm: 31.8,
    height_mm: 14.0,
    al_nh: 12.0,
    tolerance_pct: 5.0,
    mu_i: 10.0,
    le_cm: 13.0,
    ae_cm2: 1.27,
    verified: "2026-11-12",
  },
  Core {
    part: "T50-6",
    material: "-6",
    od_mm: 12.7,
    id_mm: 7.7,
    height_mm: 4.83,
    al_nh: 4.0,
    tolerance_pct: 10.0,
    mu_i: 8.5,
    le_cm: 3.19,
    ae_cm2: 0.112,
    verified: "2026-11-12",
  },
  Core {
    part: "T68-6",
    material: "-6",
    od_mm: 17.5,
    id_mm: 9.4,
    height_mm: 4.83,
    al_nh: 4.7,
    tolerance_pct: 10.0,
    mu_i: 8.5,
    le_cm: 4.23,
    ae_cm2: 0.179,
    verified: "2026-11-12",
  },
  Core {
    part: "T80-6",
    material: "-6",
    od_mm: 20.2,
    id_mm: 12.6,
    height_mm: 6.35,
    al_nh: 4.5,
    tolerance_pct: 10.0,
    mu_i: 8.5,
    le_cm: 5.14,
    ae_cm2: 0.231,
    verified: "2026-11-12",
  },
  Core {
    part: "T94-6",
    material: "-6",
    od_mm: 23.9,
    id_mm: 14.2,
    height_mm: 7.92,
    al_nh: 7.0,
    tolerance_pct: 10.0,
    mu_i: 8.5,
    le_cm: 5.97,
    ae_cm2: 0.362,
    verified: "2026-11-12",
  },
  Core {
    part: "T106-6",
    material: "-6",
    od_mm: 26.9,
    id_mm: 14.5,
    height_mm: 11.1,
    al_nh: 11.6,
    tolerance_pct: 10.0,
    mu_i: 8.5,
    le_cm: 6.49,
    ae_cm2: 0.659,
    verified: "2026-11-12",
  },
  Core {
    part: "T157-6",
    material: "-6",
    od_mm: 39.9,
    id_mm: 24.1,
    height_mm: 14.5,
    al_nh: 11.5,
    tolerance_pct: 10.0,
    mu_i: 8.5,
    le_cm: 10.1,
    ae_cm2: 1.06,
    verified: "2026-11-12",
  },
  Core {
    part: "T200-6",
    material: "-6",
    od_mm: 50.8,
    id_mm: 31.8,
    height_mm: 14.0,
    al_nh: 10.4,
    tolerance_pct: 10.0,
    mu_i: 8.5,
    le_cm: 13.0,
    ae_cm2: 1.27,
    verified: "2026-11-12",
  },
];

/// Micrometals 铁粉芯数据的核对日期，[`crate::staleness`] 用它判过期。
pub const POWDER_CHECKED_ON: &str = "2026-11-12";

/// 按料号查一个已收录的磁环。
#[must_use]
pub fn core_by_part(part: &str) -> Option<&'static Core> {
  CORES.iter().find(|c| c.part == part.trim())
}

/// 全部磁环里**最早**的一次核对日期。
///
/// 过期检查按「最旧的那条」算：只要有一条没跟着复核，这片数据就不该被算作「刚核对过」。
#[must_use]
pub fn oldest_verified() -> Option<&'static str> {
  CORES.iter().map(|c| c.verified).min()
}

/// 匝数 → 电感（µH）。`al_nh` 与 `turns` 必须为正的有限数。
#[must_use]
pub fn inductance_uh(al_nh: f64, turns: f64) -> Option<f64> {
  (positive(al_nh) && positive(turns)).then(|| al_nh * turns * turns / 1000.0)
}

/// 目标电感（µH）→ 所需匝数（实数，未取整）。
#[must_use]
pub fn turns_for(al_nh: f64, target_uh: f64) -> Option<f64> {
  (positive(al_nh) && positive(target_uh)).then(|| (1000.0 * target_uh / al_nh).sqrt())
}

/// 一圈导线要走多长（mm）：绕过磁环的截面周长，即 `2·(外径 − 内径)/2 + 2·高度` 的近似
/// `(OD − ID) + 2·H`。涂层与线径不计，所以是**偏短的下限**。
#[must_use]
pub fn length_per_turn_mm(core: &Core) -> f64 {
  (core.od_mm - core.id_mm) + 2.0 * core.height_mm
}

/// 内圈能放下的匝数的粗略上限：内圈周长除以线的外径（单层、紧密排列）。
///
/// 只是个几何提醒，不是工艺判断：实际还受绕线张力、线的刚性、是否多层、是否要留出引脚影响。
#[must_use]
pub fn max_turns_single_layer(core: &Core, wire_od_mm: f64) -> Option<u32> {
  if !positive(wire_od_mm) {
    return None;
  }
  let inner_circumference = std::f64::consts::PI * core.id_mm;
  let n = (inner_circumference / wire_od_mm).floor();
  (n.is_finite() && n >= 0.0).then_some(n as u32)
}

/// 计算结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Winding {
  /// 实数匝数（未取整）。
  pub turns_exact: f64,
  /// 取整后的匝数：向上取整，保证电感 **不低于** 目标。
  pub turns: u32,
  /// 按取整匝数算出的电感（µH，标称 AL）。
  pub actual_uh: f64,
  /// AL 公差带下的电感范围（µH）。
  pub low_uh: f64,
  pub high_uh: f64,
  /// 所需导线长度（m，下限，见 [`length_per_turn_mm`]）。没有磁环尺寸（自定义 AL）时算不出，为 `None`。
  pub wire_m: Option<f64>,
  /// 单层放不下的话为 `Some(上限匝数)`；放得下或未给线径时为 `None`。
  pub exceeds_single_layer: Option<u32>,
}

/// 由目标电感算匝数与绕线信息。
///
/// `length_per_turn_mm` 为 `None`（没有磁环尺寸）时不算导线长度；`inner_circumference_turns` 为 `None`
/// 时不做窗口占用判断。
#[must_use]
pub fn winding_for(
  al_nh: f64,
  tolerance_pct: f64,
  target_uh: f64,
  length_per_turn_mm: Option<f64>,
  inner_circumference_turns: Option<u32>,
) -> Option<Winding> {
  if !(tolerance_pct.is_finite() && (0.0..100.0).contains(&tolerance_pct)) {
    return None;
  }
  // 给了每圈长度就必须是正数；没给（自定义 AL，没有尺寸）只是少一项结果，不是错误输入。
  if length_per_turn_mm.is_some_and(|l| !positive(l)) {
    return None;
  }
  let exact = turns_for(al_nh, target_uh)?;
  // 至少 1 匝：目标电感比 1 匝还小时，磁环绕 1 匝也已经超出，这里如实给 1。
  let turns = exact.ceil().max(1.0);
  // 匝数是整数且很小（实际 < 几千），超出 u32 说明输入荒谬（AL 极小而电感极大）。
  if turns > f64::from(u32::MAX) {
    return None;
  }
  let actual = inductance_uh(al_nh, turns)?;
  let tol = tolerance_pct / 100.0;
  let turns_u32 = turns as u32;
  Some(Winding {
    turns_exact: exact,
    turns: turns_u32,
    actual_uh: actual,
    low_uh: actual * (1.0 - tol),
    high_uh: actual * (1.0 + tol),
    wire_m: length_per_turn_mm.map(|l| turns * l / 1000.0),
    exceeds_single_layer: inner_circumference_turns.filter(|&max| turns_u32 > max),
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
  }

  #[test]
  fn inductance_follows_the_n_squared_law() {
    // 440 nH/匝² × 10² = 44 µH；匝数翻倍电感变四倍。
    assert!(near(inductance_uh(440.0, 10.0).unwrap(), 44.0, 1e-9));
    assert!(near(
      inductance_uh(440.0, 20.0).unwrap() / inductance_uh(440.0, 10.0).unwrap(),
      4.0,
      1e-9
    ));
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
      assert_eq!(inductance_uh(bad, 10.0), None, "{bad}");
      assert_eq!(inductance_uh(440.0, bad), None, "{bad}");
    }
  }

  #[test]
  fn turns_and_inductance_are_inverse() {
    for (al, n) in [(440.0, 7.0), (1075.0, 12.0), (170.0, 33.0)] {
      let l = inductance_uh(al, n).unwrap();
      assert!(near(turns_for(al, l).unwrap(), n, 1e-9), "al {al} n {n}");
    }
    assert_eq!(turns_for(440.0, 0.0), None);
    assert_eq!(turns_for(0.0, 10.0), None);
  }

  #[test]
  fn al_is_proportional_to_permeability_for_the_same_geometry() {
    // 5943003801 与 5961003801 几何完全相同（同一个 Σl/A）：AL 之比应等于 µi 之比（公差内）。
    // 这是对录入数据的内部一致性检查 —— 抄错任何一个数，这里都会红。
    let a = core_by_part("5943003801").unwrap();
    let b = core_by_part("5961003801").unwrap();
    assert_eq!(
      (a.od_mm, a.id_mm, a.height_mm),
      (b.od_mm, b.id_mm, b.height_mm)
    );
    assert_eq!((a.le_cm, a.ae_cm2), (b.le_cm, b.ae_cm2));
    let al_ratio = a.al_nh / b.al_nh; // 1075 / 170 ≈ 6.32
    let mu_ratio = a.mu_i / b.mu_i; // 800 / 125 = 6.4
    assert!(
      (al_ratio - mu_ratio).abs() / mu_ratio < 0.02,
      "{al_ratio} vs {mu_ratio}"
    );
  }

  /// 铁氧体的 AL 与 `µ0·µi·Ae/le` 在公差内一致（同时校验了 Ae / le / µi 三个录入值）。
  ///
  /// 粉芯（Micrometals 的 `-2` / `-6`）**不适用**这条：它们靠分布气隙把有效磁导率压下来，
  /// `µi` 是材料本身的初始磁导率（10 / 8.5），理论 AL 会比标称值大一个数量级。
  #[test]
  fn ferrite_al_matches_the_geometry_formula_within_tolerance() {
    const MU0: f64 = 4.0e-7 * std::f64::consts::PI;
    for c in CORES.iter().filter(|c| !c.is_powder()) {
      let theory_nh = MU0 * c.mu_i * (c.ae_cm2 * 1e-4) / (c.le_cm * 1e-2) * 1e9;
      let err = (theory_nh - c.al_nh).abs() / c.al_nh;
      // 理论值用的是 µi（B<10G 的初始值）；厂商 AL 是实测，且公差 ±20–25%，再放宽到 1.15 倍公差。
      assert!(
        err < c.tolerance_pct / 100.0 * 1.15,
        "{}: 理论 {theory_nh:.0} vs 标称 {}",
        c.part,
        c.al_nh
      );
    }
  }

  /// 粉芯的 Ae / Ie 要与尺寸对得上（矩形截面环的两个几何恒等式）。
  ///
  /// AL 是厂商实测值，且**随尺寸变化**（大环的分布气隙更多、有效磁导率更低），拿 `AL ∝ Ae/le`
  /// 去反推材料常数只会得到 ±10% 的散布 —— 那条路查不出抄错，反而会误伤。真正能查的是几何：
  /// 矩形截面的环形磁芯 `Ae ≈ H·(OD−ID)/2`、`Ie ≈ π·(OD+ID)/2`。两个恒等式同时成立，
  /// 说明 OD / ID / H / Ae / Ie 这五个数抄的是同一行。
  #[test]
  fn powder_core_geometry_is_self_consistent() {
    for c in CORES.iter().filter(|c| c.is_powder()) {
      let ae = c.height_mm * (c.od_mm - c.id_mm) / 2.0 / 100.0; // mm² → cm²
      let ie = std::f64::consts::PI * (c.od_mm + c.id_mm) / 2.0 / 10.0; // mm → cm
      // 厂商给的 Ae / Ie 只到三位有效数字；实际磁路还比几何均值短一点，取 10%。
      assert!(
        (ae - c.ae_cm2).abs() / c.ae_cm2 < 0.10,
        "{}: 几何 Ae {ae:.3} cm² vs 表里 {:.3}",
        c.part,
        c.ae_cm2
      );
      assert!(
        (ie - c.le_cm).abs() / c.le_cm < 0.05,
        "{}: 几何 Ie {ie:.2} cm vs 表里 {:.2}",
        c.part,
        c.le_cm
      );
      // 粉芯的有效磁导率远低于材料本体的 µi（分布气隙），所以 AL 只做量级检查。
      assert!(
        (0.5..40.0).contains(&c.al_nh),
        "{}: AL {} nH 不像粉芯",
        c.part,
        c.al_nh
      );
    }
  }

  /// 尺寸与 Micrometals 表的英寸值对得上（英寸 × 25.4 = mm，表里只给到三位小数）。
  #[test]
  fn powder_core_dimensions_come_from_the_inch_table() {
    let cases = [
      ("T50-2", 0.5, 0.303, 0.19),
      ("T68-2", 0.69, 0.37, 0.19),
      ("T106-2", 1.06, 0.57, 0.437),
      ("T130-2", 1.3, 0.78, 0.437),
      ("T200-2", 2.0, 1.25, 0.55),
      ("T80-6", 0.795, 0.495, 0.25),
      ("T94-6", 0.942, 0.56, 0.312),
      ("T157-6", 1.57, 0.95, 0.57),
    ];
    for (part, od, id, h) in cases {
      let c = core_by_part(part).unwrap_or_else(|| panic!("{part} 应收录"));
      for (got, inch, what) in [
        (c.od_mm, od, "外径"),
        (c.id_mm, id, "内径"),
        (c.height_mm, h, "高度"),
      ] {
        assert!(
          (got - inch * 25.4).abs() < 0.06,
          "{part} {what}: {got} mm vs {inch} in（{:.1} mm）",
          inch * 25.4
        );
      }
    }
  }

  #[test]
  fn catalogue_is_sane_and_unique() {
    let mut parts: Vec<&str> = CORES.iter().map(|c| c.part).collect();
    parts.sort_unstable();
    parts.dedup();
    assert_eq!(parts.len(), CORES.len(), "料号重复");
    for c in CORES {
      // Fair-Rite 是 10 位数字料号（第 3–4 位是材料），Micrometals 是 T-xx-材料 的型号。
      if c.is_powder() {
        assert!(
          c.part.starts_with('T') && c.part.ends_with(c.material),
          "{} 不像 Micrometals 型号",
          c.part
        );
      } else {
        assert_eq!(c.part.len(), 10, "{}", c.part);
      }
      assert!(
        c.od_mm > c.id_mm && c.id_mm > 0.0 && c.height_mm > 0.0,
        "{}",
        c.part
      );
      assert!(
        c.al_nh > 0.0 && c.tolerance_pct > 0.0 && c.mu_i > 0.0,
        "{}",
        c.part
      );
      // Fair-Rite 料号第 3–4 位是材料牌号。
      if !c.is_powder() {
        assert_eq!(
          &c.part[2..4],
          c.material,
          "{} 的材料位与 material 不符",
          c.part
        );
      }
    }
    assert!(core_by_part(" 5943003801 ").is_some());
    assert!(core_by_part("T200-2").is_some_and(Core::is_powder));
    assert!(core_by_part("5943003801").is_some_and(|c| !c.is_powder()));
    // Amidon 的型号是同一批磁芯的分销型号，本站不按它收录（避免两套编号各错一半）。
    assert!(core_by_part("FT-240-43").is_none());
    assert!(core_by_part("-2").is_none());
  }

  #[test]
  fn winding_rounds_up_and_never_undershoots() {
    // 目标 50 µH、AL 440：精确匝数 = 1000·√(50/440) = 10.66 → 11 匝，实际 53.24 µH ≥ 50。
    let w = winding_for(440.0, 20.0, 50.0, Some(10.0), None).unwrap();
    assert!(near(w.turns_exact, 10.660, 1e-3));
    assert_eq!(w.turns, 11);
    assert!(near(w.actual_uh, 53.24, 1e-9));
    assert!(w.actual_uh >= 50.0);
    // 少一匝就不够，说明取整方向对。
    assert!(inductance_uh(440.0, 10.0).unwrap() < 50.0);
    // 公差带：±20%。
    assert!(near(w.low_uh, 53.24 * 0.8, 1e-9) && near(w.high_uh, 53.24 * 1.2, 1e-9));
    // 导线长度：11 匝 × 每圈 10 mm = 0.110 m。
    assert!(near(w.wire_m.unwrap(), 0.110, 1e-9));
  }

  #[test]
  fn exact_integer_turns_are_not_bumped_up() {
    // 440 nH × 10² = 44 µH 正好是整数匝；浮点误差不能让它被进位成 11。
    let w = winding_for(440.0, 20.0, 44.0, Some(10.0), None).unwrap();
    assert_eq!(w.turns, 10);
    assert!(near(w.actual_uh, 44.0, 1e-9));
  }

  #[test]
  fn at_least_one_turn() {
    // 目标电感比 1 匝还小：如实给 1 匝，而不是 0 匝。
    let w = winding_for(1075.0, 20.0, 0.5, Some(10.0), None).unwrap();
    assert_eq!(w.turns, 1);
    assert!(w.actual_uh > 0.5);
  }

  #[test]
  fn flags_when_the_winding_does_not_fit_in_a_single_layer() {
    let c = core_by_part("5943000301").unwrap();
    // 内径 7.15 mm → 内圈周长 22.5 mm；线外径 0.5 mm 最多 44 匝。
    assert_eq!(max_turns_single_layer(c, 0.5), Some(44));
    let fit = winding_for(
      c.al_nh,
      c.tolerance_pct,
      100.0,
      Some(length_per_turn_mm(c)),
      max_turns_single_layer(c, 0.5),
    )
    .unwrap();
    assert_eq!(fit.turns, 16);
    assert_eq!(fit.exceeds_single_layer, None);
    // 目标 1000 µH 需要 48 匝 > 44：要提示放不下。
    let big = winding_for(
      c.al_nh,
      c.tolerance_pct,
      1000.0,
      Some(length_per_turn_mm(c)),
      max_turns_single_layer(c, 0.5),
    )
    .unwrap();
    assert_eq!(big.turns, 48);
    assert_eq!(big.exceeds_single_layer, Some(44));
    // 线越粗放得越少。
    assert!(max_turns_single_layer(c, 1.0).unwrap() < 44);
    assert_eq!(max_turns_single_layer(c, 0.0), None);
    assert_eq!(max_turns_single_layer(c, f64::NAN), None);
  }

  #[test]
  fn length_per_turn_is_the_section_perimeter() {
    // 5943003801：(61 − 35.55) + 2 × 12.7 = 50.85 mm。
    let c = core_by_part("5943003801").unwrap();
    assert!(near(length_per_turn_mm(c), 50.85, 1e-9));
    assert!(length_per_turn_mm(core_by_part("5943000301").unwrap()) < length_per_turn_mm(c));
  }

  /// 自定义 AL 没有磁环尺寸：匝数、电感、公差照常给，只是少了导线长度；这不是错误输入。
  #[test]
  fn a_core_without_dimensions_still_gets_turns_but_no_wire_length() {
    let w = winding_for(100.0, 20.0, 1.0, None, None).unwrap();
    assert_eq!(w.turns, 4); // √(1000·1/100) = 3.16 → 4
    assert!(near(w.actual_uh, 1.6, 1e-9));
    assert_eq!(w.wire_m, None);
    assert_eq!(w.exceeds_single_layer, None);
    // 给了就必须是正数：0 与负数仍是错误输入。
    assert_eq!(winding_for(100.0, 20.0, 1.0, Some(0.0), None), None);
    assert_eq!(winding_for(100.0, 20.0, 1.0, Some(-5.0), None), None);
  }

  #[test]
  fn rejects_nonsense() {
    assert_eq!(winding_for(0.0, 20.0, 10.0, Some(10.0), None), None);
    assert_eq!(winding_for(440.0, 20.0, 0.0, Some(10.0), None), None);
    assert_eq!(winding_for(440.0, 20.0, 10.0, Some(0.0), None), None);
    assert_eq!(winding_for(440.0, -1.0, 10.0, Some(10.0), None), None);
    assert_eq!(winding_for(440.0, 100.0, 10.0, Some(10.0), None), None);
    assert_eq!(winding_for(440.0, f64::NAN, 10.0, Some(10.0), None), None);
    // AL 极小而电感极大：匝数超出 u32，不能溢出成一个小数。
    assert_eq!(winding_for(1e-30, 20.0, 1e30, Some(10.0), None), None);
  }
}
