//! 野外激活导出：把日志里的通联整理成 SOTA CSV V2 与 POTA ADIF 提交文件。
//!
//! **语义约定**：取记录里的 [`LogEntry::my_sota_ref`] / [`LogEntry::my_pota_ref`] —— **本台**所在的
//! 山峰 / 公园（ADIF 的 `MY_SOTA_REF` / `MY_POTA_REF`）。对方的编号（`SOTA_REF` / `POTA_REF`，S2S / P2P）
//! 是另外两个字段，这里不用。
//!
//! 一次激活 = 一个山峰 / 公园 × 一个 **UTC 日**，所以每个 UTC 日各出一个文件，方便逐次上传、
//! 逐次核对通联数。
//!
//! 已知边界（**没有对着平台官方文档逐条核对**，下面都是我对公开格式的了解，上传前请先用平台的
//! 校验结果为准）：
//! - SOTA CSV V2：`V2,本台呼号,山峰,DD/MM/YYYY,HHMM,频率MHz,模式,对方呼号,对方山峰,备注`，无表头，
//!   模式取 `AM / CW / Data / DV / FM / SSB / Other`；
//! - POTA ADIF：每条写 `MY_SIG=POTA`、`MY_SIG_INFO=公园编号`；只支持单个公园（不做 two-fer 逗号列表）、
//!   不写对方公园（P2P）；ADIF 规范本身（`MY_SIG` / `MY_SIG_INFO` / `MY_POTA_REF` / `MY_SOTA_REF` 的存在
//!   与含义）已对照 ADIF 3.1.4 官方文档核对过；
//!   对方的山峰 / 公园（S2S / P2P）日志里已经有字段，但本模块没有用它 —— SOTA CSV 的「对方山峰」列仍留空；
//! - 编号只校验**格式**，不查它是否真实存在；
//! - 缺日期时间 / 呼号 / 频率（SOTA）/ 波段（POTA）、以及 POTA 里模式为本站兜底分类 `DATA` / `OTHER`
//!   （不是合法 ADIF 模式）的通联整条跳过并计数，不替用户编一个值。

use std::collections::BTreeMap;

use crate::adif::to_adif_mode;
use crate::logbook::{LogEntry, adif_field, stamp};
use crate::station::StationBook;

/// POTA 一次有效激活（单个 UTC 日）通常要求的最少通联数。
pub const POTA_MIN_QSOS: usize = 10;
/// SOTA 一次有效激活通常要求的最少通联数。
pub const SOTA_MIN_QSOS: usize = 4;

/// 目标平台。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
  Sota,
  Pota,
}

impl Platform {
  /// 一次有效激活所需的最少通联数。
  #[must_use]
  pub fn min_qsos(self) -> usize {
    match self {
      Self::Sota => SOTA_MIN_QSOS,
      Self::Pota => POTA_MIN_QSOS,
    }
  }

  /// 记录里对应的编号字段。
  fn entry_ref(self, e: &LogEntry) -> &str {
    match self {
      Self::Sota => &e.my_sota_ref,
      Self::Pota => &e.my_pota_ref,
    }
  }
}

/// 编号不可用的原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefError {
  Empty,
  /// 不符合该平台编号的格式（只校验格式，不查是否存在）。
  BadFormat,
}

/// 统一编号写法：去空白、转大写。
#[must_use]
pub fn normalize_ref(s: &str) -> String {
  s.trim().to_ascii_uppercase()
}

/// 与已规范化的编号比较（trim + 忽略大小写）：`check_ref` 的输出是大写去空白，
/// 这里避免为每条记录再分配一个 `normalize_ref` 的字符串。
fn eq_normalized(s: &str, reference: &str) -> bool {
  s.trim().eq_ignore_ascii_case(reference)
}

/// SOTA 编号格式：`协会/区域-三位数字`，如 `G/LD-001`、`BY/BJ-001`、`W7W/KG-001`。
#[must_use]
pub fn is_valid_sota_ref(s: &str) -> bool {
  let s = normalize_ref(s);
  let Some((assoc, rest)) = s.split_once('/') else {
    return false;
  };
  let Some((region, num)) = rest.split_once('-') else {
    return false;
  };
  (1..=4).contains(&assoc.len())
    && assoc.chars().all(|c| c.is_ascii_alphanumeric())
    && (2..=3).contains(&region.len())
    && region.chars().all(|c| c.is_ascii_alphanumeric())
    && num.len() == 3
    && num.chars().all(|c| c.is_ascii_digit())
}

