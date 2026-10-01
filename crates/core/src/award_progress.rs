//! 由通联日志统计常见奖状进度：DXCC（总数 / 分波段 / 分模式）、WAZ、WAC、VUCC。
//!
//! 分区与大洲优先取记录中的 `CQZ` / `ITUZ` 字段，否则取 DXCC 实体的主分区（跨多个分区的
//! 大国只是估计）。「已确认」指 QSL 已收到（纸卡 / LoTW / eQSL）。

use std::collections::{BTreeMap, BTreeSet};

use crate::logbook::LogEntry;

/// DXCC 基础奖要求的实体数。
pub const DXCC_TARGET: usize = 100;
/// WAZ：40 个 CQ 分区。
pub const WAZ_TARGET: usize = 40;
/// WAC 统计的 6 个大洲（南极洲不计）。
pub const CONTINENTS: [(&str, &str); 6] = [
  ("NA", "北美洲"),
  ("SA", "南美洲"),
  ("EU", "欧洲"),
  ("AF", "非洲"),
  ("AS", "亚洲"),
  ("OC", "大洋洲"),
];

/// VUCC 各波段要求的网格数；卫星单独计。
const VUCC_TARGETS: &[(&str, usize)] = &[
  ("6m", 100),
  ("2m", 100),
  ("1.25m", 50),
  ("70cm", 50),
  ("33cm", 25),
  ("23cm", 25),
  ("13cm", 10),
  ("9cm", 5),
  ("6cm", 5),
  ("3cm", 5),
  ("SAT", 100),
];

/// 某项奖状的已通联 / 已确认集合。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress<T: Ord> {
  pub worked: BTreeSet<T>,
  pub confirmed: BTreeSet<T>,
}

impl<T: Ord> Default for Progress<T> {
  fn default() -> Self {
    Self {
      worked: BTreeSet::new(),
      confirmed: BTreeSet::new(),
    }
  }
}

impl<T: Ord + Clone> Progress<T> {
  fn add(&mut self, item: T, confirmed: bool) {
    if confirmed {
      self.confirmed.insert(item.clone());
    }
    self.worked.insert(item);
  }
}

/// DXCC 分模式统计的类别。
#[must_use]
pub fn mode_class(mode: &str) -> &'static str {
  match mode.trim().to_ascii_uppercase().as_str() {
    "CW" => "CW",
    "SSB" | "USB" | "LSB" | "AM" | "FM" | "DMR" | "C4FM" | "DSTAR" | "DIGITALVOICE" | "FREEDV" => {
      "Phone"
    }
    _ => "Digital",
  }
}

/// 全部奖状进度。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AwardProgress {
  pub dxcc: Progress<u16>,
  /// key = 波段（如 `20m`）。
  pub dxcc_by_band: BTreeMap<String, Progress<u16>>,
  /// key = [`mode_class`]。
  pub dxcc_by_mode: BTreeMap<&'static str, Progress<u16>>,
  pub waz: Progress<u8>,
  /// 大洲代码。
  pub wac: Progress<&'static str>,
  /// key = 波段或 `SAT`，值为 4 位网格。
  pub vucc: BTreeMap<String, Progress<String>>,
}

impl AwardProgress {
  /// 从日志统计。
  #[must_use]
  pub fn from_entries(entries: &[LogEntry]) -> Self {
    let mut p = Self::default();
    for e in entries {
      let ok = e.qsl_rcvd;
      let band = e.band_label();
      if let Some(en) = e.entity() {
        p.dxcc.add(en.dxcc, ok);
        if !band.is_empty() {
          p.dxcc_by_band
            .entry(band.clone())
            .or_default()
            .add(en.dxcc, ok);
        }
        p.dxcc_by_mode
          .entry(mode_class(&e.mode))
          .or_default()
          .add(en.dxcc, ok);
        if let Some((code, _)) = CONTINENTS.iter().find(|(c, _)| *c == en.continent) {
          p.wac.add(code, ok);
        }
      }
      if let Some(z) = e.cq_zone() {
        p.waz.add(z, ok);
      }
      let grid = e.gridsquare.trim().to_ascii_uppercase();
      if let Some(g4) = grid.get(..4).filter(|g| is_grid4(g)) {
        let key = if e.prop_mode.eq_ignore_ascii_case("SAT") || !e.sat_name.trim().is_empty() {
          "SAT".to_owned()
        } else {
          band
        };
        if vucc_target(&key).is_some() {
          p.vucc.entry(key).or_default().add(g4.to_owned(), ok);
        }
      }
    }
    p
  }
}

