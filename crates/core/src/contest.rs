//! 竞赛录入：常见竞赛的交换格式、查重、记分（自报分数）与 Cabrillo 3.0 导出。
//!
//! 记分按 DX 一方（中国台站）的视角简化实现，最终成绩以主办方核对为准。

use std::collections::HashSet;

use crate::dxcc::{Entity, lookup};
use crate::logbook::LogEntry;

/// 交换内容。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exch {
  /// CQ 分区（1–40）。
  CqZone,
  /// 通联序号。
  Serial,
  /// 发射功率（ARRL DX 中 DX 方发送）。
  Power,
  /// 美国州 / 加拿大省（ARRL DX 中 W/VE 方发送）。
  StateProvince,
  /// 任意文本。
  Free,
}

impl Exch {
  #[must_use]
  pub const fn label(self) -> &'static str {
    match self {
      Self::CqZone => "CQ 分区",
      Self::Serial => "序号",
      Self::Power => "功率",
      Self::StateProvince => "州 / 省",
      Self::Free => "交换",
    }
  }

  #[must_use]
  pub const fn placeholder(self) -> &'static str {
    match self {
      Self::CqZone => "24",
      Self::Serial => "001",
      Self::Power => "100",
      Self::StateProvince => "CA",
      Self::Free => "",
    }
  }
}

/// 记分规则。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scoring {
  /// CQ WW：同实体 0 分、同洲 1 分（北美之间 2 分）、跨洲 3 分；乘数 = 各波段分区数 + 各波段实体数。
  CqWw,
  /// CQ WPX：跨洲 3 分（160/80/40m 6 分）、同洲异国 1 分（低波段 2 分）、同国 1 分；乘数 = 不同前缀数。
  Wpx,
  /// ARRL DX（DX 方）：只有与 W/VE 的通联计分，每个 3 分；乘数 = 各波段州 / 省数。
  ArrlDx,
  /// 每个通联 1 分，无乘数。
  Simple,
  /// 每个通联 1 分、乘数 = 各波段 DXCC 实体数（All Asian、WAE 等）。
  DxccPerBand,
  /// 每个通联 1 分、乘数 = 各波段 CQ 分区数（JIDX、俄罗斯 DX 等简化）。
  ZoneMults,
}

/// 一个竞赛。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContestDef {
  /// Cabrillo `CONTEST` 值，同时写入日志的 `CONTEST_ID`。
  pub id: &'static str,
  pub name: &'static str,
  /// 限定模式（`CW` / `SSB`），`None` 为混合。
  pub mode: Option<&'static str>,
  pub sent: Exch,
  pub rcvd: Exch,
  pub scoring: Scoring,
}

pub const CONTESTS: &[ContestDef] = &[
  ContestDef {
    id: "CQ-WW-CW",
    name: "CQ WW DX CW",
    mode: Some("CW"),
    sent: Exch::CqZone,
    rcvd: Exch::CqZone,
    scoring: Scoring::CqWw,
  },
  ContestDef {
    id: "CQ-WW-SSB",
    name: "CQ WW DX SSB",
    mode: Some("SSB"),
    sent: Exch::CqZone,
    rcvd: Exch::CqZone,
    scoring: Scoring::CqWw,
  },
  ContestDef {
    id: "CQ-WPX-CW",
    name: "CQ WPX CW",
    mode: Some("CW"),
    sent: Exch::Serial,
    rcvd: Exch::Serial,
    scoring: Scoring::Wpx,
  },
  ContestDef {
    id: "CQ-WPX-SSB",
    name: "CQ WPX SSB",
    mode: Some("SSB"),
    sent: Exch::Serial,
    rcvd: Exch::Serial,
    scoring: Scoring::Wpx,
  },
  ContestDef {
    id: "ARRL-DX-CW",
    name: "ARRL International DX CW",
    mode: Some("CW"),
    sent: Exch::Power,
    rcvd: Exch::StateProvince,
    scoring: Scoring::ArrlDx,
  },
  ContestDef {
    id: "ARRL-DX-SSB",
    name: "ARRL International DX SSB",
    mode: Some("SSB"),
    sent: Exch::Power,
    rcvd: Exch::StateProvince,
    scoring: Scoring::ArrlDx,
  },
  ContestDef {
    id: "OTHER",
    name: "其他竞赛（序号交换）",
    mode: None,
    sent: Exch::Serial,
    rcvd: Exch::Free,
    scoring: Scoring::Simple,
  },
  ContestDef {
    id: "ALL-ASIAN-CW",
    name: "All Asian DX CW",
    mode: Some("CW"),
    sent: Exch::Serial,
    rcvd: Exch::Serial,
    scoring: Scoring::DxccPerBand,
  },
  ContestDef {
    id: "ALL-ASIAN-SSB",
    name: "All Asian DX SSB",
    mode: Some("SSB"),
    sent: Exch::Serial,
    rcvd: Exch::Serial,
    scoring: Scoring::DxccPerBand,
  },
  ContestDef {
    id: "JIDX-CW",
    name: "JIDX CW",
    mode: Some("CW"),
    sent: Exch::CqZone,
    rcvd: Exch::Free,
    scoring: Scoring::ZoneMults,
  },
  ContestDef {
    id: "JIDX-SSB",
    name: "JIDX SSB",
    mode: Some("SSB"),
    sent: Exch::CqZone,
    rcvd: Exch::Free,
    scoring: Scoring::ZoneMults,
  },
  ContestDef {
    id: "WAE-CW",
    name: "WAE DX CW",
    mode: Some("CW"),
    sent: Exch::Serial,
    rcvd: Exch::Serial,
    scoring: Scoring::DxccPerBand,
  },
  ContestDef {
    id: "WAE-SSB",
    name: "WAE DX SSB",
    mode: Some("SSB"),
    sent: Exch::Serial,
    rcvd: Exch::Serial,
    scoring: Scoring::DxccPerBand,
  },
  ContestDef {
    id: "IARU-HF",
    name: "IARU HF 锦标赛",
    mode: None,
    sent: Exch::Free,
    rcvd: Exch::Free,
    scoring: Scoring::ZoneMults,
  },
  ContestDef {
    id: "RUSSIAN-DX",
    name: "俄罗斯 DX",
    mode: None,
    sent: Exch::Serial,
    rcvd: Exch::CqZone,
    scoring: Scoring::ZoneMults,
  },
];