/// POTA 编号格式：`前缀-四到五位数字`，如 `K-0001`、`CN-0123`、`VE-1234`。
#[must_use]
pub fn is_valid_pota_ref(s: &str) -> bool {
  let s = normalize_ref(s);
  let Some((prefix, num)) = s.split_once('-') else {
    return false;
  };
  (1..=4).contains(&prefix.len())
    && prefix.chars().all(|c| c.is_ascii_alphanumeric())
    && (4..=5).contains(&num.len())
    && num.chars().all(|c| c.is_ascii_digit())
}

/// 按平台校验并规范化编号。
pub fn check_ref(platform: Platform, s: &str) -> Result<String, RefError> {
  let r = normalize_ref(s);
  if r.is_empty() {
    return Err(RefError::Empty);
  }
  let ok = match platform {
    Platform::Sota => is_valid_sota_ref(&r),
    Platform::Pota => is_valid_pota_ref(&r),
  };
  if ok { Ok(r) } else { Err(RefError::BadFormat) }
}

/// 日志里出现过的、格式合法的编号及各自的通联数（按通联数降序，同数按编号升序）。
#[must_use]
pub fn candidates(entries: &[LogEntry], platform: Platform) -> Vec<(String, usize)> {
  let mut counts: BTreeMap<String, usize> = BTreeMap::new();
  for e in entries {
    if let Ok(r) = check_ref(platform, platform.entry_ref(e)) {
      *counts.entry(r).or_default() += 1;
    }
  }
  let mut v: Vec<_> = counts.into_iter().collect();
  v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
  v
}

/// 被跳过的通联，按原因计数。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Skipped {
  /// 日期或时间无法解析。
  pub bad_datetime: usize,
  /// 缺本台呼号 / 对方呼号，或缺这个平台必需的频率（SOTA）/ 波段（POTA）。
  pub missing_data: usize,
  /// POTA：模式是本站兜底分类（`DATA` / `OTHER`），不是合法 ADIF 模式。
  pub unsupported_mode: usize,
}

impl Skipped {
  #[must_use]
  pub fn total(&self) -> usize {
    self.bad_datetime + self.missing_data + self.unsupported_mode
  }
}

/// 一个待下载的文件（一个 UTC 日的一次激活）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportFile {
  pub name: String,
  pub content: String,
  /// UTC 日期 `YYYY-MM-DD`。
  pub date: String,
  pub qsos: usize,
}

/// 导出结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Export {
  /// 按日期升序，每个 UTC 日一个文件。
  pub files: Vec<ExportFile>,
  pub skipped: Skipped,
}

/// 文件名里不能有 `/` 等字符：呼号 `BG1ABC/P`、SOTA 编号 `BY/BJ-001` 都要折一下。
fn file_part(s: &str) -> String {
  s.chars()
    .map(|c| {
      if c.is_ascii_alphanumeric() || c == '-' {
        c
      } else {
        '_'
      }
    })
    .collect()
}

/// 本站模式 → SOTA 模式（`AM / CW / Data / DV / FM / SSB / Other`）。
fn sota_mode(mode: &str) -> &'static str {
  match mode.trim().to_ascii_uppercase().as_str() {
    "CW" => "CW",
    "SSB" | "USB" | "LSB" => "SSB",
    "AM" => "AM",
    "FM" => "FM",
    "DMR" | "C4FM" | "DSTAR" | "M17" | "FREEDV" => "DV",
    "RTTY" | "PSK31" | "PSK63" | "FT8" | "FT4" | "FST4" | "JS8" | "Q65" | "JT65" | "JT9"
    | "MSK144" | "WSPR" | "OLIVIA" | "DATA" => "Data",
    _ => "Other",
  }
}

/// 频率（MHz）文本 → `14.062MHz`；不是正的有限数时 `None`。最多保留 6 位小数并去掉多余的 0。
fn sota_freq(freq: &str) -> Option<String> {
  let f: f64 = freq.trim().parse().ok()?;
  if !(f.is_finite() && f > 0.0) {
    return None;
  }
  let s = format!("{f:.6}");
  let s = s.trim_end_matches('0').trim_end_matches('.');
  Some(format!("{s}MHz"))
}

