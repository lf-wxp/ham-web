//! Cabrillo 日志导入：把别的日志软件（N1MM、DXLog 等）导出的 `.cbr` / `.log` 读成通联记录。
//!
//! 产物是 [`AdifRecord`]，所以查重、台站认领都直接复用 ADIF 导入的那一套（`Logbook::import`）。
//!
//! **宁可跳过并报告，也不猜。** `QSO:` 行的列数随竞赛而变（交换可以是一个、两个甚至更多字段），
//! 而 Cabrillo 本身没有声明列布局。这里只认「每边一个交换字段」的常见布局：
//!
//! | 列数 | 布局 |
//! | --- | --- |
//! | 10 | 频率 模式 日期 时间 己方呼号 **报告 交换** 对方呼号 **报告 交换** |
//! | 11 | 同上 + 发射机编号（多发射机类别才有） |
//! | 8 / 9 | 同上但**没有报告**（仅当竞赛不在内置模板里才接受，见 [`Cabrillo::assumed_layout`]） |
//!
//! 其它列数（Sweepstakes、NAQP 这类多字段交换）一律不认，计入 [`Cabrillo::skipped_total`]。
//! 内置模板里的竞赛交换格式是已知的（报告 + 一个字段），所以只接受 10 / 11 列；模板外的竞赛
//! 只能按常见布局猜，由 [`Cabrillo::assumed_layout`] 告诉界面「请核对几行」。
//!
//! 模式是有损的：Cabrillo 只有 `CW` / `PH` / `FM` / `RY` / `DG`，所以 `PH` 读成 `SSB`、
//! `DG` 读成 `DATA`，原日志里的 FT8 / PSK31 / USB 等细分回不来。
//!
//! 没有取到 Cabrillo 官方规范原文核对（评估时站点不可达）；格式依据是本站导出器的输出
//! （`contest::cabrillo`，往返测试覆盖）与对常见日志软件输出的了解。

use crate::adif::AdifRecord;
use crate::contest::contest;
use crate::frequencies::band_of;
use crate::logbook::{LogEntry, from_adif};
use crate::study_plan::{day_number, format_day};

/// 文件大小上限：真实的竞赛日志在 MB 以内，超过这个多半不是 Cabrillo。
pub const MAX_BYTES: u64 = 8 * 1024 * 1024;

/// 最多保留几条「无法识别的行」做样例（总数另计）。
const MAX_SAMPLES: usize = 20;

/// 样例里保留的原文最大字符数。
const MAX_RAW_CHARS: usize = 100;

/// 一行为什么没能读进来。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
  /// 列数不是认得的布局（含多字段交换）。
  Layout,
  /// 频率不是 kHz 整数，也不是 Cabrillo 的波段代号。
  Frequency,
  /// 模式不在 `CW` / `PH` / `FM` / `RY` / `DG` 里。
  Mode,
  /// 日期不是合法的 `YYYY-MM-DD`。
  Date,
  /// 时间不是合法的 `HHMM`。
  Time,
  /// 呼号看起来不是呼号。
  Callsign,
  /// 信号报告看起来不是 RST。
  Report,
}

/// 一条没读进来的行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
  /// 行号（从 1 起）。
  pub line: usize,
  pub reason: SkipReason,
  /// 原文（已截断）。
  pub text: String,
}

/// 解析结果。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cabrillo {
  /// `START-OF-LOG` 的版本（如 `3.0`）。
  pub version: Option<String>,
  /// `CONTEST` 头（大写；没有则为空）。
  pub contest: String,
  /// `CALLSIGN` 头（大写）。
  pub callsign: String,
  /// 读出来的通联，按文件顺序。
  pub records: Vec<AdifRecord>,
  /// 没读进来的行（样例，最多 [`MAX_SAMPLES`] 条）。
  pub skipped: Vec<Skipped>,
  /// 没读进来的行的总数。
  pub skipped_total: usize,
  /// 被标成 `X-QSO:`（日志软件里标为不计分 / 已删除）而忽略的行数。
  pub x_qso: usize,
  /// 竞赛不在内置模板里：列布局是按「报告 + 一个交换」的常见格式猜的，应提示用户核对。
  pub assumed_layout: bool,
}

/// 仅单台站、连续短时间窗的日志可以自动切到一场比赛；不是官方赛程。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportScope {
  pub start: String,
  pub end: String,
  pub callsign: String,
  pub gridsquare: String,
  pub exchange: String,
}