/// 按 ID 查找竞赛。
#[must_use]
pub fn contest(id: &str) -> Option<&'static ContestDef> {
  CONTESTS.iter().find(|c| c.id == id)
}

/// 模式大类：`CW` / `PH` / `DG`（也是 Cabrillo 的模式代码，RTTY 为 `RY`）。
#[must_use]
pub fn mode_code(mode: &str) -> &'static str {
  match mode.trim().to_ascii_uppercase().as_str() {
    "CW" => "CW",
    "SSB" | "AM" | "FM" | "USB" | "LSB" => "PH",
    "RTTY" => "RY",
    _ => "DG",
  }
}

/// 默认信号报告：CW / 数字 599，话音 59。
#[must_use]
pub fn default_rst(mode: &str) -> &'static str {
  if mode_code(mode) == "PH" { "59" } else { "599" }
}

/// CQ WPX 前缀：呼号主体中到最后一个数字为止的部分；没有数字时取前两个字母加 `0`。
/// `W1AW/3` → `W3`，`PA/W1AW` → `PA0`，`/P`、`/MM` 等后缀忽略。
#[must_use]
pub fn wpx_prefix(call: &str) -> Option<String> {
  let call = call.trim().to_ascii_uppercase();
  let parts: Vec<&str> = call.split('/').filter(|p| !p.is_empty()).collect();
  let ignored = ["P", "M", "MM", "AM", "QRP", "A", "LH"];
  let named: Vec<&str> = parts
    .iter()
    .copied()
    .filter(|p| !ignored.contains(p) && !(p.len() == 1 && p.chars().all(|c| c.is_ascii_digit())))
    .collect();
  // 两段时较短的一段是前缀标识（等长时取前面那段，如 `VP2E/W1AW`）
  let (base, designator) = match named.as_slice() {
    [] => return None,
    [one] => (*one, None),
    [a, b, ..] if a.len() <= b.len() => (*b, Some(*a)),
    [a, b, ..] => (*a, Some(*b)),
  };
  let lead = |s: &str| -> String {
    match s.rfind(|c: char| c.is_ascii_digit()) {
      Some(i) if i > 0 || s.len() == 1 => s[..=i].to_owned(),
      Some(_) | None => format!("{}0", s.get(..2.min(s.len())).unwrap_or(s)),
    }
  };
  // 单个数字后缀：换区
  if let Some(d) = parts
    .iter()
    .find(|p| p.len() == 1 && p.chars().all(|c| c.is_ascii_digit()))
  {
    let p = lead(base);
    return Some(format!(
      "{}{d}",
      p.trim_end_matches(|c: char| c.is_ascii_digit())
    ));
  }
  Some(lead(designator.unwrap_or(base)))
}