/// VUCC 某波段（或 `SAT`）要求的网格数；HF 波段不适用时为 `None`。
#[must_use]
pub fn vucc_target(band: &str) -> Option<usize> {
  VUCC_TARGETS
    .iter()
    .find(|(b, _)| *b == band)
    .map(|(_, n)| *n)
}

fn is_grid4(g: &str) -> bool {
  let b = g.as_bytes();
  b.len() == 4
    && (b'A'..=b'R').contains(&b[0])
    && (b'A'..=b'R').contains(&b[1])
    && b[2].is_ascii_digit()
    && b[3].is_ascii_digit()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn qso(call: &str, freq: &str, mode: &str, grid: &str, confirmed: bool) -> LogEntry {
    LogEntry {
      callsign: call.into(),
      freq: freq.into(),
      mode: mode.into(),
      gridsquare: grid.into(),
      qsl_rcvd: confirmed,
      ..Default::default()
    }
  }

  #[test]
  fn counts_dxcc_waz_wac() {
    let log = [
      qso("JA1X", "14.074", "FT8", "", true),
      qso("JA2Y", "7.010", "CW", "", false),
      qso("W1AW", "14.200", "SSB", "", false),
      qso("DL1ABC", "14.074", "FT8", "", true),
      qso("QQ1X", "14.074", "FT8", "", true),
    ];
    let p = AwardProgress::from_entries(&log);
    assert_eq!(p.dxcc.worked.len(), 3);
    assert_eq!(p.dxcc.confirmed.len(), 2);
    assert_eq!(p.dxcc_by_band["20m"].worked.len(), 3);
    assert_eq!(p.dxcc_by_band["40m"].worked.len(), 1);
    assert_eq!(p.dxcc_by_mode["CW"].worked.len(), 1);
    assert_eq!(p.dxcc_by_mode["Phone"].worked.len(), 1);
    assert_eq!(p.dxcc_by_mode["Digital"].confirmed.len(), 2);
    assert_eq!(p.waz.worked, BTreeSet::from([5, 14, 25]));
    assert_eq!(p.wac.worked, BTreeSet::from(["AS", "EU", "NA"]));
    assert_eq!(p.wac.confirmed, BTreeSet::from(["AS", "EU"]));
  }

  #[test]
  fn explicit_zone_overrides_entity() {
    let mut e = qso("W6XYZ", "14.074", "FT8", "", false);
    e.cqz = "3".into();
    assert_eq!(
      AwardProgress::from_entries(&[e]).waz.worked,
      BTreeSet::from([3])
    );
  }

  #[test]
  fn vucc_counts_vhf_grids_and_satellites_separately() {
    let mut sat = qso("BG1AA", "145.900", "FM", "OM89ab", true);
    sat.prop_mode = "SAT".into();
    let log = [
      qso("BG1AA", "50.313", "FT8", "OM89", false),
      qso("BG1BB", "50.313", "FT8", "om89xx", true),
      qso("BG1CC", "50.313", "FT8", "PM01", false),
      qso("BG1DD", "14.074", "FT8", "PM01", false),
      qso("BG1EE", "50.313", "FT8", "bad!", false),
      sat,
    ];
    let p = AwardProgress::from_entries(&log);
    assert_eq!(
      p.vucc["6m"].worked,
      BTreeSet::from(["OM89".to_owned(), "PM01".to_owned()])
    );
    assert_eq!(p.vucc["6m"].confirmed.len(), 1);
    assert_eq!(p.vucc["SAT"].confirmed.len(), 1);
    assert!(!p.vucc.contains_key("20m") && !p.vucc.contains_key("2m"));
    assert_eq!(vucc_target("6m"), Some(100));
    assert_eq!(vucc_target("20m"), None);
  }
}
