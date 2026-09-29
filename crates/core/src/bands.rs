//! 频谱波段划分表：各波段的波长/频率范围，以及其中的业余业务、卫星业余业务频段划分与脚注。
//!
//! 约定：频率范围均含上限、不含下限；C = λf = 3×10⁸ m/s。

/// 业余业务在该频段的使用状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Usage {
  /// 专用
  Exclusive,
  /// 唯一主要
  SolePrimary,
  /// 主要
  Primary,
  /// 次要
  Secondary,
}

impl Usage {
  pub const ALL: [Self; 4] = [
    Self::Exclusive,
    Self::SolePrimary,
    Self::Primary,
    Self::Secondary,
  ];

  #[must_use]
  pub const fn label(self) -> &'static str {
    match self {
      Self::Exclusive => "专用",
      Self::SolePrimary => "唯一主要",
      Self::Primary => "主要",
      Self::Secondary => "次要",
    }
  }
}

/// 备注中的一段：普通文本或脚注编号（见 [`FOOTNOTES`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Note {
  Text(&'static str),
  Ref(&'static str),
}

/// 备注单元格。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Remark {
  Empty,
  Notes(&'static [Note]),
  /// 与上一行的备注合并（表格中为纵向合并单元格）。
  MergedAbove,
}

/// 一条业余业务 / 卫星业余业务频段划分。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Allocation {
  pub range: &'static str,
  /// 该频段也供卫星业余业务使用。
  pub satellite: bool,
  pub usage: Usage,
  pub remark: Remark,
}

/// 一个波段（带号）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Band {
  pub number: i8,
  pub name: &'static str,
  pub wavelength: &'static str,
  pub freq_name: &'static str,
  pub freq_abbr: &'static str,
  pub freq_range: &'static str,
  /// 属于「微波」范围（分米波 ~ 丝米波）。
  pub microwave: bool,
  pub allocations: &'static [Allocation],
}

impl Band {
  /// 表格中占用的行数（无划分时占 1 行）。
  #[must_use]
  pub fn rows(&self) -> usize {
    self.allocations.len().max(1)
  }

  /// 第 `i` 条划分的备注单元格需要纵向合并的行数；被上一行合并时返回 `None`。
  #[must_use]
  pub fn remark_span(&self, i: usize) -> Option<usize> {
    let list = self.allocations;
    if list.get(i)?.remark == Remark::MergedAbove {
      return None;
    }
    let merged = list[i + 1..]
      .iter()
      .take_while(|a| a.remark == Remark::MergedAbove)
      .count();
    Some(1 + merged)
  }

  /// 第 `i` 条划分实际对应的备注（合并单元格会向上查找）。
  #[must_use]
  pub fn remark_for(&self, i: usize) -> &'static [Note] {
    self.allocations[..=i.min(self.allocations.len().saturating_sub(1))]
      .iter()
      .rev()
      .find_map(|a| match a.remark {
        Remark::MergedAbove => None,
        Remark::Empty => Some(&[][..]),
        Remark::Notes(n) => Some(n),
      })
      .unwrap_or(&[])
  }
}

/// 脚注。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Footnote {
  pub code: &'static str,
  pub text: &'static str,
}

use Note::{Ref, Text};
use Usage::{Exclusive, Primary, Secondary, SolePrimary};

const fn a(range: &'static str, usage: Usage, remark: Remark) -> Allocation {
  Allocation {
    range,
    satellite: false,
    usage,
    remark,
  }
}

const fn sat(range: &'static str, usage: Usage, remark: Remark) -> Allocation {
  Allocation {
    range,
    satellite: true,
    usage,
    remark,
  }
}

const fn n(notes: &'static [Note]) -> Remark {
  Remark::Notes(notes)
}

const E: Remark = Remark::Empty;
const MERGED: Remark = Remark::MergedAbove;

const fn band(
  number: i8,
  name: &'static str,
  wavelength: &'static str,
  (freq_name, freq_abbr): (&'static str, &'static str),
  freq_range: &'static str,
  allocations: &'static [Allocation],
) -> Band {
  Band {
    number,
    name,
    wavelength,
    freq_name,
    freq_abbr,
    freq_range,
    microwave: false,
    allocations,
  }
}

const fn microwave(b: Band) -> Band {
  Band {
    microwave: true,
    ..b
  }
}

