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
/// WAS：美国 50 个州。
pub const WAS_TARGET: usize = 50;
/// IOTA 基础奖要求的岛屿组数。
pub const IOTA_TARGET: usize = 100;
/// WPX 基础奖要求的前缀数。
pub const WPX_TARGET: usize = 400;
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
  /// WPX 前缀奖。
  pub wpx: Progress<String>,
  /// WAS：美国州。
  pub was: Progress<String>,
  /// IOTA：岛屿组编号。
  pub iota: Progress<String>,
  /// DXCC Challenge：各波段 DXCC 实体数之和。
  pub dxcc_challenge: usize,
}

impl AwardProgress {
  /// 从日志统计。
  #[must_use]
  pub fn from_entries(entries: &[LogEntry]) -> Self {
    let mut p = Self::default();
    for e in entries {
      let ok = e.confirmed();
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
      if let Some(prefix) = wpx_prefix(&e.callsign) {
        p.wpx.add(prefix, ok);
      }
      if !e.state.trim().is_empty() {
        p.was.add(e.state.trim().to_ascii_uppercase(), ok);
      }
      if !e.iota.trim().is_empty() {
        p.iota.add(e.iota.trim().to_ascii_uppercase(), ok);
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
    p.dxcc_challenge = p.dxcc_by_band.values().map(|x| x.worked.len()).sum();
    p
  }
}

/// 一项奖状的冲刺缺口（还差多少达标）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AwardGap {
  /// 奖状缩写（DXCC / WAZ / WAC / WAS / IOTA / WPX）。
  pub label: &'static str,
  /// 当前已通联（或已确认）数量。
  pub current: usize,
  /// 达标所需数量。
  pub target: usize,
}

impl AwardGap {
  /// 还差多少达标。
  #[must_use]
  pub fn remaining(&self) -> usize {
    self.target.saturating_sub(self.current)
  }
}

/// 未达标奖状的冲刺缺口，按「还差最少」升序（越接近达标的越靠前）。
///
/// `confirmed` 为真时按「已确认（QSL）」统计，否则按「已通联」统计。
#[must_use]
pub fn award_gaps(p: &AwardProgress, confirmed: bool) -> Vec<AwardGap> {
  let worked = |x: &Progress<u16>| {
    if confirmed {
      x.confirmed.len()
    } else {
      x.worked.len()
    }
  };
  let count = |x: &Progress<u8>| {
    if confirmed {
      x.confirmed.len()
    } else {
      x.worked.len()
    }
  };
  let str_count = |x: &Progress<&str>| {
    if confirmed {
      x.confirmed.len()
    } else {
      x.worked.len()
    }
  };
  let str_count_owned = |x: &Progress<String>| {
    if confirmed {
      x.confirmed.len()
    } else {
      x.worked.len()
    }
  };

  let mut out = Vec::new();
  let dxcc = worked(&p.dxcc);
  if dxcc < DXCC_TARGET {
    out.push(AwardGap {
      label: "DXCC",
      current: dxcc,
      target: DXCC_TARGET,
    });
  }
  let waz = count(&p.waz);
  if waz < WAZ_TARGET {
    out.push(AwardGap {
      label: "WAZ",
      current: waz,
      target: WAZ_TARGET,
    });
  }
  let wac = str_count(&p.wac);
  if wac < CONTINENTS.len() {
    out.push(AwardGap {
      label: "WAC",
      current: wac,
      target: CONTINENTS.len(),
    });
  }
  let was = str_count_owned(&p.was);
  if was < WAS_TARGET {
    out.push(AwardGap {
      label: "WAS",
      current: was,
      target: WAS_TARGET,
    });
  }
  let iota = str_count_owned(&p.iota);
  if iota < IOTA_TARGET {
    out.push(AwardGap {
      label: "IOTA",
      current: iota,
      target: IOTA_TARGET,
    });
  }
  let wpx = str_count_owned(&p.wpx);
  if wpx < WPX_TARGET {
    out.push(AwardGap {
      label: "WPX",
      current: wpx,
      target: WPX_TARGET,
    });
  }

  out.sort_by_key(AwardGap::remaining);
  out
}

/// 未通联的 DXCC 实体，按大洲分组（依 [`CONTINENTS`] 顺序，仅返回有缺口的洲）。
///
/// 用于「DXCC 缺口清单」：一眼看清还差哪些实体、各洲还差多少。
#[must_use]
pub fn dxcc_missing(p: &AwardProgress) -> Vec<(&'static str, Vec<&'static crate::dxcc::Entity>)> {
  let entities = crate::dxcc::entities();
  CONTINENTS
    .iter()
    .filter_map(|(code, name)| {
      let missing: Vec<&'static crate::dxcc::Entity> = entities
        .iter()
        .filter(|e| e.continent == *code && !p.dxcc.worked.contains(&e.dxcc))
        .collect();
      (!missing.is_empty()).then_some((*name, missing))
    })
    .collect()
}

/// 提取呼号的 WPX 前缀（用于前缀奖统计）。
///
/// 简化规则：取第一个含数字的斜杠分段为主呼号；前缀由开头字母 + 第一个数字组成，
/// 若以数字开头则延伸到第二个数字（如 `JA1X`→`JA1`、`W1AW`→`W1`、`3D2AG`→`3D2`）。
#[must_use]
pub fn wpx_prefix(callsign: &str) -> Option<String> {
  let seg = callsign
    .split('/')
    .map(str::trim)
    .find(|s| s.chars().any(|c| c.is_ascii_digit()))?;
  let chars: Vec<char> = seg.chars().collect();
  let digits: Vec<usize> = chars
    .iter()
    .enumerate()
    .filter(|(_, c)| c.is_ascii_digit())
    .map(|(i, _)| i)
    .collect();
  let end = if digits.is_empty() {
    chars.len()
  } else if digits[0] > 0 {
    digits[0] + 1
  } else if digits.len() >= 2 {
    digits[1] + 1
  } else {
    chars.len()
  };
  Some(chars[..end].iter().collect::<String>().to_ascii_uppercase())
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

  #[test]
  fn wpx_prefix_rules() {
    assert_eq!(wpx_prefix("JA1X").as_deref(), Some("JA1"));
    assert_eq!(wpx_prefix("W1AW").as_deref(), Some("W1"));
    assert_eq!(wpx_prefix("VP2E").as_deref(), Some("VP2"));
    assert_eq!(wpx_prefix("3D2AG").as_deref(), Some("3D2"));
    assert_eq!(wpx_prefix("4X4JU").as_deref(), Some("4X4"));
    assert_eq!(wpx_prefix("DL1ABC").as_deref(), Some("DL1"));
    assert_eq!(wpx_prefix("K1AA/P").as_deref(), Some("K1"));
    assert_eq!(wpx_prefix("VP2E/W1AW").as_deref(), Some("VP2"));
    assert_eq!(wpx_prefix("N0CALL").as_deref(), Some("N0"));
    assert_eq!(wpx_prefix("NOCALL"), None);
  }

  #[test]
  fn counts_wpx_was_iota_and_dxcc_challenge() {
    let mut e1 = qso("JA1X", "14.074", "FT8", "", true);
    e1.state = "CT".into();
    e1.iota = "NA-046".into();
    let mut e2 = qso("W1AW", "7.010", "CW", "", false);
    e2.state = "CT".into();
    let mut e3 = qso("DL1ABC", "14.074", "FT8", "", false);
    e3.iota = "EU-004".into();
    let p = AwardProgress::from_entries(&[e1, e2, e3]);
    assert_eq!(
      p.wpx.worked,
      BTreeSet::from(["JA1".to_owned(), "W1".to_owned(), "DL1".to_owned()])
    );
    assert_eq!(p.wpx.confirmed, BTreeSet::from(["JA1".to_owned()]));
    assert_eq!(p.was.worked, BTreeSet::from(["CT".to_owned()]));
    assert_eq!(
      p.iota.worked,
      BTreeSet::from(["NA-046".to_owned(), "EU-004".to_owned()])
    );
    // 20m 有 JA1X + DL1ABC 两个实体，40m 有 W1AW 一个实体 → challenge = 3
    assert_eq!(p.dxcc_challenge, 3);
  }

  #[test]
  fn award_gaps_sorts_by_remaining() {
    let mut p = AwardProgress::default();
    for z in 1..=39u8 {
      p.waz.worked.insert(z); // 39 分区，还差 1 个
    }
    for d in 1..=50u16 {
      p.dxcc.worked.insert(d); // 50 实体，还差 50 个
    }
    let gaps = award_gaps(&p, false);
    assert_eq!(gaps[0].label, "WAZ");
    assert_eq!(gaps[0].remaining(), 1);
    assert!(
      gaps
        .iter()
        .any(|g| g.label == "DXCC" && g.remaining() == 50)
    );

    // 全部达标时返回空。
    for z in 1..=40u8 {
      p.waz.worked.insert(z);
    }
    assert!(award_gaps(&p, false).iter().all(|g| g.label != "WAZ"));
  }

  #[test]
  fn dxcc_missing_groups_and_excludes_worked() {
    let mut p = AwardProgress::default();
    let entities = crate::dxcc::entities();
    for e in entities.iter().take(10) {
      p.dxcc.worked.insert(e.dxcc);
    }
    let missing = dxcc_missing(&p);
    let total_missing: usize = missing.iter().map(|(_, v)| v.len()).sum();
    assert_eq!(total_missing, entities.len() - 10);
    for (name, ents) in &missing {
      let code = CONTINENTS
        .iter()
        .find(|(_, n)| n == name)
        .map(|(c, _)| *c)
        .expect("continent code");
      assert!(ents.iter().all(|e| e.continent == code));
      assert!(ents.iter().all(|e| !p.dxcc.worked.contains(&e.dxcc)));
    }
  }
}