/// 备注：SOTA 的 CSV 解析器是否认引号不得而知，所以不靠引号转义，直接把逗号换成分号、剔除控制字符。
fn sota_comment(s: &str) -> String {
  s.chars()
    .filter(|c| !c.is_control())
    .map(|c| if c == ',' { ';' } else { c })
    .collect::<String>()
    .trim()
    .to_owned()
}

/// 一条通过校验的待导出记录（时间已折成 UTC 日期 + HHMM）。
struct Row<'a> {
  entry: &'a LogEntry,
  date: String,
  hhmm: String,
  station: String,
  operator: String,
  grid: String,
  minutes: i64,
}

/// 把日志导出成目标平台的提交文件：只取记录里编号等于 `reference` 的通联，按 UTC 日拆成多个文件。
pub fn export(
  entries: &[LogEntry],
  stations: &StationBook,
  platform: Platform,
  reference: &str,
) -> Result<Export, RefError> {
  let reference = check_ref(platform, reference)?;
  let mut skipped = Skipped::default();
  let mut by_day: BTreeMap<String, Vec<Row<'_>>> = BTreeMap::new();

  for e in entries {
    if !eq_normalized(platform.entry_ref(e), &reference) {
      continue;
    }
    let Some(minutes) = e.minutes_on() else {
      skipped.bad_datetime += 1;
      continue;
    };
    let (date, time) = stamp(minutes);
    let profile = stations.of_entry(e);
    let station = profile.callsign.trim().to_ascii_uppercase();
    let worked = e.callsign.trim();
    let band_or_freq_missing = match platform {
      Platform::Sota => sota_freq(&e.freq).is_none(),
      Platform::Pota => e.band_label().is_empty(),
    };
    if station.is_empty() || worked.is_empty() || band_or_freq_missing {
      skipped.missing_data += 1;
      continue;
    }
    if platform == Platform::Pota
      && matches!(
        e.mode.trim().to_ascii_uppercase().as_str(),
        "DATA" | "OTHER"
      )
    {
      skipped.unsupported_mode += 1;
      continue;
    }
    let operator = if e.operator.trim().is_empty() {
      profile.operator.trim().to_owned()
    } else {
      e.operator.trim().to_owned()
    };
    by_day.entry(date.clone()).or_default().push(Row {
      entry: e,
      hhmm: time.replace(':', ""),
      date,
      station,
      operator,
      grid: profile.gridsquare.trim().to_owned(),
      minutes,
    });
  }

  let files = by_day
    .into_iter()
    .map(|(date, mut rows)| {
      // 稳定输出：先按时刻，再按 id（同一分钟内保持录入顺序）。
      rows.sort_by_key(|r| (r.minutes, r.entry.id));
      let compact = date.replace('-', "");
      let station_for_name = file_part(&rows[0].station);
      let (name, content) = match platform {
        Platform::Sota => (
          format!("sota-{}-{compact}.csv", file_part(&reference)),
          sota_csv(&rows, &reference),
        ),
        Platform::Pota => (
          format!("{station_for_name}@{}-{compact}.adi", file_part(&reference)),
          pota_adif(&rows, &reference),
        ),
      };
      ExportFile {
        name,
        content,
        qsos: rows.len(),
        date,
      }
    })
    .collect();

  Ok(Export { files, skipped })
}

/// `YYYY-MM-DD` → `DD/MM/YYYY`。
fn dmy(date: &str) -> String {
  let mut p = date.split('-');
  match (p.next(), p.next(), p.next()) {
    (Some(y), Some(m), Some(d)) => format!("{d}/{m}/{y}"),
    _ => date.to_owned(),
  }
}