/// 同一竞赛里判断重复的 key：呼号 + 波段（混合模式竞赛再加模式大类）。
fn dupe_key(def: &ContestDef, e: &LogEntry) -> String {
  let mode = if def.mode.is_none() {
    mode_code(&e.mode)
  } else {
    ""
  };
  format!(
    "{}|{}|{mode}",
    e.callsign.trim().to_ascii_uppercase(),
    e.band_label()
  )
}

/// 是否与已录入的通联重复。
#[must_use]
pub fn is_dupe(def: &ContestDef, entries: &[LogEntry], candidate: &LogEntry) -> bool {
  let key = dupe_key(def, candidate);
  entries
    .iter()
    .filter(|e| e.id != candidate.id)
    .any(|e| dupe_key(def, e) == key)
}

/// 下一个发出的序号（已录入通联数 + 1）。
#[must_use]
pub fn next_serial(entries: &[LogEntry]) -> u32 {
  entries.len() as u32 + 1
}

/// 自报分数。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Score {
  /// 有效通联（不含重复）。
  pub qsos: u32,
  pub dupes: u32,
  pub points: u32,
  pub mults: u32,
  /// 总分：有乘数的竞赛为点数 × 乘数，否则为点数。
  pub total: u32,
}

fn low_band(band: &str) -> bool {
  matches!(band, "160m" | "80m" | "40m")
}

/// 美国 / 加拿大（ARRL DX 计分对象）。
fn is_w_ve(e: Option<&Entity>) -> bool {
  e.is_some_and(|e| matches!(e.dxcc, 291 | 1))
}

/// 计算自报分数（按时间顺序，重复通联不计分）。`my_call` 为本台呼号。
#[must_use]
pub fn score(def: &ContestDef, entries: &[LogEntry], my_call: &str) -> Score {
  let me = lookup(my_call);
  let mut sorted: Vec<&LogEntry> = entries.iter().collect();
  sorted.sort_by(|a, b| (&a.date, &a.time, a.id).cmp(&(&b.date, &b.time, b.id)));
  let mut seen = HashSet::new();
  let mut mults: HashSet<String> = HashSet::new();
  let mut s = Score::default();
  for e in sorted {
    if !seen.insert(dupe_key(def, e)) {
      s.dupes += 1;
      continue;
    }
    s.qsos += 1;
    let them = e.entity();
    let band = e.band_label();
    let same_entity = matches!((me, them), (Some(a), Some(b)) if a.dxcc == b.dxcc);
    let same_cont = matches!((me, them), (Some(a), Some(b)) if a.continent == b.continent);
    match def.scoring {
      Scoring::CqWw => {
        s.points += if same_entity {
          0
        } else if same_cont {
          if me.is_some_and(|m| m.continent == "NA") {
            2
          } else {
            1
          }
        } else if them.is_some() {
          3
        } else {
          0
        };
        let zone = e
          .srx
          .trim()
          .parse::<u8>()
          .ok()
          .filter(|z| (1..=40).contains(z))
          .or_else(|| e.cq_zone());
        if let Some(z) = zone {
          mults.insert(format!("Z|{band}|{z}"));
        }
        if let Some(t) = them {
          mults.insert(format!("E|{band}|{}", t.dxcc));
        }
      }
      Scoring::Wpx => {
        let low = low_band(&band);
        s.points += if them.is_none() {
          0
        } else if !same_cont {
          if low { 6 } else { 3 }
        } else if !same_entity && low {
          2
        } else {
          1
        };
        if let Some(p) = wpx_prefix(&e.callsign) {
          mults.insert(p);
        }
      }
      Scoring::ArrlDx => {
        if is_w_ve(them) {
          s.points += 3;
          let sp = e.srx.trim().to_ascii_uppercase();
          if !sp.is_empty() {
            mults.insert(format!("{band}|{sp}"));
          }
        }
      }
      Scoring::Simple => s.points += 1,
      Scoring::DxccPerBand => {
        s.points += u32::from(them.is_some());
        if let Some(t) = them {
          mults.insert(format!("E|{band}|{}", t.dxcc));
        }
      }
      Scoring::ZoneMults => {
        s.points += u32::from(them.is_some());
        let zone = e
          .srx
          .trim()
          .parse::<u8>()
          .ok()
          .filter(|z| (1..=40).contains(z))
          .or_else(|| e.cq_zone());
        if let Some(z) = zone {
          mults.insert(format!("Z|{band}|{z}"));
        }
      }
    }
  }
  s.mults = mults.len() as u32;
  s.total = if def.scoring == Scoring::Simple {
    s.points
  } else {
    s.points * s.mults
  };
  s
}