impl Cabrillo {
  /// 导入快照的半开时间窗。混合台站或跨度超过一周时要求用户自行分场。
  #[must_use]
  pub fn scope(&self) -> Option<ImportScope> {
    let first = self.records.first()?;
    let callsign = first.station_callsign.trim().to_ascii_uppercase();
    let mut from = i64::MAX;
    let mut to = i64::MIN;
    for r in &self.records {
      if !r.station_callsign.trim().eq_ignore_ascii_case(&callsign) {
        return None;
      }
      let minute = from_adif(r.clone()).minutes_on()?;
      from = from.min(minute);
      to = to.max(minute);
    }
    if callsign.is_empty() || to.checked_sub(from)? > 7 * 1440 {
      return None;
    }
    let text = |t| {
      let (d, h) = crate::logbook::stamp(t);
      format!("{d} {h}")
    };
    Some(ImportScope {
      start: text(from),
      end: text(to.checked_add(1)?),
      callsign,
      gridsquare: first.my_gridsquare.clone(),
      exchange: first.stx.clone(),
    })
  }

  /// 看起来是不是 Cabrillo：有版本头，或至少有一条 `QSO:` 行（不管读没读成）。
  #[must_use]
  pub fn recognized(&self) -> bool {
    self.version.is_some() || !self.records.is_empty() || self.skipped_total > 0
  }

  /// 最早一条通联的时刻，`YYYY-MM-DD HH:MM`（UTC）。
  #[must_use]
  pub fn earliest(&self) -> Option<String> {
    self
      .records
      .iter()
      .map(|r| format!("{} {}", dashed(&r.qso_date), colon(&r.time_on)))
      .min()
  }
}

/// `YYYYMMDD` → `YYYY-MM-DD`。
///
/// `records` 是 `pub` 字段：外部（旧版本落盘的 JSON、别处构造的记录）塞进来的未必是
/// 定宽写法，所以用 `get` 逐段取，而不是按长度直接切 —— 后者会 panic 在多字节边界上。
fn dashed(d: &str) -> String {
  match (d.get(..4), d.get(4..6), d.get(6..8)) {
    (Some(y), Some(m), Some(day)) => format!("{y}-{m}-{day}"),
    _ => d.to_owned(),
  }
}

/// `HHMM` → `HH:MM`（同 [`dashed`]，不假设长度）。
fn colon(t: &str) -> String {
  match (t.get(..2), t.get(2..4)) {
    (Some(h), Some(m)) => format!("{h}:{m}"),
    _ => t.to_owned(),
  }
}

/// 解析一份 Cabrillo 文本。
///
/// 容忍：CRLF、行首尾空白、键名与呼号大小写、空行、`START-OF-LOG` 缺失。
/// `END-OF-LOG` 之后的内容忽略。
#[must_use]
pub fn parse_cabrillo(text: &str) -> Cabrillo {
  let mut out = Cabrillo::default();
  let mut grid = String::new();
  // `QSO:` 行先攒起来：头信息理论上在前，但不依赖这个顺序。
  let mut qsos: Vec<(usize, &str)> = Vec::new();

  for (i, raw) in text.lines().enumerate() {
    let line = raw.trim();
    let Some((key, value)) = line.split_once(':') else {
      continue;
    };
    let value = value.trim();
    match key.trim().to_ascii_uppercase().as_str() {
      "START-OF-LOG" => out.version = Some(value.to_owned()),
      "CONTEST" => out.contest = value.to_ascii_uppercase(),
      "CALLSIGN" => out.callsign = value.to_ascii_uppercase(),
      "GRID-LOCATOR" => grid = value.to_ascii_uppercase(),
      "QSO" => qsos.push((i + 1, value)),
      "X-QSO" => out.x_qso += 1,
      "END-OF-LOG" => break,
      _ => {}
    }
  }

  let known = contest(&out.contest).is_some() && out.contest != "OTHER";
  out.assumed_layout = !known && !qsos.is_empty();

  for (line, rest) in qsos {
    match parse_qso(rest, known) {
      Ok(q) => out.records.push(AdifRecord {
        call: q.call,
        qso_date: q.date,
        time_on: q.time,
        freq: q.freq,
        band: q.band,
        mode: q.mode,
        rst_sent: q.rst_sent,
        rst_rcvd: q.rst_rcvd,
        contest_id: out.contest.clone(),
        stx: q.stx,
        srx: q.srx,
        // 逐条取 QSO 行里的己方呼号：多操作员 / 带后缀（/P）时比头信息准。
        station_callsign: q.my_call,
        my_gridsquare: grid.clone(),
        ..AdifRecord::default()
      }),
      Err(reason) => {
        out.skipped_total += 1;
        if out.skipped.len() < MAX_SAMPLES {
          out.skipped.push(Skipped {
            line,
            reason,
            text: rest.chars().take(MAX_RAW_CHARS).collect(),
          });
        }
      }
    }
  }
  // 没有头信息、也没有认得的内容：保持空结果，让界面说「这不像 Cabrillo」。
  out
}

struct Qso {
  call: String,
  my_call: String,
  date: String,
  time: String,
  freq: String,
  band: String,
  mode: String,
  rst_sent: String,
  rst_rcvd: String,
  stx: String,
  srx: String,
}

