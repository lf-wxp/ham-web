//! 执照类别 × 频率 → 合规结论：可用性、功率上限、是否需协调、法条依据。
//!
//! 把 [`crate::license_classes`]（功率口径）、[`crate::bands`]（业余业务划分表）串起来，
//! 回答「我持 A/B/C 类操作证，在某个频率能发射吗、最多多少瓦」。数据全部在站内，不需要
//! 外部来源：功率上限取自 `license_classes` 的常量，频率划分取自 `bands::BANDS` 的
//! 业余业务条目，法条引用指向 `/radio-law` 收录的《业余无线电台管理办法》条文。
//!
//! # 规则来源
//!
//! - 类别频率权限与功率上限：办法第三十条（A 类 30–3000 MHz ≤25 W；B 类 30 MHz 以下 ≤15 W、
//!   30 MHz 以上 ≤25 W；C 类 30 MHz 以下 ≤1000 W、30 MHz 以上 ≤25 W）。
//! - 是否需协调：办法第十五条（次要业务，或与其他业务共同划分为主要业务的频率，须开展频率协调）。
//! - 次要业务不得干扰主要业务：办法第三十二条。
//! - 个别频段的 EIRP 上限由 ITU 脚注额外限制（2200 m ≤1 W、60 m ≤15 W），与类别上限取较小者。

use crate::bands::{self, Usage};
use crate::frequencies;
// 频率边界（30MHz / 3000MHz）与功率上限一样收在 `license_classes` 里：那里是权限口径的
// 唯一事实来源，两处各写一份常量迟早会与 `BAND_PERMISSIONS` 的文字漂开。
pub use crate::license_classes::{
  CLASS_A_MAX_W, CLASS_A_UPPER_MHZ, CLASS_B_HF_MAX_W, CLASS_C_HF_MAX_W, VHF_THRESHOLD_MHZ,
};

/// A / B / C 类操作技术能力。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
  A,
  B,
  C,
}

impl Class {
  /// 从单选值（`"A"` / `"B"` / `"C"`，忽略大小写）解析。
  #[must_use]
  pub fn parse(s: &str) -> Option<Class> {
    match s.trim().to_ascii_uppercase().as_str() {
      "A" => Some(Class::A),
      "B" => Some(Class::B),
      "C" => Some(Class::C),
      _ => None,
    }
  }

  /// 类别标识（`A` / `B` / `C`）。
  #[must_use]
  pub const fn label(self) -> &'static str {
    match self {
      Self::A => "A",
      Self::B => "B",
      Self::C => "C",
    }
  }
}

/// 一条法条引用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LawRef {
  /// 法律 key（[`crate::radio_law::Law::key`]，如 `"办法"`）。
  pub law: &'static str,
  /// 条号。
  pub article: u16,
}

const A30: LawRef = LawRef {
  law: "办法",
  article: 30,
};
const A15: LawRef = LawRef {
  law: "办法",
  article: 15,
};
const A32: LawRef = LawRef {
  law: "办法",
  article: 32,
};

/// 仅类别权限（专用 / 唯一主要频段）：办法第三十条。
const REFS_CLASS: &[LawRef] = &[A30];
/// 需协调（与其他业务共同主要）：办法第三十、十五条。
const REFS_COORD: &[LawRef] = &[A30, A15];
/// 次要业务：办法第三十、十五、三十二条。
const REFS_SECONDARY: &[LawRef] = &[A30, A15, A32];

/// ITU 脚注额外限制 EIRP 的业余频段（MHz 半开区间 `[下, 上)`、上限 W、脚注号）。
///
/// 与类别上限取较小者 —— 否则 C 类用户在 60 m 会被误报成可上 1000 W，而 5.133B 脚注
/// 只允许 15 W（e.i.r.p.）。这里只收「脚注明确写了功率上限」的频段，脚注原文见
/// [`crate::bands::FOOTNOTES`]。
const ITU_EIRP_CAPS: &[(f64, f64, f64, &str)] = &[
  (0.1357, 0.1378, 1.0, "5.67A"),   // 2200 m ≤1 W e.i.r.p.
  (5.3515, 5.3665, 15.0, "5.133B"), // 60 m ≤15 W e.i.r.p.
];

