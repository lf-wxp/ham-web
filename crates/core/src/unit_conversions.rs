//! 单位换算速查：把考试 / 业余无线电里高频的单位换算关系集中成表，
//! 供「公式速查」页（`/formulas`）的「单位换算」区块展示并一键跳转代入计算。
//!
//! 与 [`crate::formulas::Formula`] 的区别：`Formula` 列的是「公式」，这里专列「单位换算」，
//! 每条除换算公式外还带一个数值示例。名称与示例的文案走 i18n（`exam.units-*`），
//! 换算公式是符号表达式、跨语言一致，直接内嵌。

/// 一条单位换算。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitConversion {
  /// 换算名称（i18n key，如 `exam.units-dbm-mw`）。
  pub name: &'static str,
  /// 换算公式（符号表达式，跨语言一致）。
  pub expr: &'static str,
  /// 数值示例（i18n key）。
  pub example: &'static str,
  /// `/tools` 页对应计算器的锚点 id。
  pub tool: &'static str,
}

/// 一组单位换算。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitConversionGroup {
  /// 分组名（i18n key）。
  pub name: &'static str,
  pub items: &'static [UnitConversion],
}

const fn u(
  name: &'static str,
  expr: &'static str,
  example: &'static str,
  tool: &'static str,
) -> UnitConversion {
  UnitConversion {
    name,
    expr,
    example,
    tool,
  }
}

/// 全部单位换算分组（功率 / 频率 / 分贝与电压 / 天线与传输）。
pub const UNIT_CONVERSION_GROUPS: &[UnitConversionGroup] = &[
  UnitConversionGroup {
    name: "exam.units-power",
    items: &[
      u(
        "exam.units-dbm-mw",
        "dBm = 10·lg(P/mW)",
        "exam.units-dbm-mw-example",
        "dbm-power",
      ),
      u(
        "exam.units-dbw-w",
        "dBW = 10·lg(P/W)",
        "exam.units-dbw-w-example",
        "dbm-power",
      ),
    ],
  },
  UnitConversionGroup {
    name: "exam.units-frequency",
    items: &[
      u(
        "exam.units-freq-units",
        "1 MHz = 10³ kHz = 10⁶ Hz",
        "exam.units-freq-units-example",
        "freq-units",
      ),
      u(
        "exam.units-freq-wavelength",
        "λ(m) = 300 / f(MHz)",
        "exam.units-freq-wavelength-example",
        "freq-wavelength",
      ),
    ],
  },
  UnitConversionGroup {
    name: "exam.units-decibel",
    items: &[
      u(
        "exam.units-db-power-ratio",
        "dB = 10·lg(P₁/P₀)",
        "exam.units-db-power-ratio-example",
        "decibel-gain",
      ),
      u(
        "exam.units-db-voltage-ratio",
        "dB = 20·lg(V₁/V₀)",
        "exam.units-db-voltage-ratio-example",
        "decibel-gain",
      ),
      u(
        "exam.units-dbm-dbuv",
        "dBμV = dBm + 107",
        "exam.units-dbm-dbuv-example",
        "dbm-dbuv",
      ),
    ],
  },
  UnitConversionGroup {
    name: "exam.units-antenna",
    items: &[
      u(
        "exam.units-dbi-dbd",
        "dBi = dBd + 2.15",
        "exam.units-dbi-dbd-example",
        "gain-conversion",
      ),
      u(
        "exam.units-swr-gamma",
        "SWR = (1+|Γ|)/(1-|Γ|)",
        "exam.units-swr-gamma-example",
        "swr",
      ),
      u(
        "exam.units-return-loss",
        "RL = -20·lg(|Γ|)",
        "exam.units-return-loss-example",
        "swr",
      ),
      u(
        "exam.units-antenna-length",
        "L = 143 / f(MHz)",
        "exam.units-antenna-length-example",
        "antenna-length",
      ),
    ],
  },
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn conversions_are_populated_and_well_formed() {
    assert_eq!(UNIT_CONVERSION_GROUPS.len(), 4);
    let mut tools = std::collections::HashSet::new();
    for g in UNIT_CONVERSION_GROUPS {
      assert!(!g.name.is_empty());
      assert!(!g.items.is_empty());
      for c in g.items {
        assert!(
          !c.name.is_empty() && !c.expr.is_empty() && !c.example.is_empty() && !c.tool.is_empty()
        );
        tools.insert(c.tool);
      }
    }
    assert!(tools.len() >= 6, "单位换算应覆盖至少 6 个计算器");
  }

  #[test]
  fn every_group_covers_at_least_two_conversions() {
    assert!(UNIT_CONVERSION_GROUPS.iter().all(|g| g.items.len() >= 2));
  }
}