fn sota_csv(rows: &[Row<'_>], summit: &str) -> String {
  let mut out = String::new();
  for r in rows {
    let e = r.entry;
    // `sota_freq` 已在筛选时校验过，这里不会落到 `unwrap_or_default`。
    let freq = sota_freq(&e.freq).unwrap_or_default();
    out.push_str(&format!(
      "V2,{},{summit},{},{},{freq},{},{},,{}\r\n",
      r.station,
      dmy(&r.date),
      r.hhmm,
      sota_mode(&e.mode),
      e.callsign.trim().to_ascii_uppercase(),
      sota_comment(&e.remark),
    ));
  }
  out
}

fn pota_adif(rows: &[Row<'_>], park: &str) -> String {
  let mut s = String::from("POTA activation\n");
  adif_field(&mut s, "ADIF_VER", "3.1.4");
  adif_field(&mut s, "PROGRAMID", "HamExamWeb");
  s.push_str("<EOH>\n");
  for r in rows {
    let e = r.entry;
    adif_field(&mut s, "STATION_CALLSIGN", &r.station);
    adif_field(&mut s, "OPERATOR", &r.operator);
    adif_field(&mut s, "MY_GRIDSQUARE", &r.grid);
    adif_field(&mut s, "CALL", &e.callsign.trim().to_ascii_uppercase());
    adif_field(&mut s, "QSO_DATE", &r.date.replace('-', ""));
    adif_field(&mut s, "TIME_ON", &r.hhmm);
    adif_field(&mut s, "BAND", &e.band_label());
    adif_field(&mut s, "FREQ", &e.freq);
    let (mode, submode) = to_adif_mode(&e.mode);
    adif_field(&mut s, "MODE", &mode);
    adif_field(&mut s, "SUBMODE", submode.as_deref().unwrap_or_default());
    adif_field(&mut s, "RST_SENT", &e.rst_sent);
    adif_field(&mut s, "RST_RCVD", &e.rst_rcvd);
    adif_field(&mut s, "TX_PWR", &e.tx_pwr);
    adif_field(&mut s, "MY_SIG", "POTA");
    adif_field(&mut s, "MY_SIG_INFO", park);
    s.push_str("<EOR>\n");
  }
  s
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::station::StationProfile;

  fn book(callsign: &str) -> StationBook {
    let mut b = StationBook::default();
    let id = b.upsert(StationProfile {
      callsign: callsign.into(),
      operator: "BG1ABC".into(),
      gridsquare: "OM89".into(),
      ..StationProfile::default()
    });
    b.set_active(id);
    b
  }

  #[allow(clippy::too_many_arguments)]
  fn qso(
    id: u64,
    date: &str,
    time: &str,
    call: &str,
    freq: &str,
    mode: &str,
    pota: &str,
    sota: &str,
  ) -> LogEntry {
    LogEntry {
      id,
      date: date.into(),
      time: time.into(),
      callsign: call.into(),
      freq: freq.into(),
      mode: mode.into(),
      my_pota_ref: pota.into(),
      my_sota_ref: sota.into(),
      rst_sent: "59".into(),
      rst_rcvd: "57".into(),
      ..LogEntry::default()
    }
  }

  fn names(e: &Export) -> Vec<&str> {
    e.files.iter().map(|f| f.name.as_str()).collect()
  }

  #[test]
  fn sota_refs_validate_by_format() {
    for ok in [
      "G/LD-001",
      "BY/BJ-001",
      "W7W/KG-001",
      "9A/CE-123",
      " by/bj-001 ",
    ] {
      assert!(is_valid_sota_ref(ok), "{ok}");
    }
    for bad in [
      "",
      "G/LD-1",
      "G/LD-0001",
      "GLD-001",
      "G/L-001",
      "G/LD001",
      "G/LD-00A",
      "TOOLONG/LD-001",
    ] {
      assert!(!is_valid_sota_ref(bad), "{bad}");
    }
  }

  #[test]
  fn pota_refs_validate_by_format() {
    for ok in ["K-0001", "CN-0123", "VE-1234", "K-12345", "us-1234"] {
      assert!(is_valid_pota_ref(ok), "{ok}");
    }
    for bad in [
      "",
      "K0001",
      "K-001",
      "K-123456",
      "K-ABCD",
      "K-0001,K-0002",
      "VE-5082@CA-AB",
      "TOOLONG-0001",
    ] {
      assert!(!is_valid_pota_ref(bad), "{bad}");
    }
  }

  #[test]
  fn check_ref_normalizes_and_classifies() {
    assert_eq!(
      check_ref(Platform::Pota, " cn-0123 "),
      Ok("CN-0123".to_owned())
    );
    assert_eq!(check_ref(Platform::Pota, "  "), Err(RefError::Empty));
    assert_eq!(
      check_ref(Platform::Pota, "G/LD-001"),
      Err(RefError::BadFormat)
    );
    assert_eq!(
      check_ref(Platform::Sota, "K-0001"),
      Err(RefError::BadFormat)
    );
  }

  #[test]
  fn sota_line_matches_the_v2_layout() {
    let entries = [qso(
      1,
      "2026-10-13",
      "09:05",
      "bd7xyz",
      "14.0620",
      "CW",
      "",
      "BY/BJ-001",
    )];
    let e = export(&entries, &book("BG1ABC/P"), Platform::Sota, "by/bj-001").unwrap();
    assert_eq!(e.files.len(), 1);
    assert_eq!(e.files[0].name, "sota-BY_BJ-001-20261013.csv");
    // 无表头；DD/MM/YYYY；HHMM；频率带 MHz；对方山峰留空；以 CRLF 结束。
    assert_eq!(
      e.files[0].content,
      "V2,BG1ABC/P,BY/BJ-001,13/10/2026,0905,14.062MHz,CW,BD7XYZ,,\r\n"
    );
    assert_eq!(e.skipped, Skipped::default());
  }

  #[test]
  fn sota_frequency_is_trimmed_and_validated() {
    assert_eq!(sota_freq("144.000").as_deref(), Some("144MHz"));
    assert_eq!(sota_freq("7.0325").as_deref(), Some("7.0325MHz"));
    assert_eq!(sota_freq("430.1234567").as_deref(), Some("430.123457MHz"));
    for bad in ["", "abc", "0", "-1", "NaN", "inf"] {
      assert_eq!(sota_freq(bad), None, "{bad}");
    }
  }

  #[test]
  fn sota_modes_map_to_the_platform_vocabulary() {
    for (m, want) in [
      ("CW", "CW"),
      ("SSB", "SSB"),
      ("usb", "SSB"),
      ("AM", "AM"),
      ("FM", "FM"),
      ("DMR", "DV"),
      ("C4FM", "DV"),
      ("DSTAR", "DV"),
      ("FT8", "Data"),
      ("RTTY", "Data"),
      ("DATA", "Data"),
      ("SSTV", "Other"),
      ("OTHER", "Other"),
      ("whatever", "Other"),
    ] {
      assert_eq!(sota_mode(m), want, "{m}");
    }
  }

  #[test]
  fn sota_comments_never_contain_commas_or_control_characters() {
    let mut q = qso(
      1,
      "2026-10-13",
      "09:05",
      "BD7XYZ",
      "14.062",
      "CW",
      "",
      "BY/BJ-001",
    );
    q.remark = " good, signal\n(5w) ".into();
    let e = export(&[q], &book("BG1ABC"), Platform::Sota, "BY/BJ-001").unwrap();
    let line = e.files[0].content.trim_end();
    assert!(line.ends_with(",,good; signal(5w)"), "{line}");
    assert_eq!(line.split(',').count(), 10, "{line}");
  }

  #[test]
  fn pota_record_carries_my_sig_and_park() {
    let entries = [qso(
      1,
      "2026-10-13",
      "09:05",
      "bd7xyz",
      "14.0620",
      "CW",
      "CN-0123",
      "",
    )];
    let e = export(&entries, &book("BG1ABC/P"), Platform::Pota, "cn-0123").unwrap();
    assert_eq!(e.files.len(), 1);
    assert_eq!(e.files[0].name, "BG1ABC_P@CN-0123-20261013.adi");
    let c = &e.files[0].content;
    assert!(c.starts_with("POTA activation\n<ADIF_VER:5>3.1.4<PROGRAMID:10>HamExamWeb<EOH>\n"));
    for needle in [
      "<STATION_CALLSIGN:8>BG1ABC/P",
      "<OPERATOR:6>BG1ABC",
      "<MY_GRIDSQUARE:4>OM89",
      "<CALL:6>BD7XYZ",
      "<QSO_DATE:8>20261013",
      "<TIME_ON:4>0905",
      "<BAND:3>20m",
      "<MODE:2>CW",
      "<RST_SENT:2>59",
      "<MY_SIG:4>POTA",
      "<MY_SIG_INFO:7>CN-0123",
      "<EOR>",
    ] {
      assert!(c.contains(needle), "缺 {needle}\n{c}");
    }
    // 不写对方的 POTA_REF（那是 P2P），也不写 MY_POTA_REF。
    assert!(!c.contains("POTA_REF"));
  }

  #[test]
  fn pota_uses_the_adif_parent_mode_for_submodes() {
    let entries = [qso(
      1,
      "2026-10-13",
      "09:05",
      "BD7XYZ",
      "14.080",
      "FT4",
      "CN-0123",
      "",
    )];
    let c = export(&entries, &book("BG1ABC"), Platform::Pota, "CN-0123")
      .unwrap()
      .files
      .remove(0)
      .content;
    assert!(c.contains("<MODE:4>MFSK<SUBMODE:3>FT4"), "{c}");
  }

  #[test]
  fn each_utc_day_becomes_its_own_file_in_date_order() {
    let entries = [
      qso(
        3,
        "2026-10-14",
        "00:02",
        "A1",
        "14.062",
        "CW",
        "CN-0123",
        "",
      ),
      qso(
        1,
        "2026-10-13",
        "23:58",
        "A2",
        "14.062",
        "CW",
        "CN-0123",
        "",
      ),
      qso(
        2,
        "2026-10-13",
        "09:00",
        "A3",
        "14.062",
        "CW",
        "CN-0123",
        "",
      ),
    ];
    let e = export(&entries, &book("BG1ABC"), Platform::Pota, "CN-0123").unwrap();
    assert_eq!(
      names(&e),
      ["BG1ABC@CN-0123-20261013.adi", "BG1ABC@CN-0123-20261014.adi"]
    );
    assert_eq!(e.files.iter().map(|f| f.qsos).collect::<Vec<_>>(), [2, 1]);
    assert_eq!(e.files[0].date, "2026-10-13");
    // 同一天内按时刻排序，而不是按录入顺序。
    let first = &e.files[0].content;
    assert!(first.find("<CALL:2>A3").unwrap() < first.find("<CALL:2>A2").unwrap());
  }

  #[test]
  fn only_the_chosen_reference_is_exported_case_insensitively() {
    let entries = [
      qso(
        1,
        "2026-10-13",
        "09:00",
        "A1",
        "14.062",
        "CW",
        "CN-0123",
        "",
      ),
      qso(
        2,
        "2026-10-13",
        "09:01",
        "A2",
        "14.062",
        "CW",
        "cn-0123",
        "",
      ),
      qso(
        3,
        "2026-10-13",
        "09:02",
        "A3",
        "14.062",
        "CW",
        "CN-0456",
        "",
      ),
      qso(4, "2026-10-13", "09:03", "A4", "14.062", "CW", "", ""),
    ];
    let e = export(&entries, &book("BG1ABC"), Platform::Pota, "CN-0123").unwrap();
    assert_eq!(e.files.len(), 1);
    assert_eq!(e.files[0].qsos, 2);
    assert_eq!(e.skipped, Skipped::default());
  }

  #[test]
  fn skipped_entries_are_counted_by_reason_not_silently_fixed() {
    let mut no_call = qso(4, "2026-10-13", "09:03", "", "14.062", "CW", "CN-0123", "");
    no_call.callsign.clear();
    let entries = [
      qso(
        1,
        "2026-10-13",
        "09:00",
        "OK1",
        "14.062",
        "CW",
        "CN-0123",
        "",
      ),
      qso(2, "", "09:01", "BAD", "14.062", "CW", "CN-0123", ""),
      qso(
        3,
        "2026-10-13",
        "25:99",
        "BAD",
        "14.062",
        "CW",
        "CN-0123",
        "",
      ),
      no_call,
      qso(5, "2026-10-13", "09:04", "NOFREQ", "", "CW", "CN-0123", ""),
      qso(
        6,
        "2026-10-13",
        "09:05",
        "DATA1",
        "14.070",
        "DATA",
        "CN-0123",
        "",
      ),
      qso(
        7,
        "2026-10-13",
        "09:06",
        "OTH1",
        "14.070",
        "OTHER",
        "CN-0123",
        "",
      ),
    ];
    let pota = export(&entries, &book("BG1ABC"), Platform::Pota, "CN-0123").unwrap();
    assert_eq!(pota.files[0].qsos, 1);
    assert_eq!(
      pota.skipped,
      Skipped {
        bad_datetime: 2,
        missing_data: 2,
        unsupported_mode: 2
      }
    );
    assert_eq!(pota.skipped.total(), 6);

    // SOTA 没有「模式不合法」这一类：DATA / OTHER 映射到 Data / Other，缺频率才算缺数据。
    let mut sota_entries = entries.clone();
    for e in &mut sota_entries {
      e.my_sota_ref = "BY/BJ-001".into();
    }
    let sota = export(&sota_entries, &book("BG1ABC"), Platform::Sota, "BY/BJ-001").unwrap();
    assert_eq!(sota.files[0].qsos, 3);
    assert_eq!(
      sota.skipped,
      Skipped {
        bad_datetime: 2,
        missing_data: 2,
        unsupported_mode: 0
      }
    );
  }

  #[test]
  fn a_station_without_callsign_cannot_be_exported() {
    let entries = [qso(
      1,
      "2026-10-13",
      "09:00",
      "A1",
      "14.062",
      "CW",
      "CN-0123",
      "",
    )];
    let e = export(&entries, &StationBook::default(), Platform::Pota, "CN-0123").unwrap();
    assert!(e.files.is_empty());
    assert_eq!(e.skipped.missing_data, 1);
  }

  #[test]
  fn pota_needs_a_band_but_not_a_frequency() {
    let mut q = qso(1, "2026-10-13", "09:00", "A1", "", "CW", "CN-0123", "");
    q.band = "20m".into();
    let e = export(&[q.clone()], &book("BG1ABC"), Platform::Pota, "CN-0123").unwrap();
    assert_eq!(e.files[0].qsos, 1);
    assert!(e.files[0].content.contains("<BAND:3>20m"));
    assert!(!e.files[0].content.contains("<FREQ:"));
    q.band.clear();
    assert_eq!(
      export(&[q], &book("BG1ABC"), Platform::Pota, "CN-0123")
        .unwrap()
        .skipped
        .missing_data,
      1
    );
  }

  #[test]
  fn bad_reference_is_rejected_before_anything_is_exported() {
    let entries = [qso(
      1,
      "2026-10-13",
      "09:00",
      "A1",
      "14.062",
      "CW",
      "CN-0123",
      "",
    )];
    assert_eq!(
      export(&entries, &book("BG1ABC"), Platform::Pota, "").unwrap_err(),
      RefError::Empty
    );
    assert_eq!(
      export(&entries, &book("BG1ABC"), Platform::Pota, "BY/BJ-001").unwrap_err(),
      RefError::BadFormat
    );
  }

  #[test]
  fn candidates_list_valid_refs_by_count() {
    let entries = [
      qso(
        1,
        "2026-10-13",
        "09:00",
        "A1",
        "14.062",
        "CW",
        "CN-0123",
        "BY/BJ-001",
      ),
      qso(
        2,
        "2026-10-13",
        "09:01",
        "A2",
        "14.062",
        "CW",
        "cn-0123",
        "BY/BJ-001",
      ),
      qso(
        3,
        "2026-10-13",
        "09:02",
        "A3",
        "14.062",
        "CW",
        "CN-0456",
        "",
      ),
      qso(
        4,
        "2026-10-13",
        "09:03",
        "A4",
        "14.062",
        "CW",
        "K-0001,K-0002",
        "not a ref",
      ),
    ];
    assert_eq!(
      candidates(&entries, Platform::Pota),
      vec![("CN-0123".to_owned(), 2), ("CN-0456".to_owned(), 1)]
    );
    assert_eq!(
      candidates(&entries, Platform::Sota),
      vec![("BY/BJ-001".to_owned(), 2)]
    );
    assert!(candidates(&[], Platform::Pota).is_empty());
  }

  #[test]
  fn operator_falls_back_to_the_station_profile() {
    let mut q = qso(
      1,
      "2026-10-13",
      "09:00",
      "A1",
      "14.062",
      "CW",
      "CN-0123",
      "",
    );
    q.operator = "BH1XYZ".into();
    let c = export(&[q], &book("BG1ABC"), Platform::Pota, "CN-0123")
      .unwrap()
      .files
      .remove(0)
      .content;
    assert!(c.contains("<OPERATOR:6>BH1XYZ"));
    let plain = qso(
      2,
      "2026-10-13",
      "09:00",
      "A1",
      "14.062",
      "CW",
      "CN-0123",
      "",
    );
    let c = export(&[plain], &book("BG1ABC"), Platform::Pota, "CN-0123")
      .unwrap()
      .files
      .remove(0)
      .content;
    assert!(c.contains("<OPERATOR:6>BG1ABC"));
  }

  #[test]
  fn minimum_qso_counts_per_platform() {
    assert_eq!(Platform::Pota.min_qsos(), 10);
    assert_eq!(Platform::Sota.min_qsos(), 4);
  }
}
