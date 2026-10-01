//! 通联日志的数据模型与纯逻辑：记录结构、ADIF / CSV 导出、ADIF 导入去重、录入提示。
//!
//! 记录字段对齐 ADIF 规范：本台呼号（`STATION_CALLSIGN`）、操作员、本台网格、
//! `RST_SENT` / `RST_RCVD` 分离、对方网格 / 姓名 / QTH、功率、卫星、SOTA / POTA、
//! DXCC / CQZ / ITUZ / CONT 等。

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::adif::{AdifRecord, to_adif_mode};
use crate::dxcc::{Entity, entity_by_dxcc, lookup};
use crate::frequencies::band_of;
use crate::grid::{distance_bearing, lat_lon_from_grid};

/// 录入表单可选的模式。
pub const MODES: &[&str] = &[
  "CW", "SSB", "AM", "FM", "RTTY", "PSK31", "FT8", "FT4", "JS8", "Q65", "JT65", "MSK144", "SSTV",
  "DMR", "C4FM", "DSTAR", "DATA", "OTHER",
];

/// 本台信息（站点级设置，应用到 ADIF 导出的台站字段）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StationInfo {
  /// 本台呼号 `STATION_CALLSIGN`。
  pub callsign: String,
  /// 操作员 `OPERATOR`（空则默认同呼号）。
  pub operator: String,
  /// 本台网格 `MY_GRIDSQUARE`。
  pub gridsquare: String,
  /// 设备 `MY_RIG`。
  pub rig: String,
  /// 天线 `MY_ANTENNA`。
  pub antenna: String,
}

/// 一条通联记录。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
  pub id: u64,
  /// 日期（YYYY-MM-DD）。
  pub date: String,
  /// 时间（HH:MM）。
  pub time: String,
  /// 结束时间 `TIME_OFF`（HH:MM）。
  #[serde(default)]
  pub time_off: String,
  /// 频率（MHz，文本）。
  pub freq: String,
  /// 波段（无频率时使用，如导入的 ADIF 只有 `BAND`）。
  #[serde(default)]
  pub band: String,
  /// 模式。
  pub mode: String,
  /// 对方呼号 `CALL`。
  pub callsign: String,
  /// 遗留：旧版单值 RST，加载后迁移到 `rst_sent` / `rst_rcvd`。
  #[serde(default)]
  pub rst: String,
  /// 发给对方的信号报告 `RST_SENT`。
  #[serde(default)]
  pub rst_sent: String,
  /// 收到对方的信号报告 `RST_RCVD`。
  #[serde(default)]
  pub rst_rcvd: String,
  /// 发射功率 `TX_PWR`（W）。
  #[serde(default)]
  pub tx_pwr: String,
  /// 对方网格 `GRIDSQUARE`。
  #[serde(default)]
  pub gridsquare: String,
  /// 对方操作员姓名 `NAME`。
  #[serde(default)]
  pub name: String,
  /// 对方 QTH。
  #[serde(default)]
  pub qth: String,
  /// 传播方式 `PROP_MODE`（如 `SAT`、`EME`、`ES`）。
  #[serde(default)]
  pub prop_mode: String,
  /// 卫星名 `SAT_NAME`。
  #[serde(default)]
  pub sat_name: String,
  /// `SOTA_REF`。
  #[serde(default)]
  pub sota_ref: String,
  /// `POTA_REF`。
  #[serde(default)]
  pub pota_ref: String,
  /// 竞赛 `CONTEST_ID`（Cabrillo 竞赛名，如 `CQ-WW-CW`）。
  #[serde(default)]
  pub contest_id: String,
  /// 发出的竞赛交换（序号 / 分区 / 功率等）`STX_STRING`。
  #[serde(default)]
  pub stx: String,
  /// 收到的竞赛交换 `SRX_STRING`。
  #[serde(default)]
  pub srx: String,
  /// DXCC 实体编号 `DXCC`（空则按呼号推断）。
  #[serde(default)]
  pub dxcc: String,
  /// CQ 分区 `CQZ`（空则取实体的主分区）。
  #[serde(default)]
  pub cqz: String,
  /// ITU 分区 `ITUZ`（空则取实体的主分区）。
  #[serde(default)]
  pub ituz: String,
  /// 备注 `COMMENT`。
  pub remark: String,
  /// QSL 卡片是否已寄出 `QSL_SENT`。
  #[serde(default)]
  pub qsl_sent: bool,
  /// QSL 是否已确认收到 `QSL_RCVD`。
  #[serde(default)]
  pub qsl_rcvd: bool,
}