/// 频率（kHz）列：有频率用频率，否则用波段下沿；VHF 以上按 Cabrillo 约定写 `50`、`144` 等。
fn cabrillo_freq(e: &LogEntry) -> String {
  if let Ok(mhz) = e.freq.trim().parse::<f64>()
    && mhz < 30.0
  {
    return format!("{:.0}", mhz * 1000.0);
  }
  match e.band_label().as_str() {
    "160m" => "1800",
    "80m" => "3500",
    "40m" => "7000",
    "20m" => "14000",
    "15m" => "21000",
    "10m" => "28000",
    "6m" => "50",
    "2m" => "144",
    "70cm" => "432",
    _ => "0",
  }
  .to_owned()
}

/// Cabrillo 头信息。
#[derive(Debug, Clone, Default)]
pub struct CabrilloHeader {
  pub callsign: String,
  pub operators: String,
  pub name: String,
  pub location: String,
  pub grid: String,
  /// `HIGH` / `LOW` / `QRP`。
  pub power: String,
  pub soapbox: String,
}

/// 生成 Cabrillo 3.0 日志。
#[must_use]
pub fn cabrillo(def: &ContestDef, h: &CabrilloHeader, entries: &[LogEntry]) -> String {
  let call = h.callsign.trim().to_ascii_uppercase();
  let score = score(def, entries, &call);
  let mode = def.mode.unwrap_or("MIXED");
  let mut s = String::new();
  let mut line = |k: &str, v: &str| s.push_str(&format!("{k}: {v}\n").replace(": \n", ":\n"));
  line("START-OF-LOG", "3.0");
  line("CREATED-BY", "HamExamWeb");
  line("CONTEST", def.id);
  line("CALLSIGN", &call);
  line("CATEGORY-OPERATOR", "SINGLE-OP");
  line("CATEGORY-ASSISTED", "NON-ASSISTED");
  line("CATEGORY-BAND", "ALL");
  line(
    "CATEGORY-MODE",
    if mode == "CW" {
      "CW"
    } else if mode == "SSB" {
      "SSB"
    } else {
      "MIXED"
    },
  );
  line(
    "CATEGORY-POWER",
    if h.power.is_empty() {
      "LOW"
    } else {
      h.power.as_str()
    },
  );
  line("CATEGORY-TRANSMITTER", "ONE");
  line("CLAIMED-SCORE", &score.total.to_string());
  line("GRID-LOCATOR", h.grid.trim());
  line("LOCATION", h.location.trim());
  line("NAME", h.name.trim());
  line(
    "OPERATORS",
    if h.operators.trim().is_empty() {
      &call
    } else {
      h.operators.trim()
    },
  );
  line("SOAPBOX", h.soapbox.trim());
  let mut sorted: Vec<&LogEntry> = entries.iter().collect();
  sorted.sort_by(|a, b| (&a.date, &a.time, a.id).cmp(&(&b.date, &b.time, b.id)));
  for e in sorted {
    let rst_s = if e.rst_sent.trim().is_empty() {
      default_rst(&e.mode)
    } else {
      e.rst_sent.trim()
    };
    let rst_r = if e.rst_rcvd.trim().is_empty() {
      default_rst(&e.mode)
    } else {
      e.rst_rcvd.trim()
    };
    let x = |v: &str| {
      if v.trim().is_empty() {
        "-".to_owned()
      } else {
        v.trim().to_ascii_uppercase()
      }
    };
    s.push_str(&format!(
      "QSO: {:>5} {} {} {} {:<13} {:>3} {:<6} {:<13} {:>3} {:<6} 0\n",
      cabrillo_freq(e),
      mode_code(&e.mode),
      e.date,
      e.time.replace(':', "").get(..4).unwrap_or("0000"),
      call,
      rst_s,
      x(&e.stx),
      e.callsign.trim().to_ascii_uppercase(),
      rst_r,
      x(&e.srx),
    ));
  }
  s.push_str("END-OF-LOG:\n");
  s
}

#[cfg(test)]
mod tests {
  use super::*;

  fn qso(id: u64, call: &str, freq: &str, srx: &str) -> LogEntry {
    LogEntry {
      id,
      date: "2026-10-31".into(),
      time: format!("00:{id:02}"),
      freq: freq.into(),
      mode: "CW".into(),
      callsign: call.into(),
      stx: "24".into(),
      srx: srx.into(),
      ..Default::default()
    }
  }

