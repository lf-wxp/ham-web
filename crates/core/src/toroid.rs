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
//! **数据边界（先读这个）**：[`CORES`] 里只收录**已逐个对着厂商官方零件页核对过**的磁环
//! （Fair-Rite，页面标注 AL 在 10 kHz 下测得），每一条都带料号与核对日期。业余圈里广泛流传的
//! `FT-240-43`、`T-200-2`、`T-50-6` 之类按 Amidon 型号的速查值，这里**没有收录** —— Amidon 官网
//! 无法访问核对，Micrometals 页面没有 AL 表，没有可对拍的来源就不上线（见 `ROADMAP.md` 建设原则 6）。
//! 页面也提供「自己输入 AL」的入口，用来覆盖没收录的磁环。
//!
//! 精度边界：
//! - AL 有 ±20% / ±25% 的出厂公差，且随频率、磁通与温度变化（铁氧体的 µ 随频率下降）；
//!   这里算出的是**低频（10 kHz 口径）的标称值**，不是工作频率下的有效电感；
//! - 不考虑绕线之间的分布电容、引线电感与自谐振，只给一个几何上的窗口占用率提醒；
//! - 共模扼流圈的阻抗取决于材料在工作频段的复磁导率，不等于这里的低频电感，本模块不替它下结论。

use crate::math::positive;

/// 一个磁环的厂商数据。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Core {
  /// 厂商零件号。
  pub part: &'static str,
  /// 材料牌号（Fair-Rite 的 43 / 61 等）。
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
];

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

  #[test]
  fn al_matches_the_geometry_formula_within_tolerance() {
    // 理论值 AL = µ0·µi·Ae/le（SI 换算到 nH）。厂商标称 AL 在公差内应与之一致，
    // 同时校验了 Ae / le / µi 三个录入值。
    const MU0: f64 = 4.0e-7 * std::f64::consts::PI;
    for c in CORES {
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

  #[test]
  fn catalogue_is_sane_and_unique() {
    let mut parts: Vec<&str> = CORES.iter().map(|c| c.part).collect();
    parts.sort_unstable();
    parts.dedup();
    assert_eq!(parts.len(), CORES.len(), "料号重复");
    for c in CORES {
      assert_eq!(c.part.len(), 10, "{}", c.part);
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
      assert_eq!(
        &c.part[2..4],
        c.material,
        "{} 的材料位与 material 不符",
        c.part
      );
    }
    assert!(core_by_part(" 5943003801 ").is_some());
    assert!(core_by_part("FT-240-43").is_none());
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