impl LogEntry {
  /// 波段：优先按频率推算，其次使用记录中的 `band`；都没有时为空。
  #[must_use]
  pub fn band_label(&self) -> String {
    if let Ok(f) = self.freq.trim().parse::<f64>() {
      let b = band_of(f);
      if b != "其他" {
        return b.to_owned();
      }
    }
    self.band.trim().to_ascii_lowercase()
  }

  /// 同一次通联的判定 key（导入去重用）：呼号 + 日期 + 时间（到分钟）+ 波段 + 模式。
  #[must_use]
  pub fn qso_key(&self) -> String {
    format!(
      "{}|{}|{}|{}|{}",
      self.callsign.trim().to_ascii_uppercase(),
      self.date,
      self.time.get(..5).unwrap_or(&self.time),
      self.band_label(),
      self.mode.to_ascii_uppercase()
    )
  }

  /// DXCC 实体：记录里的 `DXCC` 编号优先，否则按呼号前缀推断。
  #[must_use]
  pub fn entity(&self) -> Option<&'static Entity> {
    self
      .dxcc
      .trim()
      .parse()
      .ok()
      .and_then(entity_by_dxcc)
      .or_else(|| lookup(&self.callsign))
  }

  /// CQ 分区：记录值优先，否则取实体主分区。
  #[must_use]
  pub fn cq_zone(&self) -> Option<u8> {
    zone(&self.cqz, 40).or_else(|| self.entity().map(|e| e.cq))
  }

  /// ITU 分区：记录值优先，否则取实体主分区。
  #[must_use]
  pub fn itu_zone(&self) -> Option<u8> {
    zone(&self.ituz, 90).or_else(|| self.entity().map(|e| e.itu))
  }

  /// 补全空的 DXCC / CQZ / ITUZ（按呼号推断），已有值不覆盖。
  pub fn fill_location(&mut self) {
    let Some(en) = self.entity() else {
      return;
    };
    if self.dxcc.trim().is_empty() {
      self.dxcc = en.dxcc.to_string();
    }
    if self.cqz.trim().is_empty() {
      self.cqz = en.cq.to_string();
    }
    if self.ituz.trim().is_empty() {
      self.ituz = en.itu.to_string();
    }
  }

  /// 旧版单值 `rst` 迁移到 `rst_sent` / `rst_rcvd`。
  pub fn migrate_rst(&mut self) {
    if self.rst_sent.is_empty() && self.rst_rcvd.is_empty() && !self.rst.is_empty() {
      self.rst_sent = self.rst.clone();
      self.rst_rcvd = self.rst.clone();
    }
  }

  /// 是否命中列表搜索关键词（`q` 已大写；呼号 / 姓名 / QTH / 网格 / 备注 / SOTA / POTA）。
  #[must_use]
  pub fn matches_query(&self, q: &str) -> bool {
    q.is_empty()
      || [
        &self.callsign,
        &self.name,
        &self.qth,
        &self.gridsquare,
        &self.remark,
        &self.sota_ref,
        &self.pota_ref,
      ]
      .iter()
      .any(|f| f.to_uppercase().contains(q))
  }
}

fn zone(s: &str, max: u8) -> Option<u8> {
  s.trim().parse().ok().filter(|z| (1..=max).contains(z))
}

/// 全部日志。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Logbook {
  pub entries: Vec<LogEntry>,
}

impl Logbook {
  /// 下一个不冲突的 ID。
  #[must_use]
  pub fn next_id(&self) -> u64 {
    self.entries.iter().map(|e| e.id).max().unwrap_or(0) + 1
  }

  /// 加载后的数据迁移。
  pub fn migrate(&mut self) {
    for e in &mut self.entries {
      e.migrate_rst();
    }
  }