pub const BANDS: &[Band] = &[
  band(
    -1,
    "至长波/千兆米波",
    "10000-1000Mm",
    ("至低频", "TLF"),
    "0.03-0.3Hz",
    &[],
  ),
  band(
    0,
    "至长波/百兆米波",
    "1000-100Mm",
    ("至低频", "TLF"),
    "0.3-3Hz",
    &[],
  ),
  band(1, "极长波", "100-10Mm", ("极低频", "ELF"), "3-30 Hz", &[]),
  band(2, "超长波", "10-1Mm", ("超低频", "SLF"), "30-300 Hz", &[]),
  band(
    3,
    "特长波",
    "1000-100km",
    ("特低频", "ULF"),
    "300-3000 Hz",
    &[],
  ),
  band(4, "甚长波", "100-10km", ("甚低频", "VLF"), "3-30 kHz", &[]),
  band(
    5,
    "长波",
    "10km-1km",
    ("低频", "LF"),
    "30-300 kHz",
    &[a("135.7-137.8kHz", Secondary, n(&[Ref("5.67A")]))],
  ),
  band(
    6,
    "中波",
    "1000-100m",
    ("中频", "MF"),
    "300-3000 kHz",
    &[a("1800-2000kHz", Primary, n(&[Text("160米业余波段")]))],
  ),
  band(
    7,
    "短波",
    "100-10m",
    ("高频", "HF"),
    "3-30 MHz",
    &[
      a("3.5-3.9MHz", Primary, n(&[Text("80米业余波段")])),
      a("5.3515-5.3665MHz", Secondary, n(&[Ref("5.133B")])),
      sat("7-7.1MHz", Exclusive, n(&[Text("40米业余波段")])),
      a("7.1-7.2MHz", Primary, n(&[Text("40米业余波段")])),
      a(
        "10.1-10.15MHz",
        Secondary,
        n(&[Text("WARC频段；不能用于通话")]),
      ),
      sat("14-14.25MHz", Exclusive, n(&[Text("20米业余波段")])),
      a("14.25-14.35MHz", Primary, n(&[Text("20米业余波段")])),
      sat("18.068-18.168MHz", Primary, n(&[Text("WARC频段")])),
      sat("21-21.45MHz", Exclusive, n(&[Text("15米业余波段")])),
      a("24.89-24.99MHz", Primary, n(&[Text("WARC频段")])),
      sat(
        "28-29.7MHz",
        Exclusive,
        n(&[Ref("CHN7"), Text("；10米业余波段")]),
      ),
    ],
  ),
  band(
    8,
    "米波/超短波",
    "10-1m",
    ("甚高频", "VHF"),
    "30-300 MHz",
    &[
      a("50-54MHz", Secondary, n(&[Text("6米业余波段")])),
      sat("144-146MHz", SolePrimary, n(&[Text("2米业余波段")])),
      a("146-148MHz", Primary, MERGED),
    ],
  ),
  microwave(band(
    9,
    "分米波",
    "10-1dm",
    ("特高频", "UHF"),
    "300-3000 MHz",
    &[
      a("430-440MHz", Secondary, n(&[Text("0.7米业余波段")])),
      a("1240-1260MHz", Secondary, E),
      a("1260-1300MHz", Secondary, E),
      a("2300-2450MHz", Secondary, E),
    ],
  )),
  microwave(band(
    10,
    "厘米波",
    "10-1cm",
    ("超高频", "SHF"),
    "3-30 GHz",
    &[
      a("3.3-3.4GHz", Secondary, n(&[Ref("5.149")])),
      a("3.4-3.5GHz", Secondary, E),
      a("5.65-5.725GHz", Secondary, E),
      a("5.725-5.83GHz", Secondary, E),
      sat("(空对地)5.83-5.85GHz", Secondary, E),
      a("10-10.4GHz", Secondary, E),
      a("10.4-10.45GHz", Secondary, E),
      sat("10.45-10.5GHz", Secondary, E),
      sat("24-24.05GHz", Primary, E),
      a("24.05-24.25GHz", Secondary, E),
    ],
  )),
  microwave(band(
    11,
    "毫米波",
    "10-1mm",
    ("极高频", "EHF"),
    "30-300 GHz",
    &[
      sat("47-47.2GHz", Exclusive, E),
      sat("76-77.5GHz", Secondary, n(&[Ref("5.149")])),
      sat("77.5-78GHz", Primary, n(&[Ref("5.149")])),
      sat("78-79GHz", Secondary, n(&[Ref("5.149")])),
      sat("79-81GHz", Secondary, n(&[Ref("5.149")])),
      a("122.5-123GHz", Secondary, E),
      sat("134-136GHz", SolePrimary, E),
      sat("136-141GHz", Secondary, n(&[Ref("5.149")])),
      sat("241-248GHz", Secondary, n(&[Ref("5.149")])),
      sat("248-250GHz", SolePrimary, n(&[Ref("5.149")])),
    ],
  )),
  microwave(band(
    12,
    "丝米波/亚毫米波",
    "10-1dmm",
    ("至高频", "THF"),
    "300-3000 GHz",
    &[],
  )),
];