/// 命中的业余业务划分。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AllocationHit {
  /// 划分范围原文（如 `"7-7.1MHz"`）。
  pub range: &'static str,
  /// 使用状态（专用 / 主要 / 次要 / 唯一主要）。
  pub usage: Usage,
  /// 常用波段名（`"40m"` / `"70cm"` 等）；微波段等不在常用表里的频段为 `None`，
  /// 由界面翻成当前语言的「其他」—— 这里不能返回中文，否则 en / es 界面会露馅。
  pub common_band: Option<&'static str>,
}

/// 一次合规检查的结论。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Compliance {
  /// 命中的业余业务划分；`None` 表示频率不在任何业余业务频段内。
  pub allocation: Option<AllocationHit>,
  /// 该类别是否允许使用这个频率（频率本身可能仍在业余频段内）。
  pub class_permitted: bool,
  /// 允许发射时的**发射功率**上限（W）；不允许使用时为 `None`。
  ///
  /// 已经与 ITU 脚注的 e.i.r.p. 上限取较小者。脚注给的是 e.i.r.p.，这里按 0 dBi 折算成发射功率
  /// （与类别上限同一量纲）—— 天线有增益时实际可用的发射功率更低，界面会另行注明受哪条脚注限制。
  pub tx_power_limit_w: Option<f64>,
  /// 额外限制该频段的 ITU 脚注（上限 W、脚注号）；无脚注限制时为 `None`。
  pub eirp_cap: Option<(f64, &'static str)>,
  /// 是否需要开展频率协调（次要业务或与其他业务共同主要）。
  pub needs_coordination: bool,
  /// 关联法条。
  pub refs: &'static [LawRef],
}

/// 频率（MHz）命中的业余业务划分。
///
/// 前提：`bands::BANDS` 里的各业余划分是**互不重叠**的半开区间（相邻划分共享端点但不重叠，
/// 如 `7-7.1MHz` 与 `7.1-7.2MHz`），因此命中即唯一，按遍历顺序取首个命中即可，不存在
/// 「先宽后窄」的优先级问题。`bands` 的 `amateur_allocations_do_not_overlap` 单测守着这条
/// 前提，改了划分表一旦引入重叠会当场红。
fn find_allocation(freq_mhz: f64) -> Option<AllocationHit> {
  for band in bands::BANDS {
    for a in band.allocations {
      let Some((lo, hi)) = bands::parse_range_mhz(a.range) else {
        continue;
      };
      // 与 `frequencies::band_of` 同口径：[下, 上)。
      if lo <= freq_mhz && freq_mhz < hi {
        return Some(AllocationHit {
          range: a.range,
          usage: a.usage,
          common_band: frequencies::band_of_opt(freq_mhz),
        });
      }
    }
  }
  None
}

/// 类别是否允许使用该频率（与是否落在业余频段无关）。
fn class_permitted(class: Class, freq_mhz: f64) -> bool {
  match class {
    Class::A => (VHF_THRESHOLD_MHZ..=CLASS_A_UPPER_MHZ).contains(&freq_mhz),
    Class::B | Class::C => true,
  }
}

/// 类别在该频率的功率上限（W）。
fn class_max_power(class: Class, freq_mhz: f64) -> f64 {
  match class {
    Class::A => CLASS_A_MAX_W,
    Class::B => {
      if freq_mhz < VHF_THRESHOLD_MHZ {
        CLASS_B_HF_MAX_W
      } else {
        CLASS_A_MAX_W
      }
    }
    Class::C => {
      if freq_mhz < VHF_THRESHOLD_MHZ {
        CLASS_C_HF_MAX_W
      } else {
        CLASS_A_MAX_W
      }
    }
  }
}

/// 该频率是否被 ITU 脚注额外限制 EIRP。
fn eirp_cap(freq_mhz: f64) -> Option<(f64, &'static str)> {
  ITU_EIRP_CAPS
    .iter()
    .find(|(lo, hi, _, _)| *lo <= freq_mhz && freq_mhz < *hi)
    .map(|&(_, _, w, code)| (w, code))
}

/// 该使用状态是否需要协调（办法第十五条）。
fn needs_coordination(usage: Usage) -> bool {
  matches!(usage, Usage::Secondary | Usage::Primary)
}