  /// 导入 ADIF 记录，跳过与已有记录（或同批次）重复的通联，返回 `(导入数, 重复数)`。
  pub fn import(&mut self, records: Vec<AdifRecord>) -> (usize, usize) {
    let mut seen: HashSet<String> = self.entries.iter().map(LogEntry::qso_key).collect();
    let mut id = self.next_id();
    let (mut added, mut dupes) = (0, 0);
    for r in records {
      let mut entry = from_adif(r);
      if !seen.insert(entry.qso_key()) {
        dupes += 1;
        continue;
      }
      entry.fill_location();
      entry.id = id;
      id += 1;
      self.entries.push(entry);
      added += 1;
    }
    (added, dupes)
  }
}

/// 一个 ADIF 字段（空值不输出；长度按 ADIF 规范的字符数）。
fn adif_field(out: &mut String, name: &str, value: &str) {
  let value = value.trim();
  if !value.is_empty() {
    out.push_str(&format!("<{name}:{}>{value}", value.chars().count()));
  }
}

/// 把日志序列化为 ADIF 文本（台站字段写入每条记录）。
#[must_use]
pub fn export_adif(entries: &[LogEntry], station: &StationInfo) -> String {
  let mut s = String::from("Amateur radio logbook\n");
  adif_field(&mut s, "ADIF_VER", "3.1.4");
  adif_field(&mut s, "PROGRAMID", "HamExamWeb");
  s.push_str("<EOH>\n");
  for e in entries {
    adif_field(&mut s, "STATION_CALLSIGN", &station.callsign);
    adif_field(&mut s, "OPERATOR", &station.operator);
    adif_field(&mut s, "MY_GRIDSQUARE", &station.gridsquare);
    adif_field(&mut s, "MY_RIG", &station.rig);
    adif_field(&mut s, "MY_ANTENNA", &station.antenna);
    adif_field(&mut s, "QSO_DATE", &e.date.replace('-', ""));
    adif_field(&mut s, "TIME_ON", &e.time.replace(':', ""));
    adif_field(&mut s, "TIME_OFF", &e.time_off.replace(':', ""));
    adif_field(&mut s, "CALL", &e.callsign);
    adif_field(&mut s, "FREQ", &e.freq);
    adif_field(&mut s, "BAND", &e.band_label());
    let (mode, submode) = to_adif_mode(&e.mode);
    adif_field(&mut s, "MODE", &mode);
    adif_field(&mut s, "SUBMODE", submode.as_deref().unwrap_or_default());
    adif_field(&mut s, "RST_SENT", &e.rst_sent);
    adif_field(&mut s, "RST_RCVD", &e.rst_rcvd);
    adif_field(&mut s, "TX_PWR", &e.tx_pwr);
    adif_field(&mut s, "GRIDSQUARE", &e.gridsquare);
    adif_field(&mut s, "NAME", &e.name);
    adif_field(&mut s, "QTH", &e.qth);
    adif_field(&mut s, "PROP_MODE", &e.prop_mode);
    adif_field(&mut s, "SAT_NAME", &e.sat_name);
    adif_field(&mut s, "SOTA_REF", &e.sota_ref);
    adif_field(&mut s, "POTA_REF", &e.pota_ref);
    adif_field(&mut s, "CONTEST_ID", &e.contest_id);
    for (num, string, v) in [("STX", "STX_STRING", &e.stx), ("SRX", "SRX_STRING", &e.srx)] {
      if v.trim().parse::<u32>().is_ok() {
        adif_field(&mut s, num, v);
      }
      adif_field(&mut s, string, v);
    }
    if let Some(en) = e.entity() {
      adif_field(&mut s, "DXCC", &en.dxcc.to_string());
      adif_field(&mut s, "COUNTRY", en.name_en);
      adif_field(&mut s, "CONT", en.continent);
    }
    adif_field(
      &mut s,
      "CQZ",
      &e.cq_zone().map(|z| z.to_string()).unwrap_or_default(),
    );
    adif_field(
      &mut s,
      "ITUZ",
      &e.itu_zone().map(|z| z.to_string()).unwrap_or_default(),
    );
    adif_field(&mut s, "COMMENT", &e.remark);
    adif_field(&mut s, "QSL_SENT", if e.qsl_sent { "Y" } else { "N" });
    adif_field(&mut s, "QSL_RCVD", if e.qsl_rcvd { "Y" } else { "N" });
    s.push_str("<EOR>\n");
  }
  s
}

