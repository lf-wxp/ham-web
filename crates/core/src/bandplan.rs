//! IARU 业余波段规划（三区，中国大陆口径）：各波段内的模式子段分配。
//!
//! 每个子段带两样东西：给人看的文字（[`BandSegment::text`]）与**机读**的模式类别
//! （[`SegmentModes`]）。日志体检的「模式与频率不匹配」就是据此判断的
//! （见 [`phone_out_of_segment`]）—— 数据只有这一份，文字与标志位的一致性由单测**双向**
//! 钉住（文字里提到话务 ⇔ 允许话务），免得改了文案却忘了改标志位。

use std::sync::LazyLock;

/// 一个子段允许的模式类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentModes {
  /// 电报。
  pub cw: bool,
  /// 窄带数据（RTTY / PSK / FT8 / JT65 等）。
  pub narrow: bool,
  /// 话务（SSB / AM / FM / 数字语音）。
  ///
  /// 体检只用到这一位 —— 但它单独看没有意义（「这个子段是干什么的」要看三位），
  /// 所以另外两位一并留着，作为文字一致性的依据。
  pub phone: bool,
}

impl SegmentModes {
  /// 仅电报（如 1.800–1.830）。
  pub const CW: Self = Self {
    cw: true,
    narrow: false,
    phone: false,
  };
  /// 仅窄带数据（如 1.830–1.840）。
  pub const NARROW: Self = Self {
    cw: false,
    narrow: true,
    phone: false,
  };
  /// 电报 + 窄带数据（如 7.000–7.100）。
  pub const CW_NARROW: Self = Self {
    cw: true,
    narrow: true,
    phone: false,
  };
  /// 仅话务（如 3.700–3.900）。
  pub const PHONE: Self = Self {
    cw: false,
    narrow: false,
    phone: true,
  };
  /// 电报 + 话务（如 70cm 的弱信号 / 卫星段：CW 与 SSB 各占一半）。
  pub const CW_PHONE: Self = Self {
    cw: true,
    narrow: false,
    phone: true,
  };
  /// 话务 + 窄带数据（如 60m 那一段）。
  pub const PHONE_NARROW: Self = Self {
    cw: false,
    narrow: true,
    phone: true,
  };
  /// 各种模式。
  pub const ALL: Self = Self {
    cw: true,
    narrow: true,
    phone: true,
  };

  /// 该子段是否允许话务。
  #[must_use]
  pub const fn phone_allowed(self) -> bool {
    self.phone
  }
}

/// 一个模式子段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BandSegment {
  /// 频率子段，如 `14.000–14.150`（MHz）。
  pub range: &'static str,
  /// 模式说明（给人看）。
  pub text: &'static str,
  /// 模式类别（机读）。
  pub modes: SegmentModes,
}

/// 一个波段规划。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BandPlan {
  pub band: &'static str,
  pub freq_range: &'static str,
  /// 频率子段列表（按频率升序，互不重叠）。
  pub segments: &'static [BandSegment],
}