  #[test]
  fn wpx_prefixes() {
    for (call, p) in [
      ("BG4ABC", "BG4"),
      ("W1AW", "W1"),
      ("N8ZZ", "N8"),
      ("KP4XX", "KP4"),
      ("9A1AA", "9A1"),
      ("4X4AA", "4X4"),
      ("W1AW/3", "W3"),
      ("PA/W1AW", "PA0"),
      ("VP2E/W1AW", "VP2"),
      ("RAEM", "RA0"),
      ("DL1ABC/P", "DL1"),
    ] {
      assert_eq!(wpx_prefix(call).as_deref(), Some(p), "{call}");
    }
  }

  #[test]
  fn detects_dupes_per_band() {
    let def = contest("CQ-WW-CW").unwrap();
    let log = vec![qso(1, "JA1ABC", "14.020", "25")];
    assert!(is_dupe(def, &log, &qso(2, "ja1abc", "14.030", "25")));
    assert!(!is_dupe(def, &log, &qso(2, "JA1ABC", "7.010", "25")));
    // 编辑自身不算重复
    assert!(!is_dupe(def, &log, &qso(1, "JA1ABC", "14.020", "25")));
  }

  #[test]
  fn cq_ww_score_from_china() {
    let def = contest("CQ-WW-CW").unwrap();
    let log = vec![
      qso(1, "JA1ABC", "14.020", "25"), // 同洲异国 1 分
      qso(2, "W1AW", "14.021", "5"),    // 跨洲 3 分
      qso(3, "BG1XYZ", "14.022", "24"), // 同实体 0 分
      qso(4, "W1AW", "14.023", "5"),    // 重复
      qso(5, "W1AW", "7.010", "5"),     // 另一波段 3 分
    ];
    let s = score(def, &log, "BG4ABC");
    assert_eq!((s.qsos, s.dupes, s.points), (4, 1, 7));
    // 分区：20m 25/5/24 + 40m 5 = 4；实体：20m 日本/美国/中国 + 40m 美国 = 4
    assert_eq!(s.mults, 8);
    assert_eq!(s.total, 56);
  }

  #[test]
  fn wpx_and_arrl_scores() {
    let wpx = contest("CQ-WPX-CW").unwrap();
    let log = vec![
      qso(1, "W1AW", "7.010", "1"),
      qso(2, "JA1ABC", "14.020", "2"),
    ];
    let s = score(wpx, &log, "BG4ABC");
    assert_eq!((s.points, s.mults, s.total), (6 + 1, 2, 14));

    let arrl = contest("ARRL-DX-CW").unwrap();
    let log = vec![
      qso(1, "W1AW", "14.020", "CT"),
      qso(2, "VE3ABC", "14.021", "ON"),
      qso(3, "JA1ABC", "14.022", ""),
    ];
    let s = score(arrl, &log, "BG4ABC");
    assert_eq!((s.points, s.mults, s.total), (6, 2, 12));

    let other = contest("OTHER").unwrap();
    assert_eq!(score(other, &log, "BG4ABC").total, 3);
  }

  #[test]
  fn dxcc_per_band_and_zone_mults() {
    let aa = contest("ALL-ASIAN-CW").unwrap();
    let log = vec![
      qso(1, "W1AW", "14.020", "001"),
      qso(2, "JA1ABC", "14.021", "002"),
      qso(3, "W1AW", "7.010", "003"),
    ];
    let s = score(aa, &log, "BG4ABC");
    // 3 分；乘数 = 20m 美国/日本 + 40m 美国 = 3
    assert_eq!((s.points, s.mults, s.total), (3, 3, 9));

    let jidx = contest("JIDX-CW").unwrap();
    let log = vec![
      qso(1, "JA1ABC", "14.020", "10"),
      qso(2, "JA2DEF", "14.021", "11"),
      qso(3, "JA3GHI", "7.010", "10"),
    ];
    let s = score(jidx, &log, "BG4ABC");
    // 3 分；乘数 = 20m 10/11 + 40m 10 = 3
    assert_eq!((s.points, s.mults, s.total), (3, 3, 9));
  }

  #[test]
  fn cabrillo_lines() {
    let def = contest("CQ-WW-CW").unwrap();
    let h = CabrilloHeader {
      callsign: "bg4abc".into(),
      ..Default::default()
    };
    let text = cabrillo(def, &h, &[qso(1, "W1AW", "14.021", "5")]);
    assert!(text.starts_with("START-OF-LOG: 3.0\n"));
    assert!(text.contains("CONTEST: CQ-WW-CW\n"));
    assert!(text.contains("CLAIMED-SCORE: 6\n"));
    assert!(text.contains("SOAPBOX:\n"));
    assert!(
      text.contains(
        "QSO: 14021 CW 2026-10-31 0001 BG4ABC        599 24     W1AW          599 5      0\n"
      ),
      "{text}"
    );
    assert!(text.ends_with("END-OF-LOG:\n"));
  }
}
