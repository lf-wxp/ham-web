//! 电台频率护栏：把「电台此刻调在哪个频率、什么模式」对照操作证与波段规划，给出一盏灯。
//!
//! 只做**判定**，不碰电台：CAT 联动对日志页是只读的（读频率与模式回填表单），这里在那份读数上
//! 多回答一句「如果现在发射，合规吗」。判定全部复用站内已有的内核，不另立一套规则：
//!
//! - 频率是否在业余频段、类别是否有权使用、是否需要协调、功率上限：[`crate::license_compliance`]；
//! - 话务落在不许话务的子段：[`crate::bandplan::phone_out_of_segment`]（与日志体检同一个判定）。
//!
//! # 灯的口径
//!
//! | 灯 | 含义 | 来源 |
//! | --- | --- | --- |
//! | 灰 [`Lamp::OutOfBand`] | 不在任何业余频段 | 常见于只收听，**不算违规**，所以不亮红 |
//! | 绿 [`Lamp::Ok`] | 没有发现问题 | |
//! | 黄 [`Lamp::Notice`] | 提醒，不是违规 | 需要频率协调；CW / 数据落在话务专用子段（偏离波段规划） |
//! | 红 [`Lamp::Alert`] | 此刻发射不合规 | 类别无权使用该频率；话务落在不许话务的子段 |
//!
//! 「偏离波段规划」只给黄灯、不升红：波段规划是 IARU 的君子协定，CW / 数据出现在话务段很常见，
//! 各区域规划并未普遍禁止（与 `bandplan` 里「宁可漏报也不误报」同一口径）。
//!
//! # 边界
//!
//! 读到的是电台**当前显示的那个 VFO**：分频（split）、卫星模式下，发射频率可能是另一个，
//! 这里不知道也不猜。数字语音（DMR / C4FM 等）按话务算；`DATA` 等其余模式按数据算。

use crate::bandplan;
use crate::license_compliance::{self, Class};

/// 一盏灯的颜色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lamp {
  /// 灰：不在业余频段。
  OutOfBand,
  /// 绿：没发现问题。
  Ok,
  /// 黄：提醒。
  Notice,
  /// 红：此刻发射不合规。
  Alert,
}

/// 一条发现。`band` / `range` 是数据值（波段名、子段范围原文），界面按需翻译 / 原样显示。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Finding {
  /// 红：该类别无权使用这个频率（办法第三十条）。
  ClassNotPermitted(Class),
  /// 红：话务落在不许话务的子段。
  PhoneOutOfSegment {
    band: &'static str,
    range: &'static str,
  },
  /// 黄：需要开展频率协调（办法第十五条）。
  NeedsCoordination,
  /// 黄：CW / 数据落在话务专用子段（偏离波段规划）。
  OffPlan {
    band: &'static str,
    range: &'static str,
  },
}

impl Finding {
  /// 这条发现对应的灯。
  #[must_use]
  pub const fn lamp(self) -> Lamp {
    match self {
      Self::ClassNotPermitted(_) | Self::PhoneOutOfSegment { .. } => Lamp::Alert,
      Self::NeedsCoordination | Self::OffPlan { .. } => Lamp::Notice,
    }
  }
}

/// 一次判定的结论。
#[derive(Debug, Clone, PartialEq)]
pub struct Verdict {
  /// 频率是否落在业余业务频段里。
  pub in_amateur_band: bool,
  /// 常用波段名（`"20m"`）；不在常用表里（微波等）或不在业余频段时为 `None`。
  pub band: Option<&'static str>,
  /// 发现的问题，红灯在前、黄灯在后；空表示没发现问题。
  pub findings: Vec<Finding>,
  /// 该类别在这个频率的发射功率上限（W）；没选类别、或类别无权使用时为 `None`。
  pub power_limit_w: Option<f64>,
}

impl Verdict {
  /// 整体的灯：取各条发现里最严重的；不在业余频段是灰灯。
  #[must_use]
  pub fn lamp(&self) -> Lamp {
    if !self.in_amateur_band {
      return Lamp::OutOfBand;
    }
    self
      .findings
      .iter()
      .map(|f| f.lamp())
      .fold(
        Lamp::Ok,
        |worst, l| {
          if rank(l) > rank(worst) { l } else { worst }
        },
      )
  }
}

/// 灯的严重程度（灰不参与比较：灰灯只在「不在业余频段」时整体返回）。
const fn rank(l: Lamp) -> u8 {
  match l {
    Lamp::OutOfBand | Lamp::Ok => 0,
    Lamp::Notice => 1,
    Lamp::Alert => 2,
  }
}