/// 主要业余波段的模式子段分配。
pub const BAND_PLANS: &[BandPlan] = &[
  BandPlan {
    band: "160m",
    freq_range: "1.8–2.0 MHz",
    segments: &[
      BandSegment {
        range: "1.800–1.830",
        text: "CW",
        modes: SegmentModes::CW,
      },
      BandSegment {
        range: "1.830–1.840",
        text: "窄带数据",
        modes: SegmentModes::NARROW,
      },
      BandSegment {
        range: "1.840–2.000",
        text: "话音（SSB）",
        modes: SegmentModes::PHONE,
      },
    ],
  },
  BandPlan {
    band: "80m",
    freq_range: "3.5–3.9 MHz",
    segments: &[
      BandSegment {
        range: "3.500–3.600",
        text: "CW",
        modes: SegmentModes::CW,
      },
      BandSegment {
        range: "3.600–3.700",
        text: "数据",
        modes: SegmentModes::NARROW,
      },
      BandSegment {
        range: "3.700–3.900",
        text: "话音（SSB）",
        modes: SegmentModes::PHONE,
      },
    ],
  },
  BandPlan {
    band: "40m",
    freq_range: "7.0–7.2 MHz",
    segments: &[
      BandSegment {
        range: "7.000–7.100",
        text: "CW / 数据",
        modes: SegmentModes::CW_NARROW,
      },
      BandSegment {
        range: "7.100–7.200",
        text: "话音（SSB）",
        modes: SegmentModes::PHONE,
      },
    ],
  },
  BandPlan {
    band: "60m",
    freq_range: "5.3515–5.3665 MHz",
    segments: &[BandSegment {
      range: "5.3515–5.3665",
      text: "USB 话音 / 窄带数据（WARC，e.i.r.p. ≤15W）",
      modes: SegmentModes::PHONE_NARROW,
    }],
  },
  BandPlan {
    band: "30m",
    freq_range: "10.10–10.15 MHz",
    segments: &[BandSegment {
      range: "10.100–10.150",
      text: "仅 CW / 窄带数据（WARC）",
      modes: SegmentModes::CW_NARROW,
    }],
  },
  BandPlan {
    band: "20m",
    freq_range: "14.0–14.35 MHz",
    segments: &[
      BandSegment {
        range: "14.000–14.150",
        text: "CW",
        modes: SegmentModes::CW,
      },
      BandSegment {
        range: "14.150–14.350",
        text: "话音（SSB）",
        modes: SegmentModes::PHONE,
      },
    ],
  },
  BandPlan {
    band: "17m",
    freq_range: "18.068–18.168 MHz",
    segments: &[
      BandSegment {
        range: "18.068–18.110",
        text: "CW",
        modes: SegmentModes::CW,
      },
      BandSegment {
        range: "18.110–18.168",
        text: "话音（WARC）",
        modes: SegmentModes::PHONE,
      },
    ],
  },
  BandPlan {
    band: "15m",
    freq_range: "21.0–21.45 MHz",
    segments: &[
      BandSegment {
        range: "21.000–21.150",
        text: "CW",
        modes: SegmentModes::CW,
      },
      BandSegment {
        range: "21.150–21.450",
        text: "话音（SSB）",
        modes: SegmentModes::PHONE,
      },
    ],
  },
  BandPlan {
    band: "12m",
    freq_range: "24.89–24.99 MHz",
    segments: &[
      BandSegment {
        range: "24.890–24.930",
        text: "CW",
        modes: SegmentModes::CW,
      },
      BandSegment {
        range: "24.930–24.990",
        text: "话音（WARC）",
        modes: SegmentModes::PHONE,
      },
    ],
  },
  BandPlan {
    band: "10m",
    freq_range: "28.0–29.7 MHz",
    segments: &[
      BandSegment {
        range: "28.000–28.200",
        text: "CW",
        modes: SegmentModes::CW,
      },
      BandSegment {
        range: "28.200–29.700",
        text: "话音 / 信标",
        modes: SegmentModes::PHONE,
      },
    ],
  },
  BandPlan {
    band: "6m",
    freq_range: "50–54 MHz",
    segments: &[
      BandSegment {
        range: "50.0–50.1",
        text: "CW / 信标",
        modes: SegmentModes::CW,
      },
      BandSegment {
        range: "50.1–50.3",
        text: "SSB",
        modes: SegmentModes::PHONE,
      },
      BandSegment {
        range: "50.3–54.0",
        text: "FM / 数据",
        modes: SegmentModes::PHONE_NARROW,
      },
    ],
  },
  BandPlan {
    band: "2m",
    freq_range: "144–148 MHz",
    segments: &[
      BandSegment {
        range: "144.000–144.100",
        text: "CW / 数据",
        modes: SegmentModes::CW_NARROW,
      },
      BandSegment {
        range: "144.100–144.400",
        text: "SSB",
        modes: SegmentModes::PHONE,
      },
      BandSegment {
        range: "144.400–145.800",
        text: "FM / 中继",
        modes: SegmentModes::PHONE,
      },
      BandSegment {
        range: "145.800–146.000",
        text: "业余卫星（FM / SSB）",
        modes: SegmentModes::PHONE,
      },
      BandSegment {
        range: "146.000–148.000",
        text: "FM / 中继（扩展段）",
        modes: SegmentModes::PHONE,
      },
    ],
  },
  BandPlan {
    band: "70cm",
    freq_range: "430–440 MHz",
    segments: &[
      BandSegment {
        range: "430.000–432.000",
        text: "各种模式（CW / 数据 / 话音）",
        modes: SegmentModes::ALL,
      },
      BandSegment {
        range: "432.000–438.000",
        text: "弱信号 / 卫星（CW / SSB）",
        modes: SegmentModes::CW_PHONE,
      },
      BandSegment {
        range: "438.000–440.000",
        text: "FM / 中继",
        modes: SegmentModes::PHONE,
      },
    ],
  },
];

