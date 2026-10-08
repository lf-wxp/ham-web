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
use crate::qsl_status::QslVia;
use crate::station::StationBook;
use crate::study_plan::{day_number, format_day};

/// `HH:MM` → `(时, 分)`；格式或取值不合法时返回 `None`。
///
/// 时间字段的**唯一**解析入口：热力图的小时分布、体检的「时间在未来」判定、
/// [`LogEntry::minutes_on`] 都走它。各写一份的话容忍的写法会不一致 —— 曾经就是
/// （体检用 `split_once(':')`、热力图用 `get(..2)`），于是「体检查不出问题、
/// 热力图却少一条」。
#[must_use]
pub fn split_hhmm(time: &str) -> Option<(u32, u32)> {
  let (h, m) = time.trim().split_once(':')?;
  let h: u32 = h.trim().parse().ok()?;
  let m: u32 = m.trim().parse().ok()?;
  (h < 24 && m < 60).then_some((h, m))
}

/// `YYYY-MM-DD` + `HH:MM` → 自 1970-01-01 起经过的分钟数。
#[must_use]
pub fn minutes_of(date: &str, time: &str) -> Option<i64> {
  let days = day_number(date)?;
  let (h, m) = split_hhmm(time)?;
  Some(days * 1440 + i64::from(h) * 60 + i64::from(m))
}

/// 分钟数 → `YYYY-MM-DD` + `HH:MM`（[`minutes_of`] 的逆运算）。
#[must_use]
pub fn stamp(minutes: i64) -> (String, String) {
  let (days, rest) = (minutes.div_euclid(1440), minutes.rem_euclid(1440));
  (
    format_day(days),
    format!("{:02}:{:02}", rest / 60, rest % 60),
  )
}

/// 时间文本 → 统一的 `HH:MM`；解析不出时原样返回（宁可留着怪写法，也不要凭空改记录）。
///
/// 让 `8:30` 与 `08:30` 落到同一个 key：两者是同一次通联，写法不同就判成两条会
/// 在同步时补出一条重复记录。
fn normalized_hhmm(time: &str) -> String {
  match split_hhmm(time) {
    Some((h, m)) => format!("{h:02}:{m:02}"),
    None => time.trim().to_owned(),
  }
}