fn parse_qso(rest: &str, known: bool) -> Result<Qso, SkipReason> {
  let t: Vec<&str> = rest.split_whitespace().collect();
  let (has_rst, has_tx) = match (t.len(), known) {
    (10, _) => (true, false),
    (11, _) => (true, true),
    (8, false) => (false, false),
    (9, false) => (false, true),
    _ => return Err(SkipReason::Layout),
  };
  if has_tx && !is_transmitter(t[t.len() - 1]) {
    return Err(SkipReason::Layout);
  }

  let (freq, band) = parse_freq(t[0]).ok_or(SkipReason::Frequency)?;
  let mode = parse_mode(t[1]).ok_or(SkipReason::Mode)?;
  let date = parse_date(t[2]).ok_or(SkipReason::Date)?;
  let time = parse_time(t[3]).ok_or(SkipReason::Time)?;
  let my_call = t[4].to_ascii_uppercase();
  if !is_callsign(&my_call) {
    return Err(SkipReason::Callsign);
  }

  let (rst_sent, stx, call, rst_rcvd, srx) = if has_rst {
    (t[5], t[6], t[7], t[8], t[9])
  } else {
    ("", t[5], t[6], "", t[7])
  };
  if has_rst && !(is_report(rst_sent) && is_report(rst_rcvd)) {
    return Err(SkipReason::Report);
  }
  let call = call.to_ascii_uppercase();
  if !is_callsign(&call) {
    return Err(SkipReason::Callsign);
  }

  Ok(Qso {
    call,
    my_call,
    date,
    time,
    freq,
    band,
    mode,
    rst_sent: rst_sent.to_ascii_uppercase(),
    rst_rcvd: rst_rcvd.to_ascii_uppercase(),
    stx: exchange(stx),
    srx: exchange(srx),
  })
}

/// 导出器对空交换写 `-`：读回来还原成空。
fn exchange(v: &str) -> String {
  if v == "-" {
    String::new()
  } else {
    v.to_ascii_uppercase()
  }
}

/// 发射机编号：一两位数字（`0` / `1`，多发射机类别才有）。
fn is_transmitter(s: &str) -> bool {
  (1..=2).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
}

/// RST：2 或 3 位，每位 1–9 或 `N`（`5NN`）。
fn is_report(s: &str) -> bool {
  (2..=3).contains(&s.len()) && s.bytes().all(|b| matches!(b, b'1'..=b'9' | b'N' | b'n'))
}

/// 导入与补全共用呼号校验，避免同一合法呼号在两个入口得到不同结果。
fn is_callsign(s: &str) -> bool {
  crate::scp::is_callsign(s)
}

fn parse_mode(s: &str) -> Option<String> {
  Some(
    match s.to_ascii_uppercase().as_str() {
      "CW" => "CW",
      "PH" => "SSB",
      "FM" => "FM",
      "RY" => "RTTY",
      "DG" => "DATA",
      _ => return None,
    }
    .to_owned(),
  )
}

/// `YYYY-MM-DD` → `YYYYMMDD`（日历上必须真有这一天）。
///
/// 不能只靠 [`day_number`]：它只检查「日在 1–31」，`2026-02-30` 会被静默折成 3 月 2 日。
/// 所以转成天数再转回来，必须与原文一字不差。
fn parse_date(s: &str) -> Option<String> {
  let b = s.as_bytes();
  if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
    return None;
  }
  (format_day(day_number(s)?) == s).then(|| s.replace('-', ""))
}

/// `HHMM`。
fn parse_time(s: &str) -> Option<String> {
  if s.len() != 4 || !s.bytes().all(|b| b.is_ascii_digit()) {
    return None;
  }
  let (h, m): (u32, u32) = (s[..2].parse().ok()?, s[2..].parse().ok()?);
  (h < 24 && m < 60).then(|| s.to_owned())
}

/// 频率列 → `(MHz 文本, 波段)`。
///
/// - 1000 及以上：kHz 整数（`14021` → `14.021`）。波段留空由频率推算；推算不出的（如 1.25 m）补一个名字；
/// - 小于 1000：VHF 波段代号（`50` / `144` / `432` …），没有频率，只给波段；
/// - `1.2G` / `10G` / `LIGHT` 这类微波以上代号：同样只给波段。
fn parse_freq(s: &str) -> Option<(String, String)> {
  let up = s.to_ascii_uppercase();
  if let Ok(v) = up.parse::<u64>() {
    // 上限 300 GHz（= 300 000 000 kHz）：再大只可能是垃圾。
    if v >= 1000 {
      if v > 300_000_000 {
        return None;
      }
      let mhz = format!("{}.{:03}", v / 1000, v % 1000);
      #[allow(clippy::cast_precision_loss)]
      let f = v as f64 / 1000.0;
      let band = if band_of(f) == "其他" {
        extra_band(f)?
      } else {
        ""
      };
      return Some((mhz, band.to_owned()));
    }
    let band = match v {
      50 => "6m",
      70 => "4m",
      144 => "2m",
      222 => "1.25m",
      432 => "70cm",
      902 => "33cm",
      _ => return None,
    };
    return Some((String::new(), band.to_owned()));
  }
  let band = match up.as_str() {
    "1.2G" => "23cm",
    "2.3G" => "13cm",
    "3.4G" => "9cm",
    "5.7G" => "6cm",
    "10G" => "3cm",
    "24G" | "47G" | "75G" | "119G" | "142G" | "241G" | "300G" | "LIGHT" => {
      return Some((String::new(), up.to_ascii_lowercase()));
    }
    _ => return None,
  };
  Some((String::new(), band.to_owned()))
}