/// 频率文本（`14.000–14.150`、`1.8–2.0 MHz`、`136–141kHz`）→ MHz 区间（半开 `[低, 高)`）。
///
/// 兼容连接号与单位后缀：规划表里两种写法都有；`kHz` 后缀会**换算成 MHz** ——
/// 不换算的话，将来加一条 kHz 子段会拿 MHz 频率去比 kHz 数值，判定全错。
#[must_use]
pub fn parse_range(text: &str) -> Option<(f64, f64)> {
  let body = text.trim();
  let (body, khz) = match body.strip_suffix("kHz") {
    Some(b) => (b, true),
    None => match body.strip_suffix("MHz") {
      Some(b) => (b, false),
      None => (body, false),
    },
  };
  // 表里用的是连接号（en dash），但用户与未来的数据可能写普通 hyphen，两个都认。
  let (lo, hi) = body
    .trim()
    .split_once('–')
    .or_else(|| body.split_once('-'))?;
  let lo: f64 = lo.trim().parse().ok()?;
  let hi: f64 = hi.trim().parse().ok()?;
  let scale = if khz { 1000.0 } else { 1.0 };
  (hi > lo).then_some((lo / scale, hi / scale))
}

/// 预解析的子段边界（表是静态的，[`segment_at`] 每条通联都会调用，每次现解析字符串是纯浪费）。
///
/// 解析失败的子段在这里被丢掉 —— 数据正确性由
/// [`tests::segment_ranges_parse_and_sit_inside_the_band`] 钉住，解析不出来的表行会让单测红。
static SEGMENT_RANGES: LazyLock<Vec<(&'static BandPlan, &'static BandSegment, f64, f64)>> =
  LazyLock::new(|| {
    BAND_PLANS
      .iter()
      .flat_map(|plan| {
        plan
          .segments
          .iter()
          .filter_map(move |seg| parse_range(seg.range).map(|(lo, hi)| (plan, seg, lo, hi)))
      })
      .collect()
  });

/// 某个频率落在哪条规划的子段里（含波段与子段）。
#[must_use]
pub fn segment_at(mhz: f64) -> Option<(&'static BandPlan, &'static BandSegment)> {
  SEGMENT_RANGES
    .iter()
    .find(|(_, _, lo, hi)| mhz >= *lo && mhz < *hi)
    .map(|(plan, seg, _, _)| (*plan, *seg))
}

/// 该模式是否属于话务。
///
/// 复用奖状统计的模式分类（[`crate::award_progress::mode_class`]），不另立一套映射 ——
/// 新增模式时两处一起生效。
#[must_use]
pub fn is_phone_mode(mode: &str) -> bool {
  crate::award_progress::mode_class(mode) == "Phone"
}