/// CSV 字段转义。
fn csv_field(s: &str) -> String {
  if s.contains(',') || s.contains('"') || s.contains('\n') {
    format!("\"{}\"", s.replace('"', "\"\""))
  } else {
    s.to_owned()
  }
}

/// 把日志序列化为 CSV 文本。
#[must_use]
pub fn export_csv(entries: &[LogEntry], station: &StationInfo) -> String {
  let mut s = String::from(
    "本台呼号,日期,时间,结束时间,频率,波段,模式,对方呼号,RST发送,RST接收,功率,网格,姓名,QTH,DXCC,CQ分区,ITU分区,传播方式,卫星,SOTA,POTA,备注,QSL寄出,QSL收到\n",
  );
  for e in entries {
    let entity = e.entity().map(|en| en.name).unwrap_or_default();
    let cq = e.cq_zone().map(|z| z.to_string()).unwrap_or_default();
    let itu = e.itu_zone().map(|z| z.to_string()).unwrap_or_default();
    let fields = [
      station.callsign.trim(),
      &e.date,
      &e.time,
      &e.time_off,
      &e.freq,
      &e.band_label(),
      &e.mode,
      &e.callsign,
      &e.rst_sent,
      &e.rst_rcvd,
      &e.tx_pwr,
      &e.gridsquare,
      &e.name,
      &e.qth,
      entity,
      &cq,
      &itu,
      &e.prop_mode,
      &e.sat_name,
      &e.sota_ref,
      &e.pota_ref,
      &e.remark,
      if e.qsl_sent { "Y" } else { "" },
      if e.qsl_rcvd { "Y" } else { "" },
    ];
    s.push_str(
      &fields
        .iter()
        .map(|f| csv_field(f))
        .collect::<Vec<_>>()
        .join(","),
    );
    s.push('\n');
  }
  s
}

/// `HHMM[SS]` → `HH:MM`。
fn adif_time(t: &str) -> String {
  match (t.get(0..2), t.get(2..4)) {
    (Some(h), Some(m)) if t.len() >= 4 => format!("{h}:{m}"),
    _ => t.to_owned(),
  }
}

/// 把 ADIF 记录转为本地日志条目（ID 由导入方分配）。
#[must_use]
pub fn from_adif(r: AdifRecord) -> LogEntry {
  let date = match (
    r.qso_date.get(0..4),
    r.qso_date.get(4..6),
    r.qso_date.get(6..8),
  ) {
    (Some(y), Some(m), Some(d)) if r.qso_date.len() == 8 => format!("{y}-{m}-{d}"),
    _ => r.qso_date,
  };
  LogEntry {
    date,
    time: adif_time(&r.time_on),
    time_off: adif_time(&r.time_off),
    freq: r.freq,
    band: r.band.to_ascii_lowercase(),
    mode: r.mode.to_uppercase(),
    callsign: r.call,
    rst_sent: r.rst_sent,
    rst_rcvd: r.rst_rcvd,
    tx_pwr: r.tx_pwr,
    gridsquare: r.gridsquare,
    name: r.name,
    qth: r.qth,
    prop_mode: r.prop_mode,
    sat_name: r.sat_name,
    sota_ref: r.sota_ref,
    pota_ref: r.pota_ref,
    contest_id: r.contest_id,
    stx: r.stx,
    srx: r.srx,
    dxcc: r.dxcc,
    cqz: r.cqz,
    ituz: r.ituz,
    remark: r.comment,
    qsl_sent: r.qsl_sent,
    qsl_rcvd: r.qsl_rcvd,
    ..Default::default()
  }
}

/// 到对方的距离（km）与方位角（度）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Path {
  pub km: f64,
  pub bearing: f64,
  /// `true` 表示对方位置取自实体中心（未填网格），只是粗略估计。
  pub approximate: bool,
}