/// 判定。
///
/// - `class`：用户的操作证类别；没选就只做与类别无关的检查（是否在业余频段、协调、模式与子段），
///   不给「类别无权使用」也不给功率上限；
/// - `freq_mhz`：电台读到的频率；非有限 / 非正按「不在业余频段」处理；
/// - `mode`：电台读到的模式（`USB` / `CW` / `DATA` …）；还没读到（`None`）就跳过与模式有关的检查。
#[must_use]
pub fn check(class: Option<Class>, freq_mhz: f64, mode: Option<&str>) -> Verdict {
  // 类别无关的部分（划分、协调）用 C 类取：C 类对任何频率都「允许」，其余输出只在选了类别时才采用。
  let base = license_compliance::check(class.unwrap_or(Class::C), freq_mhz);
  let Some(allocation) = base.allocation else {
    return Verdict {
      in_amateur_band: false,
      band: None,
      findings: Vec::new(),
      power_limit_w: None,
    };
  };

  let mut findings = Vec::new();
  let class_permitted = class.is_none_or(|_| base.class_permitted);
  if let Some(class) = class
    && !base.class_permitted
  {
    findings.push(Finding::ClassNotPermitted(class));
  }
  if let Some(mode) = mode {
    if let Some((plan, seg)) = bandplan::phone_out_of_segment(mode, freq_mhz) {
      findings.push(Finding::PhoneOutOfSegment {
        band: plan.band,
        range: seg.range,
      });
    } else if !bandplan::is_phone_mode(mode)
      && let Some((plan, seg)) = bandplan::segment_at(freq_mhz)
      && seg.modes == bandplan::SegmentModes::PHONE
    {
      findings.push(Finding::OffPlan {
        band: plan.band,
        range: seg.range,
      });
    }
  }
  // 类别无权使用时谈协调没有意义：先要解决的是「根本不该用这个频率」。
  if base.needs_coordination && class_permitted {
    findings.push(Finding::NeedsCoordination);
  }
  // 红灯在前：界面按顺序列出，最该先看的放最上面。
  findings.sort_by_key(|f| std::cmp::Reverse(rank(f.lamp())));

  Verdict {
    in_amateur_band: true,
    band: allocation.common_band,
    findings,
    power_limit_w: class.and(base.tx_power_limit_w),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn kinds(v: &Verdict) -> Vec<&'static str> {
    v.findings
      .iter()
      .map(|f| match f {
        Finding::ClassNotPermitted(_) => "class",
        Finding::PhoneOutOfSegment { .. } => "phone",
        Finding::NeedsCoordination => "coord",
        Finding::OffPlan { .. } => "offplan",
      })
      .collect()
  }

  /// 不在任何业余频段（也包括非法频率）：灰灯、没有发现，也不 panic。
  #[test]
  fn frequencies_outside_the_amateur_bands_get_a_grey_lamp() {
    // 7.3 MHz：中国的 40 m 只到 7.2；1 MHz 是中波广播。
    for f in [7.3, 1.0, 100.0, 5000.0, 0.0, -14.0, f64::NAN, f64::INFINITY] {
      let v = check(Some(Class::C), f, Some("USB"));
      assert!(!v.in_amateur_band, "{f}");
      assert_eq!(v.lamp(), Lamp::OutOfBand, "{f}");
      assert!(v.findings.is_empty(), "{f}");
      assert_eq!(v.power_limit_w, None, "{f}");
    }
  }

  #[test]
  fn a_class_is_red_on_hf_but_fine_on_two_metres() {
    // A 类只有 30–3000 MHz：20 m 是「证外」。
    let v = check(Some(Class::A), 14.2, Some("USB"));
    assert_eq!(v.lamp(), Lamp::Alert);
    assert!(v.findings.contains(&Finding::ClassNotPermitted(Class::A)));
    assert_eq!(v.power_limit_w, None, "无权使用时没有功率上限");
    // 2 m：可用，25 W。
    let v = check(Some(Class::A), 145.5, Some("FM"));
    assert_eq!(v.lamp(), Lamp::Ok, "{:?}", v.findings);
    assert_eq!(v.power_limit_w, Some(25.0));
    // C 类在 20 m 是允许的。
    assert!(
      !check(Some(Class::C), 14.2, Some("USB"))
        .findings
        .contains(&Finding::ClassNotPermitted(Class::C))
    );
  }

  /// 话务落在不许话务的子段：红；同一频率上换成 CW 不报（CW / 数据在那里是常见的）。
  #[test]
  fn phone_in_a_segment_that_forbids_phone_is_red() {
    // 14.000–14.070 是电报 / 窄带数据：先钉住前提，免得改了波段规划测试悄悄空转。
    let (_, seg) = bandplan::segment_at(14.03).unwrap();
    assert!(!seg.modes.phone_allowed(), "前提：{}", seg.range);
    for mode in ["USB", "LSB", "usb", "FM", "AM", "SSB"] {
      let v = check(Some(Class::C), 14.03, Some(mode));
      assert_eq!(v.lamp(), Lamp::Alert, "{mode}");
      assert_eq!(kinds(&v), vec!["phone"], "{mode}");
    }
    for mode in ["CW", "RTTY", "DATA"] {
      let v = check(Some(Class::C), 14.03, Some(mode));
      assert_eq!(v.lamp(), Lamp::Ok, "{mode}: {:?}", v.findings);
    }
  }

  /// CW / 数据落在话务专用子段：只给黄灯「偏离波段规划」，话务本身不报。
  ///
  /// 3.8 MHz（80 m）按划分数据本来就要协调，所以每条结论里都带一条协调；这里把两件事分开断言，
  /// 免得以后划分表变了、「偏离规划」的断言跟着悄悄失去意义。
  #[test]
  fn cw_or_data_in_a_phone_only_segment_is_only_a_notice() {
    let (_, seg) = bandplan::segment_at(3.8).unwrap();
    assert_eq!(
      seg.modes,
      bandplan::SegmentModes::PHONE,
      "前提：{}",
      seg.range
    );
    assert!(
      license_compliance::check(Class::C, 3.8).needs_coordination,
      "前提"
    );
    for mode in ["CW", "RTTY", "DATA"] {
      let v = check(Some(Class::C), 3.8, Some(mode));
      assert_eq!(v.lamp(), Lamp::Notice, "{mode}");
      assert_eq!(kinds(&v), vec!["offplan", "coord"], "{mode}");
      assert!(
        v.findings.iter().all(|f| f.lamp() == Lamp::Notice),
        "{mode}: 偏离规划不能升成红灯"
      );
    }
    // 话务在话务段里是对路的：只剩协调这一条提醒。
    let v = check(Some(Class::C), 3.8, Some("LSB"));
    assert_eq!(kinds(&v), vec!["coord"]);
  }

  /// 需要协调：黄灯；类别无权使用时不再谈协调。
  #[test]
  fn coordination_is_a_notice_and_only_when_the_class_may_use_the_frequency() {
    // 前提来自 `license_compliance` 自己：10.125 MHz（30 m）是次要业务，须协调。
    assert!(license_compliance::check(Class::C, 10.125).needs_coordination);
    let v = check(Some(Class::C), 10.125, Some("CW"));
    assert_eq!(v.lamp(), Lamp::Notice);
    assert_eq!(kinds(&v), vec!["coord"]);
    // A 类无权使用 30 m：只报「证外」，不再叠一条协调。
    let v = check(Some(Class::A), 10.125, Some("CW"));
    assert_eq!(kinds(&v), vec!["class"]);
  }

  /// 没选类别、没读到模式：只做与它们无关的检查，且不编造结论。
  #[test]
  fn missing_class_or_mode_skips_only_what_depends_on_them() {
    // 没选类别：20 m 不会因「证外」变红，也不给功率上限。
    let v = check(None, 14.2, Some("USB"));
    assert_eq!(v.lamp(), Lamp::Ok, "{:?}", v.findings);
    assert_eq!(v.power_limit_w, None);
    // 没选类别照样查模式与子段、协调。
    assert_eq!(check(None, 14.03, Some("USB")).lamp(), Lamp::Alert);
    assert_eq!(check(None, 10.125, None).lamp(), Lamp::Notice);
    // 没读到模式：跳过模式相关的检查（14.03 上话务才会红）。
    assert_eq!(check(Some(Class::C), 14.03, None).lamp(), Lamp::Ok);
    // 类别与模式都没有：剩下「在不在业余频段」。
    let v = check(None, 14.2, None);
    assert!(v.in_amateur_band);
    assert_eq!(v.lamp(), Lamp::Ok);
    assert_eq!(v.band, Some("20m"));
  }

  /// 红灯排在黄灯前面，整体灯取最严重的那条。
  #[test]
  fn findings_are_listed_red_first_and_the_lamp_is_the_worst_one() {
    // A 类在 3.8 MHz 发 CW：证外（红）+ 偏离规划（黄）。
    let v = check(Some(Class::A), 3.8, Some("CW"));
    assert_eq!(kinds(&v), vec!["class", "offplan"]);
    assert_eq!(v.lamp(), Lamp::Alert);
  }

  #[test]
  fn power_limit_follows_the_class_and_is_capped_by_itu_footnotes() {
    assert_eq!(
      check(Some(Class::C), 7.1, Some("CW")).power_limit_w,
      Some(1000.0)
    );
    assert_eq!(
      check(Some(Class::B), 7.1, Some("CW")).power_limit_w,
      Some(15.0)
    );
    assert_eq!(
      check(Some(Class::C), 145.0, Some("FM")).power_limit_w,
      Some(25.0)
    );
    // 60 m 的 5.133B 脚注把 C 类的 1000 W 压到 15 W。
    assert_eq!(
      check(Some(Class::C), 5.36, Some("USB")).power_limit_w,
      Some(15.0)
    );
  }
}