/// 日期文本 → 统一的 `YYYY-MM-DD`；解析不出时原样返回（宁可留着怪写法，也不要凭空改记录）。
///
/// 与 [`normalized_hhmm`] 同一个理由：`2026-1-5` 与 `2026-01-05` 是同一天，写法不同就
/// 判成两条、同步时补出一条重复记录。归一化复用 [`day_number`] / [`format_day`]，与
/// [`LogEntry::dedupe_key`] 的「日期必须可解析」是同一口径 —— 否则会出现「能通过
/// `dedupe_key` 的资格检查、却与报告行的 key 对不上」的半吊子状态。
fn normalized_date(date: &str) -> String {
  match day_number(date) {
    Some(days) => format_day(days),
    None => date.trim().to_owned(),
  }
}

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
  /// 操作员 `OPERATOR`（多操作员分账用）。空 = 用所属台站档案的操作员。
  ///
  /// ADIF 里 `OPERATOR` 本就**逐条**出现（俱乐部台 / 家庭台多人操作是常态），
  /// 所以它跟归属台站一样是记录级字段，而不是整库一个值。
  #[serde(default)]
  pub operator: String,
  /// 归属台站（[`crate::station::StationProfile::id`]）。
  ///
  /// `0` = 未指定：老日志没有这个字段，删掉的台站也会留下悬空 id —— 两种都按「当前台站」
  /// 回落（见 [`crate::station::StationBook::of_entry`]），因此不需要数据迁移。
  #[serde(default)]
  pub station_id: u64,
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
  /// 美国州 `STATE`（WAS 奖状）。
  #[serde(default)]
  pub state: String,
  /// IOTA 岛屿编号 `IOTA`。
  #[serde(default)]
  pub iota: String,
  /// 备注 `COMMENT`。
  pub remark: String,
  /// QSL 卡片是否已寄出 `QSL_SENT`。
  #[serde(default)]
  pub qsl_sent: bool,
  /// 纸卡寄出方式 `QSL_SENT_VIA`（卡片局 / 直寄 / 未知）。
  ///
  /// 只有 `qsl_sent` 为真时才有意义；旧数据与不带该字段的 ADIF 一律落到「未知」。
  #[serde(default)]
  pub qsl_sent_via: QslVia,
  /// QSL 卡片是否已确认收到 `QSL_RCVD`。
  #[serde(default)]
  pub qsl_rcvd: bool,
  /// LoTW 是否已上传 `LOTW_QSL_SENT`。
  #[serde(default)]
  pub lotw_sent: bool,
  /// LoTW 是否已确认 `LOTW_QSL_RCVD`。
  #[serde(default)]
  pub lotw_rcvd: bool,
  /// eQSL 是否已寄出 `EQSL_QSL_SENT`。
  #[serde(default)]
  pub eqsl_sent: bool,
  /// eQSL 是否已确认 `EQSL_QSL_RCVD`。
  #[serde(default)]
  pub eqsl_rcvd: bool,
  /// QRZ Logbook 是否已确认。
  ///
  /// ADIF 没有这个标准字段（QRZ 的站内确认是与纸质卡共用的 `QSL_RCVD`），所以走本工具的
  /// 应用自定义字段 [`crate::adif::APP_QRZ_RCVD`]：别的软件忽略它，导出再导入时原样读回。
  #[serde(default)]
  pub qrz_rcvd: bool,
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

  /// 同一次通联的判定 key：呼号 + 日期 + 时间（到分钟）+ 波段 + 模式。
  ///
  /// 只在日期与时间都能解析时才是可靠的身份判据 —— 缺任一项时它会退化成
  /// 「呼号|||波段|模式」，把同一台站在同一波段模式上的多次通联撞成一条。需要拿它
  /// 当去重 / 匹配依据的地方一律用 [`LogEntry::dedupe_key`]。
  #[must_use]
  pub fn qso_key(&self) -> String {
    format!(
      "{}|{}|{}|{}|{}",
      self.callsign.trim().to_ascii_uppercase(),
      normalized_date(&self.date),
      normalized_hhmm(&self.time),
      self.band_label(),
      self.mode.trim().to_ascii_uppercase()
    )
  }

  /// 去重 / 跨来源匹配用的 key：日期与时间都能解析时才有值。
  ///
  /// 四个调用方（导入去重、备份合并、QSL 同步、日志体检）拿它当「这是不是同一次通联」
  /// 的身份判据，而误判的代价都是**丢记录** —— 导入静默跳过、备份合并与体检直接删。
  /// 日期或时间缺失时宁可漏判一条重复，也不要合并两条真实通联，所以这里返回 `None`
  /// 让调用方跳过判定。
  #[must_use]
  pub fn dedupe_key(&self) -> Option<String> {
    self.minutes_on()?;
    Some(self.qso_key())
  }

  /// 通联时刻（自 1970-01-01 起经过的分钟数）；日期或时间不合法时为 `None`。
  #[must_use]
  pub fn minutes_on(&self) -> Option<i64> {
    minutes_of(&self.date, &self.time)
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

  /// 是否已被任一途径（纸卡 / LoTW / eQSL）确认。
  #[must_use]
  pub fn confirmed(&self) -> bool {
    self.qsl_rcvd || self.lotw_rcvd || self.eqsl_rcvd || self.qrz_rcvd
  }

  /// 是否命中列表搜索关键词（`q` 已大写；呼号 / 姓名 / QTH / 网格 / 备注 / SOTA / POTA / DXCC 实体名）。
  #[must_use]
  pub fn matches_query(&self, q: &str) -> bool {
    if q.is_empty() {
      return true;
    }
    if [
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
    {
      return true;
    }
    // DXCC 实体名（中文 / 英文），支持从 DXCC 地图点击实体后按实体过滤。
    self
      .entity()
      .is_some_and(|e| e.name.contains(q) || e.name_en.to_uppercase().contains(q))
  }
}

/// 已寄出（纸卡 / LoTW / eQSL）但尚未确认收到 QSL 的通联（待追卡清单）。
#[must_use]
pub fn pending_qsl(entries: &[LogEntry]) -> Vec<&LogEntry> {
  entries
    .iter()
    .filter(|e| (e.qsl_sent || e.lotw_sent || e.eqsl_sent) && !e.confirmed())
    .collect()
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

  /// 导入 ADIF 记录（按 [`LogEntry::dedupe_key`] 去重），跳过与已有记录（或同批次）
  /// 重复的通联，并把台站归属认领到 `stations`，返回 `(导入数, 重复数)`。
  ///
  /// 台站归属要在 [`from_adif`] 消费记录**之前**取：报告里的 `STATION_CALLSIGN` /
  /// `MY_GRIDSQUARE` 是这份 ADIF 自带的身份信息，认领后 `LogEntry` 只留一个本地 id。
  pub fn import(&mut self, records: Vec<AdifRecord>, stations: &mut StationBook) -> (usize, usize) {
    let mut seen: HashSet<String> = self
      .entries
      .iter()
      .filter_map(LogEntry::dedupe_key)
      .collect();
    let mut id = self.next_id();
    let (mut added, mut dupes) = (0, 0);
    for r in records {
      let station_callsign = r.station_callsign.clone();
      let my_gridsquare = r.my_gridsquare.clone();
      let mut entry = from_adif(r);
      // 没有 `dedupe_key`（日期或时间缺失）就说不上重不重复，一律导入 —— 见
      // `LogEntry::dedupe_key`：这条 ADIF 里没有日期时，把整批同呼号同波段的记录
      // 当成一条吞掉，比放进去几条「待补日期」的记录糟糕得多。
      if entry.dedupe_key().is_some_and(|key| !seen.insert(key)) {
        dupes += 1;
        continue;
      }
      // 认领放在去重之后：重复行不该在档案册里留下新台站 —— 导入一份全是重复项的
      // ADIF 时用户以为什么都没发生，档案册却悄悄多出几条，两者对不上。
      let station_id = stations.claim(&station_callsign, &my_gridsquare);
      entry.fill_location();
      entry.id = id;
      entry.station_id = station_id;
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

/// 把日志序列化为 ADIF 文本（台站字段**按每条记录自己的台站**写入）。
///
/// 台站字段放在每条记录里而不是文件头：一次 POTA 野外通联与家里的固定台可以同在一个日志，
/// 导出成一份 ADIF 时只有逐条写才对得上（ADIF 本来也允许 `STATION_CALLSIGN` 逐条出现）。
#[must_use]
pub fn export_adif(entries: &[LogEntry], stations: &StationBook) -> String {
  let mut s = String::from("Amateur radio logbook\n");
  adif_field(&mut s, "ADIF_VER", "3.1.4");
  adif_field(&mut s, "PROGRAMID", "HamExamWeb");
  s.push_str("<EOH>\n");
  for e in entries {
    let profile = stations.of_entry(e);
    let station = profile.info();
    adif_field(&mut s, "STATION_CALLSIGN", &station.callsign);
    // 逐条写：记录里填了 `OPERATOR` 就写它，否则回落到所属台站档案的操作员。
    let operator = if e.operator.trim().is_empty() {
      station.operator.as_str()
    } else {
      e.operator.trim()
    };
    adif_field(&mut s, "OPERATOR", operator);
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
    adif_field(&mut s, "STATE", &e.state);
    adif_field(&mut s, "IOTA", &e.iota);
    adif_field(&mut s, "COMMENT", &e.remark);
    adif_field(&mut s, "QSL_SENT", if e.qsl_sent { "Y" } else { "N" });
    // 寄出方式只在确实记了的时候写出（`adif_field` 会跳过空值）。
    if let Some(code) = e.qsl_sent_via.adif_code() {
      adif_field(&mut s, "QSL_SENT_VIA", code);
    }
    adif_field(&mut s, "QSL_RCVD", if e.qsl_rcvd { "Y" } else { "N" });
    adif_field(&mut s, "LOTW_QSL_SENT", if e.lotw_sent { "Y" } else { "N" });
    adif_field(&mut s, "LOTW_QSL_RCVD", if e.lotw_rcvd { "Y" } else { "N" });
    adif_field(&mut s, "EQSL_QSL_SENT", if e.eqsl_sent { "Y" } else { "N" });
    adif_field(&mut s, "EQSL_QSL_RCVD", if e.eqsl_rcvd { "Y" } else { "N" });
    // 应用自定义字段：始终写出 Y/N，这样任何一份本工具导出的 ADIF 都是「能代表 QRZ 渠道」的。
    adif_field(
      &mut s,
      crate::adif::APP_QRZ_RCVD,
      if e.qrz_rcvd { "Y" } else { "N" },
    );
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
pub fn export_csv(entries: &[LogEntry], stations: &StationBook) -> String {
  let mut s = String::from(
    "本台呼号,日期,时间,结束时间,频率,波段,模式,对方呼号,RST发送,RST接收,功率,网格,姓名,QTH,DXCC,CQ分区,ITU分区,传播方式,卫星,SOTA,POTA,备注,QSL寄出,QSL收到\n",
  );
  for e in entries {
    let entity = e.entity().map(|en| en.name).unwrap_or_default();
    let cq = e.cq_zone().map(|z| z.to_string()).unwrap_or_default();
    let itu = e.itu_zone().map(|z| z.to_string()).unwrap_or_default();
    let fields = [
      stations.of_entry(e).callsign.trim(),
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
///
/// 只认前四位全是数字的写法：`12:30` 这类「已经带冒号」的输入按原文返回 —— 位置切片会
/// 切出 `12::3` 这种坏值，它随后既拿不到 `dedupe_key`（同步时被当成「本地缺失」）、
/// 又会从热力图上消失。校验交给 `split_hhmm` 的调用方，这里只保证不改坏原文。
fn adif_time(t: &str) -> String {
  let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
  match (t.get(0..2), t.get(2..4)) {
    (Some(h), Some(m)) if t.len() >= 4 && digits(h) && digits(m) => format!("{h}:{m}"),
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
    state: r.state,
    iota: r.iota,
    remark: r.comment,
    operator: r.operator.trim().to_ascii_uppercase(),
    qsl_sent: r.qsl_sent,
    qsl_sent_via: r.qsl_sent_via,
    qsl_rcvd: r.qsl_rcvd,
    lotw_sent: r.lotw_sent,
    lotw_rcvd: r.lotw_rcvd,
    eqsl_sent: r.eqsl_sent,
    eqsl_rcvd: r.eqsl_rcvd,
    qrz_rcvd: r.qrz_rcvd,
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
  fn matches_query_by_entity_name() {
    let e = qso(1, "BG1AA", "14.074", "FT8");
    assert!(e.matches_query("BG1")); // 呼号
    assert!(e.matches_query("中国")); // 实体中文名（BG1AA → 中国）
    assert!(e.matches_query("CHINA")); // 实体英文名（q 已大写）
    assert!(!e.matches_query("日本"));
    assert!(e.matches_query("")); // 空查询命中全部
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
    let (added, dupes) = lb.import(parse_adif(adif), &mut StationBook::default());
    assert_eq!((added, dupes), (1, 2));
    let ja = &lb.entries[1];
    assert_eq!(
      (ja.id, ja.time.as_str(), ja.band_label()),
      (6, "13:00", "40m".to_owned())
    );
    assert_eq!((ja.dxcc.as_str(), ja.cqz.as_str()), ("339", "25"));
  }

  #[test]
  fn duplicate_imports_do_not_claim_stations() {
    // 全部是重复行时，档案册不该多出任何台站：用户以为「什么都没导入」，
    // 档案册悄悄多几条会对不上。
    let mut lb = Logbook {
      entries: vec![qso(5, "K1AA", "14.074", "FT8")],
    };
    let mut book = StationBook::default();
    let before = book.clone();
    let adif = "<EOH>\
      <CALL:4>K1AA<STATION_CALLSIGN:6>BG4XXX<MY_GRIDSQUARE:6>OL99AA<QSO_DATE:8>20260930<TIME_ON:6>123459<FREQ:6>14.075<MODE:3>FT8<EOR>";
    let (added, dupes) = lb.import(parse_adif(adif), &mut book);
    assert_eq!((added, dupes), (0, 1));
    assert_eq!(book, before, "重复行不该动档案册");

    // 新通联照常认领：同一份带本台字段的 ADIF 里出现新记录时会新建档案。
    let adif = "<EOH>\
      <CALL:4>K1AA<STATION_CALLSIGN:6>BG4XXX<MY_GRIDSQUARE:6>OL99AA<QSO_DATE:8>20260930<TIME_ON:6>123459<FREQ:6>14.075<MODE:3>FT8<EOR>\
      <CALL:4>JA1X<STATION_CALLSIGN:6>BG4XXX<MY_GRIDSQUARE:6>OL99AA<QSO_DATE:8>20260930<TIME_ON:4>1300<BAND:3>40M<MODE:2>CW<EOR>";
    let (added, dupes) = lb.import(parse_adif(adif), &mut book);
    assert_eq!((added, dupes), (1, 1));
    assert!(
      book.profiles.iter().any(|p| p.matches("BG4XXX", "OL99AA")),
      "新通联要认领出台站"
    );
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
    let text = export_adif(&[e.clone()], &StationBook::from_info(&station));
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
  fn adif_roundtrips_the_qrz_extension_field() {
    // QRZ 确认没有标准 ADIF 字段，走本工具的应用自定义字段：导出写、导入读回来。
    let mut e = qso(1, "JA1X", "14.074", "FT8");
    e.qrz_rcvd = true;
    let text = export_adif(&[e], &StationBook::default());
    assert!(text.contains("<APP_HAMEXAMWEB_QRZ_RCVD:1>Y"), "{text}");
    assert!(from_adif(parse_adif(&text).remove(0)).qrz_rcvd);

    // 没确认时写 N（而不是省略）：任何一份本工具导出的 ADIF 都能代表 QRZ 渠道，
    // 否则喂回来会被判成「只做匹配分析」（见 `qsl_sync` 的证据规则）。
    let plain = qso(2, "W1AW", "14.074", "SSB");
    let text = export_adif(&[plain], &StationBook::default());
    assert!(text.contains("<APP_HAMEXAMWEB_QRZ_RCVD:1>N"), "{text}");
    assert!(!from_adif(parse_adif(&text).remove(0)).qrz_rcvd);

    // 别的软件不写这个字段时，读进来就是「没确认」，且不影响其它标志位。
    let foreign =
      "<EOH>\n<CALL:4>JA1X<QSO_DATE:8>20260930<TIME_ON:4>1234<MODE:3>FT8<QSL_RCVD:1>Y<EOR>";
    let r = from_adif(parse_adif(foreign).remove(0));
    assert!(!r.qrz_rcvd && r.qsl_rcvd);
  }

  #[test]
  fn adif_roundtrips_the_qsl_sent_via() {
    // 记了寄出方式：导出要写 `QSL_SENT_VIA`，导入要读回来。
    let mut e = qso(1, "JA1X", "14.074", "FT8");
    e.qsl_sent = true;
    e.qsl_sent_via = QslVia::Bureau;
    let text = export_adif(&[e], &StationBook::default());
    assert!(text.contains("<QSL_SENT_VIA:1>B"), "{text}");
    assert_eq!(
      from_adif(parse_adif(&text).remove(0)).qsl_sent_via,
      QslVia::Bureau
    );

    // 没记方式时不能凭空多出一行（旧数据往返保持原样）。
    let plain = qso(2, "W1AW", "14.074", "SSB");
    let text = export_adif(&[plain], &StationBook::default());
    assert!(!text.contains("QSL_SENT_VIA"), "{text}");
  }

  #[test]
  fn adif_roundtrips_per_entry_station_attribution() {
    // 一个日志里两个台站：同呼号、不同网格（家里固定台与 POTA 野外）。
    let mut book = StationBook::from_info(&StationInfo {
      callsign: "BG4XXX".into(),
      gridsquare: "OM89EW".into(),
      ..Default::default()
    });
    let home = book.active_id;
    let field = book.claim("BG4XXX", "OL99AA");
    assert_ne!(home, field);
    let mut a = qso(1, "JA1X", "14.074", "FT8");
    a.station_id = home;
    let mut b = qso(2, "JA2Y", "7.074", "FT8");
    b.station_id = field;

    // 台站字段逐条写：同一份 ADIF 里两个台站各自的网格都要在。
    let text = export_adif(&[a, b], &book);
    assert!(text.contains("<MY_GRIDSQUARE:6>OM89EW"), "{text}");
    assert!(text.contains("<MY_GRIDSQUARE:6>OL99AA"), "{text}");

    // 导入到另一台机器：按「呼号 + 网格」认领出两条档案，归属各自对得上
    // （只按呼号认领会把野外那条并回家里）。
    let mut fresh = Logbook::default();
    let mut another = StationBook::default();
    assert_eq!(fresh.import(parse_adif(&text), &mut another), (2, 0));
    assert_eq!(another.profiles.len(), 3, "默认那条 + 认领出的两条");
    assert_ne!(
      fresh.entries[0].station_id, fresh.entries[1].station_id,
      "两条通联要归属到不同的台站"
    );
    assert_eq!(another.of_entry(&fresh.entries[0]).gridsquare, "OM89EW");
    assert_eq!(another.of_entry(&fresh.entries[1]).gridsquare, "OL99AA");
  }

  #[test]
  fn csv_escapes_fields() {
    let mut e = qso(1, "K1AA", "14.074", "FT8");
    e.remark = "hello, \"world\"".into();
    let csv = export_csv(&[e], &StationBook::default());
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

  #[test]
  fn pending_qsl_filters_sent_but_unconfirmed() {
    let mut a = qso(1, "JA1X", "14.074", "FT8");
    a.qsl_sent = true;
    let mut b = qso(2, "W1AW", "14.074", "SSB");
    b.lotw_sent = true;
    b.lotw_rcvd = true;
    let c = qso(3, "DL1A", "7.010", "CW");
    let arr = [a, b, c];
    let pending = pending_qsl(&arr);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].callsign, "JA1X");
  }
}