/// 本台网格到对方的距离与方位：对方网格优先，否则用 DXCC 实体中心估算。
#[must_use]
pub fn path_to(my_grid: &str, their_grid: &str, callsign: &str) -> Option<Path> {
  let (lat1, lon1) = lat_lon_from_grid(my_grid.trim())?;
  let (lat2, lon2, approximate) = match lat_lon_from_grid(their_grid.trim()) {
    Some((lat, lon)) => (lat, lon, false),
    None => {
      let en = lookup(callsign)?;
      (en.lat, en.lon, true)
    }
  };
  let (km, bearing) = distance_bearing(lat1, lon1, lat2, lon2);
  Some(Path {
    km,
    bearing,
    approximate,
  })
}

/// 录入呼号时的历史提示。
#[derive(Debug, Clone, PartialEq)]
pub struct CallHint {
  pub entity: Option<&'static Entity>,
  /// 日志里从未通联过该实体。
  pub new_dxcc: bool,
  /// 通联过该实体，但不是这个波段。
  pub new_band: bool,
  /// 该呼号已通联次数。
  pub worked: usize,
  /// 最近一次通联：`日期 波段 模式`。
  pub last: Option<String>,
  /// 同波段同模式已联过。
  pub dupe: bool,
}

/// 根据日志计算录入提示；`skip` 为正在编辑的记录 ID（不与自己比较）。
#[must_use]
pub fn call_hint(
  entries: &[LogEntry],
  call: &str,
  band: &str,
  mode: &str,
  skip: Option<u64>,
) -> CallHint {
  let entity = lookup(call);
  let others: Vec<&LogEntry> = entries.iter().filter(|e| Some(e.id) != skip).collect();
  let same_call: Vec<&LogEntry> = others
    .iter()
    .copied()
    .filter(|e| e.callsign.eq_ignore_ascii_case(call))
    .collect();
  let (new_dxcc, new_band) = match entity {
    Some(en) => {
      let worked: Vec<&LogEntry> = others
        .iter()
        .copied()
        .filter(|e| e.entity().is_some_and(|x| x.dxcc == en.dxcc))
        .collect();
      let on_band = !band.is_empty() && worked.iter().any(|e| e.band_label() == band);
      (
        worked.is_empty(),
        !worked.is_empty() && !band.is_empty() && !on_band,
      )
    }
    None => (false, false),
  };
  let last = same_call
    .iter()
    .max_by(|a, b| (&a.date, &a.time).cmp(&(&b.date, &b.time)))
    .map(|e| format!("{} {} {}", e.date, e.band_label(), e.mode));
  CallHint {
    entity,
    new_dxcc,
    new_band,
    worked: same_call.len(),
    last,
    dupe: !band.is_empty()
      && same_call
        .iter()
        .any(|e| e.band_label() == band && e.mode.eq_ignore_ascii_case(mode)),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::adif::parse_adif;

  fn qso(id: u64, call: &str, freq: &str, mode: &str) -> LogEntry {
    LogEntry {
      id,
      date: "2026-09-30".into(),
      time: "12:34".into(),
      freq: freq.into(),
      mode: mode.into(),
      callsign: call.into(),
      ..Default::default()
    }
  }

  #[test]
  fn band_label_prefers_frequency() {
    let mut e = qso(1, "K1AA", "14.074", "FT8");
    e.band = "40M".into();
    assert_eq!(e.band_label(), "20m");
    e.freq.clear();
    assert_eq!(e.band_label(), "40m");
  }

  #[test]
  fn import_skips_existing_and_batch_duplicates() {
    let mut lb = Logbook {
      entries: vec![qso(5, "K1AA", "14.074", "FT8")],
    };
    let adif = "<EOH>\
      <CALL:4>K1AA<QSO_DATE:8>20260930<TIME_ON:6>123459<FREQ:6>14.075<MODE:3>FT8<EOR>\
      <CALL:4>JA1X<QSO_DATE:8>20260930<TIME_ON:4>1300<BAND:3>40M<MODE:2>CW<EOR>\
      <CALL:4>ja1x<QSO_DATE:8>20260930<TIME_ON:4>1300<BAND:3>40m<MODE:2>cw<EOR>";
    let (added, dupes) = lb.import(parse_adif(adif));
    assert_eq!((added, dupes), (1, 2));
    let ja = &lb.entries[1];
    assert_eq!(
      (ja.id, ja.time.as_str(), ja.band_label()),
      (6, "13:00", "40m".to_owned())
    );
    assert_eq!((ja.dxcc.as_str(), ja.cqz.as_str()), ("339", "25"));
  }

  #[test]
  fn location_fields_override_lookup() {
    let mut e = qso(1, "W1AW", "", "CW");
    assert_eq!(e.entity().map(|x| x.dxcc), Some(291));
    assert_eq!(e.cq_zone(), Some(5));
    e.cqz = "4".into();
    e.fill_location();
    assert_eq!((e.dxcc.as_str(), e.cqz.as_str()), ("291", "4"));
    e.cqz = "99".into();
    assert_eq!(e.cq_zone(), Some(5));
  }

  #[test]
  fn adif_export_roundtrip() {
    let station = StationInfo {
      callsign: "BG4XXX".into(),
      gridsquare: "OM89".into(),
      ..Default::default()
    };
    let mut e = qso(1, "JA1X", "7.010", "FT4");
    e.name = "太郎".into();
    e.qsl_rcvd = true;
    let text = export_adif(&[e.clone()], &station);
    assert!(text.contains("<STATION_CALLSIGN:6>BG4XXX"));
    assert!(text.contains("<MODE:4>MFSK<SUBMODE:3>FT4"));
    assert!(text.contains("<DXCC:3>339<COUNTRY:5>Japan<CONT:2>AS<CQZ:2>25<ITUZ:2>45"));
    let back = from_adif(parse_adif(&text).remove(0));
    assert_eq!(
      (
        back.callsign.as_str(),
        back.mode.as_str(),
        back.name.as_str(),
        back.band.as_str()
      ),
      ("JA1X", "FT4", "太郎", "40m")
    );
    assert!(back.qsl_rcvd && !back.qsl_sent);
    assert_eq!(back.qso_key(), e.qso_key());
  }

  #[test]
  fn csv_escapes_fields() {
    let mut e = qso(1, "K1AA", "14.074", "FT8");
    e.remark = "hello, \"world\"".into();
    let csv = export_csv(&[e], &StationInfo::default());
    assert!(
      csv
        .lines()
        .nth(1)
        .expect("row")
        .ends_with(",\"hello, \"\"world\"\"\",,")
    );
  }

  #[test]
  fn hints_new_dxcc_band_and_dupes() {
    let log = vec![
      qso(1, "JA1X", "14.074", "FT8"),
      qso(2, "JA2Y", "14.074", "CW"),
    ];
    let h = call_hint(&log, "VK2ABC", "20m", "FT8", None);
    assert!(h.new_dxcc && !h.new_band && h.worked == 0);
    let h = call_hint(&log, "JA3Z", "40m", "FT8", None);
    assert!(!h.new_dxcc && h.new_band);
    let h = call_hint(&log, "ja1x", "20m", "FT8", None);
    assert!(h.dupe && h.worked == 1);
    assert_eq!(h.last.as_deref(), Some("2026-09-30 20m FT8"));
    assert!(!call_hint(&log, "JA1X", "20m", "FT8", Some(1)).dupe);
  }

  #[test]
  fn path_uses_grid_or_entity_center() {
    let p = path_to("OM89", "PM95", "JA1X").expect("path");
    assert!(!p.approximate && p.km > 1500.0 && p.km < 2500.0);
    assert!(p.bearing > 60.0 && p.bearing < 120.0);
    assert!(path_to("OM89", "", "JA1X").expect("path").approximate);
    assert_eq!(path_to("", "PM95", "JA1X"), None);
  }

  #[test]
  fn legacy_json_migrates_rst() {
    let json = r#"{"entries":[{"id":1,"date":"2026-01-01","time":"00:00","freq":"","mode":"CW","callsign":"K1AA","rst":"599","remark":""}]}"#;
    let mut lb: Logbook = serde_json::from_str(json).expect("legacy");
    lb.migrate();
    assert_eq!(
      (
        lb.entries[0].rst_sent.as_str(),
        lb.entries[0].rst_rcvd.as_str()
      ),
      ("599", "599")
    );
    assert!(lb.entries[0].cqz.is_empty());
  }
}