/// 全站波段表（只到 70 cm 的 `431–440`）之外、日志里常见的几个业余波段。
fn extra_band(mhz: f64) -> Option<&'static str> {
  const EXTRA: &[(&str, f64, f64)] = &[
    ("1.25m", 219.0, 225.0),
    ("70cm", 420.0, 450.0),
    ("33cm", 902.0, 928.0),
    ("23cm", 1240.0, 1300.0),
    ("13cm", 2300.0, 2450.0),
    ("9cm", 3300.0, 3500.0),
    ("6cm", 5650.0, 5925.0),
    ("3cm", 10000.0, 10500.0),
    ("24g", 24000.0, 24250.0),
    ("47g", 47000.0, 47200.0),
    ("75g", 75500.0, 81000.0),
    ("119g", 119980.0, 120020.0),
    ("142g", 142000.0, 149000.0),
    ("241g", 241000.0, 250000.0),
  ];
  EXTRA
    .iter()
    .find(|(_, lo, hi)| (*lo..=*hi).contains(&mhz))
    .map(|(name, _, _)| *name)
}

/// 宽松的「是同一次通联」键：呼号 + 时刻 + 波段 + **模式大类**。
///
/// Cabrillo 的模式只有 `CW` / `PH` / `DG` 等大类，读回来的 `SSB` / `DATA` 与日志里原来的
/// `USB` / `FT8` 对不上——用 `Logbook::import` 的精确键会把自己导出的日志当成全新通联再导一遍。
fn loose_key(e: &LogEntry) -> Option<String> {
  Some(format!(
    "{}|{}|{}|{}",
    e.callsign.trim().to_ascii_uppercase(),
    e.minutes_on()?,
    e.band_label(),
    crate::contest::cabrillo_mode(&e.mode)
  ))
}