/// 话务落在**不允许话务**的子段里 → 返回该子段的（波段, 子段）。
///
/// 返回 `None` 的情形一并说明，因为它们都是「不该报」的：
/// - 记录里的模式不是话务（CW / 数据在话务段是常见且各区域规划并未普遍禁止的，
///   报了就是误报 —— 宁可漏报也不误报）；
/// - 频率不在已收录的子段里（微波、越界频率等）。
#[must_use]
pub fn phone_out_of_segment(
  mode: &str,
  mhz: f64,
) -> Option<(&'static BandPlan, &'static BandSegment)> {
  if !is_phone_mode(mode) {
    return None;
  }
  let (plan, seg) = segment_at(mhz)?;
  (!seg.modes.phone_allowed()).then_some((plan, seg))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::bands::AMATEUR_BAND_EDGES;
  use crate::logbook::MODES;

  #[test]
  fn band_plans_are_populated() {
    assert!(BAND_PLANS.len() >= 10);
    for b in BAND_PLANS {
      assert!(!b.band.is_empty());
      assert!(!b.segments.is_empty());
    }
  }

  #[test]
  fn segment_ranges_parse_and_sit_inside_the_band() {
    for plan in BAND_PLANS {
      let (band_lo, band_hi) = parse_range(plan.freq_range)
        .unwrap_or_else(|| panic!("{} 的频段范围解析不了：{}", plan.band, plan.freq_range));
      // 与 ITU 边界（唯一事实来源）对得上：规划的频段范围必须落在该波段内。
      if let Some(&(_, edge_lo, edge_hi)) = AMATEUR_BAND_EDGES
        .iter()
        .find(|&&(name, _, _)| name == plan.band)
      {
        assert!(
          band_lo >= edge_lo - 1e-9 && band_hi <= edge_hi + 1e-9,
          "{} 的规划范围 {band_lo}–{band_hi} 超出 ITU 边界 {edge_lo}–{edge_hi}",
          plan.band
        );
      }
      let mut prev: Option<(f64, f64)> = None;
      for seg in plan.segments {
        let (lo, hi) = parse_range(seg.range)
          .unwrap_or_else(|| panic!("{} 的子段解析不了：{}", plan.band, seg.range));
        assert!(
          lo >= band_lo - 1e-9 && hi <= band_hi + 1e-9,
          "{} 的子段 {} 超出该波段范围 {band_lo}–{band_hi}",
          plan.band,
          seg.range
        );
        if let Some((_, prev_hi)) = prev {
          assert!(
            lo >= prev_hi - 1e-9,
            "{} 的子段 {} 与前一段重叠或乱序",
            plan.band,
            seg.range
          );
        }
        prev = Some((lo, hi));
      }
    }
  }

  #[test]
  fn segment_text_and_mode_flags_agree_both_ways() {
    // 双向检查：改了文案忘了改标志位（或反过来）都会红。
    let has = |text: &str, needles: &[&str]| needles.iter().any(|n| text.contains(n));
    for plan in BAND_PLANS {
      for seg in plan.segments {
        let t = seg.text;
        let m = seg.modes;
        if has(t, &["话音", "SSB", "USB", "FM", "LSB"]) {
          assert!(
            m.phone,
            "{} {}：文字说了话务，标志位却没允许",
            plan.band, seg.range
          );
        }
        if m.phone {
          assert!(
            has(t, &["话音", "SSB", "USB", "FM", "LSB"]),
            "{} {}：标志位允许话务，文字里却没写",
            plan.band,
            seg.range
          );
        }
        let says_cw = t.contains("CW");
        assert_eq!(
          says_cw, m.cw,
          "{} {}：CW 与文字不一致",
          plan.band, seg.range
        );
        let says_narrow = has(t, &["数据", "窄带"]);
        assert_eq!(
          says_narrow, m.narrow,
          "{} {}：窄带数据与文字不一致",
          plan.band, seg.range
        );
      }
    }
  }

  #[test]
  fn phone_is_flagged_only_in_segments_that_forbid_it() {
    // 规划里明确不放话务的地方（含文档里举的两个例子）。
    for (mode, mhz) in [
      ("SSB", 14.050),  // 20m 的 CW 段
      ("AM", 3.550),    // 80m 的 CW 段
      ("SSB", 10.130),  // 30m 整段（WARC：只允许 CW / 窄带数据）
      ("SSB", 144.050), // 2m 的 CW / 数据段
      ("FM", 28.100),   // 10m 的 CW 段
      ("SSB", 18.090),  // 17m 的 CW 段
    ] {
      let (plan, seg) =
        phone_out_of_segment(mode, mhz).unwrap_or_else(|| panic!("{mode}@{mhz} 应被报出"));
      assert!(!seg.modes.phone_allowed());
      assert!(!plan.band.is_empty());
    }

    // 正常用法一律不报。
    for (mode, mhz) in [
      ("SSB", 14.200),
      ("SSB", 7.150),
      ("CW", 7.030),
      ("CW", 14.050),  // CW 在自己的段里
      ("FT8", 14.074), // 数据落在 CW 段（本模块只判话务，见函数文档）
      ("FT8", 10.136), // 30m 的数据用法
      ("FM", 29.600),  // 10m 的话务段
      ("FM", 145.500), // 2m 的 FM / 中继段
      ("SSB", 144.200),
      ("SSB", 432.200), // 70cm 弱信号段
      ("SSB", 5.360),   // 60m 的 USB 话音段
    ] {
      assert!(
        phone_out_of_segment(mode, mhz).is_none(),
        "{mode}@{mhz} 不该被报出"
      );
    }
  }

  #[test]
  fn nothing_is_flagged_off_plan_or_without_a_mode() {
    // 微波 / 业余频段之外：没有子段可判。
    assert!(segment_at(2400.0).is_none());
    assert!(segment_at(27.555).is_none());
    assert!(phone_out_of_segment("SSB", 2400.0).is_none());
    // 模式认不出（或本就不是话务）时不动嘴。
    for mode in ["", "OTHER", "DATA", "RTTY", "JT65", "SSTV", "DMR  "] {
      let _ = phone_out_of_segment(mode, 14.050);
      if !is_phone_mode(mode) {
        assert!(phone_out_of_segment(mode, 14.050).is_none(), "{mode}");
      }
    }
  }

  #[test]
  fn voice_modes_are_recognised_and_cw_data_are_not() {
    for mode in [
      "SSB", "ssb", "USB", "LSB", "AM", "FM", "DMR", "C4FM", "DSTAR",
    ] {
      assert!(is_phone_mode(mode), "{mode} 应是话务");
    }
    for mode in [
      "CW", "RTTY", "FT8", "FT4", "JS8", "Q65", "JT65", "MSK144", "PSK31", "DATA",
    ] {
      assert!(!is_phone_mode(mode), "{mode} 不是话务");
    }
    // 界面里的可选模式必须都能归类（新增模式时这条会提醒你决定它算不算话务）。
    for mode in MODES {
      let classified = is_phone_mode(mode)
        || matches!(
          crate::award_progress::mode_class(mode),
          "CW" | "Digital" | "Phone"
        );
      assert!(classified, "模式 {mode} 没有归类");
    }
  }

  #[test]
  fn range_parsing_accepts_both_dashes_and_units() {
    assert_eq!(parse_range("14.000–14.150"), Some((14.0, 14.15)));
    assert_eq!(parse_range("1.8–2.0 MHz"), Some((1.8, 2.0)));
    assert_eq!(parse_range(" 50 – 54 MHz "), Some((50.0, 54.0)));
    // kHz 后缀要换算成 MHz，否则与 MHz 频率比较时量纲不对。
    assert_eq!(parse_range("136–141kHz"), Some((0.136, 0.141)));
    assert_eq!(parse_range("18.068"), None);
    assert_eq!(parse_range("14.150–14.000"), None, "反着写不算");
    assert_eq!(parse_range(""), None);
  }
}