/// 检查某类别在某频率（MHz）能否发射、功率上限与注意事项。
#[must_use]
pub fn check(class: Class, freq_mhz: f64) -> Compliance {
  // 非法 / 非正频率：不落在任何业余频段，直接给「不可用」。
  let Some(hit) = find_allocation(freq_mhz) else {
    return Compliance {
      allocation: None,
      class_permitted: false,
      tx_power_limit_w: None,
      eirp_cap: None,
      needs_coordination: false,
      refs: &[],
    };
  };
  let permitted = class_permitted(class, freq_mhz);
  // 变量与函数同名会让人以为这里在递归（初始值里解析到的确实是函数）。
  let eirp = permitted.then(|| eirp_cap(freq_mhz)).flatten();
  let tx_power_limit_w = permitted.then(|| {
    let base = class_max_power(class, freq_mhz);
    eirp.map_or(base, |(cap, _)| base.min(cap))
  });
  let refs = match hit.usage {
    Usage::Secondary => REFS_SECONDARY,
    Usage::Primary => REFS_COORD,
    Usage::Exclusive | Usage::SolePrimary => REFS_CLASS,
  };
  Compliance {
    allocation: Some(hit),
    class_permitted: permitted,
    tx_power_limit_w,
    eirp_cap: eirp,
    needs_coordination: needs_coordination(hit.usage),
    refs,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn at(freq_mhz: f64) -> Compliance {
    check(Class::C, freq_mhz)
  }

  #[test]
  fn class_a_is_restricted_to_30_to_3000_mhz() {
    // A 类在 20 m（14.2）：频率在业余频段内，但类别无权使用。
    let c = check(Class::A, 14.2);
    assert!(c.allocation.is_some());
    assert!(!c.class_permitted);
    assert_eq!(c.tx_power_limit_w, None);
    // A 类在 2 m：可用，25 W。
    let c = check(Class::A, 145.0);
    assert!(c.class_permitted);
    assert_eq!(c.tx_power_limit_w, Some(25.0));
    // A 类在 5.7 GHz（超过 3000 MHz）：无权使用。
    let c = check(Class::A, 5760.0);
    assert!(c.allocation.is_some());
    assert!(!c.class_permitted);
  }

  #[test]
  fn power_limits_follow_class_and_30mhz_boundary() {
    // 30 MHz 以下：B ≤15 W、C ≤1000 W；30 MHz 以上 A/B/C 一律 ≤25 W。
    assert_eq!(
      check(Class::B, 7.1).tx_power_limit_w,
      Some(CLASS_B_HF_MAX_W)
    );
    assert_eq!(
      check(Class::C, 7.1).tx_power_limit_w,
      Some(CLASS_C_HF_MAX_W)
    );
    assert_eq!(check(Class::B, 145.0).tx_power_limit_w, Some(CLASS_A_MAX_W));
    assert_eq!(check(Class::C, 145.0).tx_power_limit_w, Some(CLASS_A_MAX_W));
    assert_eq!(check(Class::A, 145.0).tx_power_limit_w, Some(CLASS_A_MAX_W));
  }

  #[test]
  fn itu_eirp_caps_cap_the_class_limit() {
    // 60 m（5.36 MHz）：C 类本可 1000 W，但 5.133B 脚注只允许 15 W e.i.r.p.。
    let c = check(Class::C, 5.36);
    assert!(c.class_permitted);
    assert_eq!(c.tx_power_limit_w, Some(15.0));
    assert_eq!(c.eirp_cap, Some((15.0, "5.133B")));
    // B 类在 60 m 本就 ≤15 W，取较小者仍是 15 W。
    assert_eq!(check(Class::B, 5.36).tx_power_limit_w, Some(15.0));
    // 2200 m（0.136 MHz）：B 类 ≤1 W e.i.r.p.。
    let c = check(Class::B, 0.136);
    assert!(c.class_permitted);
    assert_eq!(c.tx_power_limit_w, Some(1.0));
    assert_eq!(c.eirp_cap, Some((1.0, "5.67A")));
  }

  #[test]
  fn frequencies_outside_amateur_allocations_are_unusable() {
    // 10.5 MHz 不在任何业余划分内。
    let c = at(10.5);
    assert!(c.allocation.is_none());
    assert!(!c.class_permitted);
    assert_eq!(c.tx_power_limit_w, None);
    assert!(c.refs.is_empty());
    // 非正数 / NaN 也不会命中。
    assert!(check(Class::C, 0.0).allocation.is_none());
    assert!(check(Class::C, -3.0).allocation.is_none());
    assert!(check(Class::C, f64::NAN).allocation.is_none());
  }

  #[test]
  fn allocation_hit_carries_range_usage_and_common_band() {
    let c = at(14.2);
    let hit = c.allocation.expect("20m");
    assert_eq!(hit.range, "14-14.25MHz");
    assert_eq!(hit.usage, Usage::Exclusive);
    assert_eq!(hit.common_band, Some("20m"));
    // 23 cm（1240 MHz）不在常用波段表里，common_band 为 None（界面翻成「其他」），但划分仍命中。
    let c = at(1240.0);
    let hit = c.allocation.expect("23cm");
    assert_eq!(hit.common_band, None);
    assert_eq!(hit.usage, Usage::Secondary);
  }

  #[test]
  fn coordination_and_refs_follow_usage() {
    // 40 m（专用）：不协调，只引办法第三十条。
    let c = at(7.05);
    assert!(!c.needs_coordination);
    assert_eq!(c.refs, &[A30]);
    // 80 m（主要，与其他业务共用）：协调，引第三十、十五条。
    let c = at(3.7);
    assert!(c.needs_coordination);
    assert_eq!(c.refs, &[A30, A15]);
    // 70 cm（次要）：协调 + 不得干扰主要业务，引第三十、十五、三十二条。
    let c = at(433.0);
    assert!(c.needs_coordination);
    assert_eq!(c.refs, &[A30, A15, A32]);
    // 2 m（唯一主要）：不协调。
    assert!(!at(144.5).needs_coordination);
  }

  #[test]
  fn class_parse_is_lenient() {
    assert_eq!(Class::parse("A"), Some(Class::A));
    assert_eq!(Class::parse(" b "), Some(Class::B));
    assert_eq!(Class::parse("C"), Some(Class::C));
    assert_eq!(Class::parse("D"), None);
    assert_eq!(Class::parse(""), None);
  }

  #[test]
  fn a_class_frequency_limits_are_inclusive_at_both_ends() {
    // 办法第三十条的「30-3000MHz」两端都含。这里直接测权限判定：30MHz 与 3GHz 本身都不落在
    // 任何业余划分内，走 `check` 会被「不在频段内」先短路，测不到边界。
    assert!(class_permitted(Class::A, VHF_THRESHOLD_MHZ));
    assert!(!class_permitted(Class::A, VHF_THRESHOLD_MHZ - 0.001));
    assert!(class_permitted(Class::A, CLASS_A_UPPER_MHZ));
    assert!(!class_permitted(Class::A, CLASS_A_UPPER_MHZ + 0.001));
    // B / C 不受频率限制（与 `BAND_PERMISSIONS` 最后一行一致）。
    assert!(class_permitted(Class::B, CLASS_A_UPPER_MHZ + 0.001));
    assert!(class_permitted(Class::C, 5760.0));
  }

  #[test]
  fn band_permissions_table_agrees_with_check() {
    // `license_classes::BAND_PERMISSIONS` 是写给读者看的散文口径，判定逻辑在这里。
    // 逐行取一个典型频点对拍，两处任一方改动都不会静默漂开。
    struct Row {
      mhz: f64,
      /// 表格这一行写「哪些类别可用」。
      a: bool,
      b: bool,
      c: bool,
    }
    let rows = [
      // 「160m / 80m / 40m 等短波」→ B、C 类。
      Row {
        mhz: 3.7,
        a: false,
        b: true,
        c: true,
      },
      // 「10m（28–29.7MHz）」→ B、C 类。
      Row {
        mhz: 28.5,
        a: false,
        b: true,
        c: true,
      },
      // 「6m 及以上」/「2m / 70cm」/「23cm / 13cm 等 3GHz 以下微波」→ A、B、C 类。
      Row {
        mhz: 50.1,
        a: true,
        b: true,
        c: true,
      },
      Row {
        mhz: 145.0,
        a: true,
        b: true,
        c: true,
      },
      Row {
        mhz: 1296.0,
        a: true,
        b: true,
        c: true,
      },
      // 「3GHz 以上微波」→ 只有 B、C 类。
      Row {
        mhz: 5760.0,
        a: false,
        b: true,
        c: true,
      },
    ];
    for r in rows {
      assert_eq!(
        check(Class::A, r.mhz).class_permitted,
        r.a,
        "A 类 @ {}MHz 与 BAND_PERMISSIONS 不一致",
        r.mhz
      );
      assert_eq!(
        check(Class::B, r.mhz).class_permitted,
        r.b,
        "B 类 @ {}MHz 与 BAND_PERMISSIONS 不一致",
        r.mhz
      );
      assert_eq!(
        check(Class::C, r.mhz).class_permitted,
        r.c,
        "C 类 @ {}MHz 与 BAND_PERMISSIONS 不一致",
        r.mhz
      );
    }
  }
}