/// 去掉日志里已有的通联：返回（剩下的记录，去掉的条数）。
///
/// 用 [`loose_key`] 而不是 `Logbook::import` 的精确键，所以先过这一道，再交给 `import` 做精确查重。
#[must_use]
pub fn drop_logged(
  records: Vec<AdifRecord>,
  existing: &[LogEntry],
  stations: &crate::station::StationBook,
) -> (Vec<AdifRecord>, usize) {
  let have: std::collections::HashSet<(u64, String)> = existing
    .iter()
    .filter_map(|e| loose_key(e).map(|key| (stations.of_entry(e).id, key)))
    .collect();
  let mut identities = stations.clone();
  let before = records.len();
  let kept: Vec<AdifRecord> = records
    .into_iter()
    .filter(|r| {
      let id = identities.claim(&r.station_callsign, &r.my_gridsquare);
      loose_key(&from_adif(r.clone())).is_none_or(|k| !have.contains(&(id, k)))
    })
    .collect();
  let dropped = before - kept.len();
  (kept, dropped)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::contest::{CabrilloHeader, cabrillo, score};

  fn stations() -> crate::station::StationBook {
    let mut book = crate::station::StationBook::default();
    book.profiles[0].callsign = "BG4ABC".into();
    book.profiles[0].gridsquare = "OM89".into();
    book
  }

  fn parse(body: &str) -> Cabrillo {
    parse_cabrillo(&format!(
      "START-OF-LOG: 3.0\nCONTEST: CQ-WW-CW\nCALLSIGN: BG4ABC\nGRID-LOCATOR: OM89\n{body}\nEND-OF-LOG:\n"
    ))
  }

  fn entry(id: u64, call: &str, freq: &str, srx: &str) -> LogEntry {
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
  fn reads_a_line_with_every_field() {
    let c =
      parse("QSO: 14021 CW 2026-10-31 0010 BG4ABC        599 24     W1AW          599 5      0");
    assert_eq!(c.version.as_deref(), Some("3.0"));
    assert_eq!(
      (c.contest.as_str(), c.callsign.as_str()),
      ("CQ-WW-CW", "BG4ABC")
    );
    assert!(!c.assumed_layout);
    assert_eq!(c.skipped_total, 0);
    let r = &c.records[0];
    assert_eq!(
      (
        r.call.as_str(),
        r.qso_date.as_str(),
        r.time_on.as_str(),
        r.freq.as_str()
      ),
      ("W1AW", "20261031", "0010", "14.021")
    );
    assert_eq!(
      (r.mode.as_str(), r.rst_sent.as_str(), r.rst_rcvd.as_str()),
      ("CW", "599", "599")
    );
    assert_eq!((r.stx.as_str(), r.srx.as_str()), ("24", "5"));
    assert_eq!(r.contest_id, "CQ-WW-CW");
    assert_eq!(
      (r.station_callsign.as_str(), r.my_gridsquare.as_str()),
      ("BG4ABC", "OM89")
    );
    // 14.021 MHz 本来就在业余波段表里：波段留空，让 `band_label` 按频率推。
    assert_eq!(r.band, "");
  }

  #[test]
  fn what_our_own_exporter_writes_reads_back_and_scores_the_same() {
    let def = contest("CQ-WW-CW").unwrap();
    let log = vec![
      entry(1, "W1AW", "14.021", "5"),
      entry(2, "JA1ABC", "7.011", "25"),
      entry(3, "DL1ABC", "21.020", "14"),
      entry(4, "VK2XX", "28.030", ""), // 空交换：导出写 `-`
    ];
    let h = CabrilloHeader {
      callsign: "bg4abc".into(),
      grid: "OM89".into(),
      ..Default::default()
    };
    let text = cabrillo(def, &h, &log);
    let c = parse_cabrillo(&text);
    assert_eq!((c.records.len(), c.skipped_total), (4, 0), "{text}");
    let back: Vec<LogEntry> = c.records.iter().cloned().map(from_adif).collect();
    for (a, b) in log.iter().zip(&back) {
      assert_eq!(a.callsign, b.callsign);
      assert_eq!(
        (a.date.as_str(), a.time.as_str()),
        (b.date.as_str(), b.time.as_str())
      );
      assert_eq!(a.freq, b.freq, "{}", a.callsign);
      assert_eq!(
        (a.stx.as_str(), a.srx.as_str()),
        (b.stx.as_str(), b.srx.as_str())
      );
      assert_eq!(b.contest_id, "CQ-WW-CW");
    }
    // 记分结果一致：导出再导入没有丢失任何影响得分的东西。
    assert_eq!(score(def, &log, "BG4ABC"), score(def, &back, "BG4ABC"));
  }

  #[test]
  fn times_written_without_zero_padding_survive_the_export() {
    // 日志里的时间可能是 `8:30` / `0:10`（导入的 ADIF、手填都会这样）。导出曾把它们写成 `0000`，
    // 提交给主办方的日志里这条通联的时间就是错的，且读回来对不上。
    let def = contest("CQ-WW-CW").unwrap();
    let h = CabrilloHeader {
      callsign: "BG4ABC".into(),
      ..Default::default()
    };
    for (written, hhmm) in [
      ("0:10", "0010"),
      ("8:30", "0830"),
      ("08:30", "0830"),
      ("08:30:15", "0830"),
      ("23:59", "2359"),
      (" 7:05 ", "0705"),
    ] {
      let mut e = entry(1, "W1AW", "14.021", "5");
      e.time = written.into();
      let text = cabrillo(def, &h, std::slice::from_ref(&e));
      let c = parse_cabrillo(&text);
      assert_eq!(c.records.len(), 1, "{written}: {text}");
      assert_eq!(c.records[0].time_on, hhmm, "{written}");
    }
  }

  #[test]
  fn tolerates_crlf_case_blank_lines_and_stops_at_end_of_log() {
    let text = "start-of-log: 2.0\r\n\r\ncontest: cq-ww-cw\r\n  Callsign:  bg4abc \r\n\r\n   qso:  14021 cw 2026-10-31 0010 bg4abc 599 24 w1aw 599 5\r\nEND-OF-LOG:\r\nQSO: 14022 CW 2026-10-31 0011 BG4ABC 599 24 JA1ABC 599 25\r\n";
    let c = parse_cabrillo(text);
    assert_eq!(c.version.as_deref(), Some("2.0"));
    assert_eq!(c.contest, "CQ-WW-CW");
    // END-OF-LOG 之后的那条不读。
    assert_eq!(c.records.len(), 1);
    assert_eq!(c.records[0].call, "W1AW");
    assert_eq!(c.records[0].station_callsign, "BG4ABC");
  }

  #[test]
  fn every_rejection_has_its_own_reason_and_line_number() {
    let c = parse(
      "QSO: 14021 CW 2026-10-31 0010 BG4ABC 599 24 W1AW 599\n\
       QSO: 14.021 CW 2026-10-31 0010 BG4ABC 599 24 W1AW 599 5\n\
       QSO: 14021 XX 2026-10-31 0010 BG4ABC 599 24 W1AW 599 5\n\
       QSO: 14021 CW 2026-02-30 0010 BG4ABC 599 24 W1AW 599 5\n\
       QSO: 14021 CW 2026-10-31 2460 BG4ABC 599 24 W1AW 599 5\n\
       QSO: 14021 CW 2026-10-31 0010 BG4ABC 599 24 ??? 599 5\n\
       QSO: 14021 CW 2026-10-31 0010 BG4ABC 099 24 W1AW 599 5\n\
       QSO: 14021 CW 2026-10-31 0010 BG4ABC 599 24 W1AW 599 5 x\n\
       QSO: 14021 CW 2026-10-31 0010 BG4ABC 599 24 W1AW 599 5 0",
    );
    let got: Vec<_> = c.skipped.iter().map(|s| (s.line, s.reason)).collect();
    // 前 4 行是头信息，第一条 QSO 在第 5 行。
    assert_eq!(
      got,
      vec![
        (5, SkipReason::Layout),
        (6, SkipReason::Frequency),
        (7, SkipReason::Mode),
        (8, SkipReason::Date),
        (9, SkipReason::Time),
        (10, SkipReason::Callsign),
        (11, SkipReason::Report),
        (12, SkipReason::Layout),
      ]
    );
    assert_eq!(c.skipped_total, 8);
    assert_eq!(c.records.len(), 1); // 最后一条是对的
    assert_eq!(
      c.skipped[0].text,
      "14021 CW 2026-10-31 0010 BG4ABC 599 24 W1AW 599"
    );
  }

  #[test]
  fn multi_field_exchanges_are_refused_not_misread() {
    // Sweepstakes 一类：每边多个交换字段，列数对不上。必须跳过，不能把某个字段当成呼号。
    let c = parse_cabrillo(
      "START-OF-LOG: 3.0\nCONTEST: ARRL-SS-CW\nQSO: 14021 CW 2026-10-31 0010 BG4ABC 001 A 72 CT W1AW 002 B 73 CT\nEND-OF-LOG:\n",
    );
    assert_eq!((c.records.len(), c.skipped_total), (0, 1));
    assert_eq!(c.skipped[0].reason, SkipReason::Layout);
    assert!(c.assumed_layout);
  }

  #[test]
  fn two_token_exchanges_that_look_like_reports_are_the_known_blind_spot() {
    // 已知盲区，用测试写明而不是假装没有：模板外的竞赛里，若每边是两个都像 RST 的数字字段，
    // 10 列布局会把它们当成「报告 + 交换」读进来。所以模板外的竞赛一律带 `assumed_layout`，
    // 界面据此提示用户核对。
    let c = parse_cabrillo(
      "START-OF-LOG: 3.0\nCONTEST: SOME-OTHER\nQSO: 14021 CW 2026-10-31 0010 BG4ABC 123 45 W1AW 234 56\nEND-OF-LOG:\n",
    );
    assert_eq!(c.records.len(), 1);
    assert!(c.assumed_layout);
    assert_eq!(c.records[0].rst_sent, "123");
  }

  #[test]
  fn known_contests_only_accept_report_plus_one_field() {
    // 模板里的竞赛：8 / 9 列（无报告）不接受 —— 我们知道它们的交换里有报告。
    let known = parse("QSO: 14021 CW 2026-10-31 0010 BG4ABC 24 W1AW 5");
    assert_eq!((known.records.len(), known.skipped_total), (0, 1));
    // 模板外：接受，并标记为猜的布局。
    let other = parse_cabrillo(
      "START-OF-LOG: 3.0\nCONTEST: NAQP-CW\nQSO: 14021 CW 2026-10-31 0010 BG4ABC 24 W1AW 5\nQSO: 14022 CW 2026-10-31 0011 BG4ABC 24 JA1ABC 25 1\nEND-OF-LOG:\n",
    );
    assert_eq!(other.records.len(), 2);
    assert!(other.assumed_layout);
    assert_eq!(other.records[0].rst_sent, "");
    assert_eq!(
      (other.records[0].stx.as_str(), other.records[0].srx.as_str()),
      ("24", "5")
    );
    assert_eq!(other.records[1].call, "JA1ABC"); // 9 列：末尾是发射机编号
  }

  #[test]
  fn an_unnamed_contest_is_treated_as_unknown() {
    let c = parse_cabrillo("QSO: 14021 CW 2026-10-31 0010 BG4ABC 599 24 W1AW 599 5\n");
    assert_eq!(c.contest, "");
    assert!(c.assumed_layout);
    assert_eq!(c.records.len(), 1);
    assert_eq!(c.version, None);
    // OTHER 是内置的「通用」模板，不代表交换格式已知。
    let o =
      parse_cabrillo("CONTEST: OTHER\nQSO: 14021 CW 2026-10-31 0010 BG4ABC 599 24 W1AW 599 5\n");
    assert!(o.assumed_layout);
  }

  #[test]
  fn frequencies_khz_vhf_designators_and_microwave() {
    let f = |s: &str| parse_freq(s);
    let s = |a: &str, b: &str| Some((a.to_owned(), b.to_owned()));
    assert_eq!(f("14021"), s("14.021", ""));
    assert_eq!(f("1800"), s("1.800", ""));
    assert_eq!(f("28400"), s("28.400", ""));
    assert_eq!(f("50125"), s("50.125", "")); // 把 6 m 写成 kHz 的日志软件
    assert_eq!(f("432100"), s("432.100", ""));
    assert_eq!(f("445000"), s("445.000", "70cm")); // 全站波段表只到 440
    assert_eq!(f("223500"), s("223.500", "1.25m"));
    assert_eq!(f("50"), s("", "6m"));
    assert_eq!(f("144"), s("", "2m"));
    assert_eq!(f("222"), s("", "1.25m"));
    assert_eq!(f("432"), s("", "70cm"));
    assert_eq!(f("1.2G"), s("", "23cm"));
    assert_eq!(f("10g"), s("", "3cm"));
    assert_eq!(f("LIGHT"), s("", "light"));
    // 认不出的一律拒绝：频率 / 波段是去重键的一部分，猜错会把不同通联并成一条。
    for bad in ["", "0", "999", "14.021", "abc", "300000001", "-5"] {
      assert_eq!(f(bad), None, "{bad:?}");
    }
  }

  #[test]
  fn vhf_designator_lines_carry_a_band_and_no_frequency() {
    let c = parse(
      "QSO: 50 PH 2026-10-31 0010 BG4ABC 59 24 W1AW 59 5\nQSO: 144 FM 2026-10-31 0011 BG4ABC 59 24 JA1ABC 59 25",
    );
    assert_eq!(c.records.len(), 2);
    assert_eq!(
      (c.records[0].freq.as_str(), c.records[0].band.as_str()),
      ("", "6m")
    );
    let e = from_adif(c.records[0].clone());
    assert_eq!(e.band_label(), "6m"); // 没有频率时由 band 字段兜底
    assert_eq!(c.records[0].mode, "SSB");
    assert_eq!(c.records[1].mode, "FM");
  }

  #[test]
  fn modes_map_to_the_logbook_vocabulary() {
    let m = |s: &str| parse_mode(s);
    assert_eq!(m("CW").as_deref(), Some("CW"));
    assert_eq!(m("ph").as_deref(), Some("SSB"));
    assert_eq!(m("FM").as_deref(), Some("FM"));
    assert_eq!(m("RY").as_deref(), Some("RTTY"));
    assert_eq!(m("DG").as_deref(), Some("DATA"));
    assert_eq!(m("SSB"), None);
    // 读出来的模式在日志里仍落在原来的大类：往返不会改变查重键里的 `mode_code`。
    for (cab, code) in [
      ("CW", "CW"),
      ("PH", "PH"),
      ("FM", "PH"),
      ("RY", "RY"),
      ("DG", "DG"),
    ] {
      assert_eq!(
        crate::contest::mode_code(&parse_mode(cab).unwrap()),
        code,
        "{cab}"
      );
    }
  }

  #[test]
  fn dates_must_exist_on_the_calendar() {
    let d = |s: &str| parse_date(s);
    assert_eq!(d("2026-10-31").as_deref(), Some("20261031"));
    assert_eq!(d("2024-02-29").as_deref(), Some("20240229")); // 闰年
    for bad in [
      "2026-02-29", // 非闰年
      "2026-02-30",
      "2026-04-31",
      "2100-02-29", // 世纪年非闰
      "2026-00-10",
      "2026-13-01",
      "2026-10-00",
      "2026-1-05",
      "20261031",
      "",
    ] {
      assert_eq!(d(bad), None, "{bad:?}");
    }
    assert_eq!(d("2000-02-29").as_deref(), Some("20000229")); // 400 年一闰
  }

  #[test]
  fn x_qso_lines_are_counted_and_left_out() {
    let c = parse(
      "QSO: 14021 CW 2026-10-31 0010 BG4ABC 599 24 W1AW 599 5\nX-QSO: 14022 CW 2026-10-31 0011 BG4ABC 599 24 JA1ABC 599 25",
    );
    assert_eq!((c.records.len(), c.x_qso, c.skipped_total), (1, 1, 0));
  }

  #[test]
  fn per_qso_station_call_beats_the_header() {
    let c = parse("QSO: 14021 CW 2026-10-31 0010 bg4abc/p 599 24 W1AW 599 5");
    assert_eq!(c.records[0].station_callsign, "BG4ABC/P");
    assert_eq!(c.callsign, "BG4ABC");
  }

  #[test]
  fn garbage_is_not_recognized_and_samples_are_capped() {
    let g = parse_cabrillo("hello\nworld: 1\n<EOH>\n");
    assert!(!g.recognized());
    assert_eq!((g.records.len(), g.skipped_total), (0, 0));
    assert!(parse_cabrillo("START-OF-LOG: 3.0\nEND-OF-LOG:\n").recognized());

    let many: String = (0..50).map(|_| "QSO: 14021 CW bad\n").collect();
    let c = parse_cabrillo(&format!("START-OF-LOG: 3.0\nCONTEST: CQ-WW-CW\n{many}"));
    assert_eq!(c.skipped_total, 50);
    assert_eq!(c.skipped.len(), MAX_SAMPLES);
    assert_eq!(c.skipped[0].line, 3);
    // 超长原文被截断，不会把整行塞进界面。
    let long = format!("QSO: {}", "x".repeat(500));
    let c = parse_cabrillo(&long);
    assert_eq!(c.skipped[0].text.chars().count(), MAX_RAW_CHARS);
  }

  #[test]
  fn earliest_picks_the_first_moment_not_the_first_line() {
    let c = parse(
      "QSO: 14021 CW 2026-10-31 0150 BG4ABC 599 24 W1AW 599 5\nQSO: 14022 CW 2026-10-30 2359 BG4ABC 599 24 JA1ABC 599 25\nQSO: 14023 CW 2026-10-31 0001 BG4ABC 599 24 DL1ABC 599 14",
    );
    assert_eq!(c.earliest().as_deref(), Some("2026-10-30 23:59"));
    assert_eq!(parse("").earliest(), None);
  }

  #[test]
  fn already_logged_qsos_are_dropped_even_when_mode_and_time_are_written_differently() {
    let c = parse(
      "QSO: 14021 PH 2026-10-31 0010 BG4ABC 59 24 W1AW 59 5\nQSO: 14022 CW 2026-10-31 0011 BG4ABC 599 24 JA1ABC 599 25",
    );
    // 日志里这条原来是 USB、时间写成 `0:10`、波段靠频率推出来：与 Cabrillo 的 PH 同属话音大类。
    let existing = vec![LogEntry {
      id: 1,
      date: "2026-10-31".into(),
      time: "0:10".into(),
      freq: "14.021".into(),
      mode: "USB".into(),
      callsign: "w1aw".into(),
      ..Default::default()
    }];
    let (kept, dropped) = drop_logged(c.records, &existing, &stations());
    assert_eq!(dropped, 1);
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].call, "JA1ABC");
  }

  #[test]
  fn not_the_same_qso_when_band_mode_class_or_minute_differs() {
    let base = LogEntry {
      id: 1,
      date: "2026-10-31".into(),
      time: "00:10".into(),
      freq: "14.021".into(),
      mode: "CW".into(),
      callsign: "W1AW".into(),
      ..Default::default()
    };
    let one = |line: &str| parse(line).records;
    // 同呼号同时刻但换了波段 / 模式大类 / 差一分钟：都是另一次通联。
    for line in [
      "QSO: 7021 CW 2026-10-31 0010 BG4ABC 599 24 W1AW 599 5",
      "QSO: 14021 PH 2026-10-31 0010 BG4ABC 59 24 W1AW 59 5",
      "QSO: 14021 CW 2026-10-31 0011 BG4ABC 599 24 W1AW 599 5",
    ] {
      let (kept, dropped) = drop_logged(one(line), std::slice::from_ref(&base), &stations());
      assert_eq!((kept.len(), dropped), (1, 0), "{line}");
    }
    // 日志里时间坏掉的记录没法判断，不能拿来吞掉新记录。
    let broken = LogEntry {
      time: "??".into(),
      ..base
    };
    let (kept, dropped) = drop_logged(
      one("QSO: 14021 CW 2026-10-31 0010 BG4ABC 599 24 W1AW 599 5"),
      &[broken],
      &stations(),
    );
    assert_eq!((kept.len(), dropped), (1, 0));
  }

  #[test]
  fn earliest_survives_records_that_are_not_fixed_width() {
    // `records` 是 `pub` 字段：旧版本落盘的 JSON / 别处构造的记录未必是定宽写法，
    // 按长度切片会 panic 在多字节边界上（`é` 是 2 字节、`中` 是 3 字节）。
    let mut c = Cabrillo {
      records: vec![
        AdifRecord {
          qso_date: "中".into(),
          time_on: "é".into(),
          ..AdifRecord::default()
        },
        AdifRecord {
          qso_date: "20261031".into(),
          time_on: "2359".into(),
          ..AdifRecord::default()
        },
      ],
      ..Cabrillo::default()
    };
    assert_eq!(c.earliest().as_deref(), Some("2026-10-31 23:59"));
    // 只剩不成形的那条：原样拼，不按长度假设。
    c.records.remove(1);
    assert_eq!(c.earliest().as_deref(), Some("中 é"));
  }
}