pub const FOOTNOTES: &[Footnote] = &[
  Footnote {
    code: "5.67A",
    text: "使用135.7-137.8kHz频段内频率的业余业务台站，其最大辐射功率不得超过1瓦（e.i.r.p.）。",
  },
  Footnote {
    code: "5.133B",
    text: "使用5351.5-5366.5kHz频段的业余业务电台的最大辐射功率不得超过15W（e.i.r.p.）。",
  },
  Footnote {
    code: "5.149",
    text: "敦促主管部门采用一切实际可行的措施保护射电天文业务免受有害干扰。星载电台或机载电台的发射对射电天文业务可能是特别严重的干扰源（见4.5和4.6款以及第29条）。（WRC-07）",
  },
  Footnote {
    code: "CHN7",
    text: "27.5-29.7MHz频段内现有渔业电台可用至报废为止。",
  },
];

/// 全部业余业务频段划分条数。
#[must_use]
pub fn allocation_count() -> usize {
  BANDS.iter().map(|b| b.allocations.len()).sum()
}

/// 可供卫星业余业务使用的频段条数。
#[must_use]
pub fn satellite_count() -> usize {
  BANDS
    .iter()
    .flat_map(|b| b.allocations)
    .filter(|a| a.satellite)
    .count()
}

/// 微波波段在表格中占用的总行数。
#[must_use]
pub fn microwave_rows() -> usize {
  BANDS.iter().filter(|b| b.microwave).map(Band::rows).sum()
}

#[cfg(test)]
mod tests {
  use std::collections::HashSet;

  use super::*;

  fn refs() -> impl Iterator<Item = &'static str> {
    BANDS
      .iter()
      .flat_map(|b| b.allocations)
      .filter_map(|a| match a.remark {
        Remark::Notes(n) => Some(n),
        _ => None,
      })
      .flatten()
      .filter_map(|n| match n {
        Ref(code) => Some(*code),
        Text(_) => None,
      })
  }

  #[test]
  fn band_numbers_are_consecutive() {
    let nums: Vec<i8> = BANDS.iter().map(|b| b.number).collect();
    assert_eq!(nums, (-1..=12).collect::<Vec<_>>());
  }

  #[test]
  fn microwave_bands_are_contiguous() {
    let first = BANDS.iter().position(|b| b.microwave).unwrap();
    let last = BANDS.iter().rposition(|b| b.microwave).unwrap();
    assert!(BANDS[first..=last].iter().all(|b| b.microwave));
    assert_eq!(microwave_rows(), 4 + 10 + 10 + 1);
  }

  #[test]
  fn footnote_refs_resolve_and_all_used() {
    let codes: HashSet<_> = FOOTNOTES.iter().map(|f| f.code).collect();
    let used: HashSet<_> = refs().collect();
    assert_eq!(codes.len(), FOOTNOTES.len(), "duplicate footnote code");
    assert_eq!(used, codes);
  }

  #[test]
  fn merged_remarks_have_a_head() {
    for b in BANDS {
      if let Some(first) = b.allocations.first() {
        assert_ne!(first.remark, Remark::MergedAbove, "band {}", b.number);
      }
    }
  }

  #[test]
  fn remark_span_and_resolution() {
    let vhf = BANDS.iter().find(|b| b.number == 8).unwrap();
    assert_eq!(vhf.remark_span(0), Some(1));
    assert_eq!(vhf.remark_span(1), Some(2));
    assert_eq!(vhf.remark_span(2), None);
    assert_eq!(vhf.remark_for(2), &[Text("2米业余波段")]);
    let uhf = BANDS.iter().find(|b| b.number == 9).unwrap();
    assert!(uhf.remark_for(1).is_empty());
  }

  #[test]
  fn counts() {
    assert_eq!(allocation_count(), 1 + 1 + 11 + 3 + 4 + 10 + 10);
    assert_eq!(satellite_count(), 5 + 1 + 3 + 9);
  }
}
