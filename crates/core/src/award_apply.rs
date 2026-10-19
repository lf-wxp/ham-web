//! 奖状申请记录表：把日志整理成「每个 DXCC 实体一条代表通联」。
//!
//! 这是提交 DXCC 时核对用的一张表，**不是可直接上传的申请表** —— ARRL 的在线申请
//! 没有公开的 CSV / 文本格式，本站不编一个看起来像官方格式的东西。表格的用途是：
//! 逐条核对实体、日期、波段与 QSL 方式，打印后附在卡片里寄出。

use std::collections::{HashMap, hash_map::Entry};

use crate::dxcc::Entity;
use crate::logbook::LogEntry;

/// 这条记录靠什么确认。
///
/// 刻意**不**派生 `Ord`：按 `via` 排序没有业务含义，而派生顺序（`None < Card < Lotw <
/// Eqsl < Qrz`）与 [`apply_via`] 的优先次序（LoTW > 卡片 > eQSL > QRZ）并不一致 ——
/// 谁拿 `max()` 选途径就会选到 QRZ。需要优先次序时用 [`apply_via`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyVia {
  /// 尚未确认。
  None,
  /// 纸质卡片。
  Card,
  /// LoTW。
  Lotw,
  /// eQSL。
  Eqsl,
  /// QRZ Logbook。
  Qrz,
}

/// 一行申请记录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyRow {
  /// DXCC 实体名（中文；页面按界面语言再翻）。
  pub entity: &'static str,
  pub entity_en: &'static str,
  pub callsign: String,
  pub date: String,
  pub band: String,
  pub mode: String,
  pub via: ApplyVia,
}

/// 一条记录的确认途径；多种并存时按 DXCC 的优先次序取一个。
#[must_use]
pub fn apply_via(entry: &LogEntry) -> ApplyVia {
  if entry.lotw_rcvd {
    ApplyVia::Lotw
  } else if entry.qsl_rcvd {
    ApplyVia::Card
  } else if entry.eqsl_rcvd {
    ApplyVia::Eqsl
  } else if entry.qrz_rcvd {
    ApplyVia::Qrz
  } else {
    ApplyVia::None
  }
}

/// `candidate` 是否比 `current` 更早。
///
/// 不能比字符串：日志里 `2026-1-5` 与 `2026-01-05` 是同一天（手改、备份恢复、第三方
/// ADIF 都会带进来，见 `logbook::normalized_date` 的说明），逐字符比时 `'-' < '2'`
/// 会把 1 月 5 日判成早于 12 月 1 日。一律走 [`LogEntry::minutes_on`] 换算成时刻；
/// 解析不出的记录排在「已知时刻」之后 —— 畸形日期不该抢走「日期最早」这一条。
fn earlier(candidate: &LogEntry, current: &LogEntry) -> bool {
  match (candidate.minutes_on(), current.minutes_on()) {
    (Some(a), Some(b)) => a < b,
    (Some(_), None) => true,
    (None, Some(_)) => false,
    // 两条都解析不出：退回字符串比较，至少保证排序是确定的。
    (None, None) => (&candidate.date, &candidate.time) < (&current.date, &current.time),
  }
}

/// 是否拿到了 **DXCC 认可的**确认：只有纸质卡片与 LoTW 计入，eQSL / QRZ 不算。
///
/// 刻意不用 [`LogEntry::confirmed`]（它把四个来源都算确认）：这张表的用途是照着 `via`
/// 一列寄卡，若挑了只被 QRZ 确认的一条，ARRL 会直接拒收。
fn dxcc_confirmed(entry: &LogEntry) -> bool {
  entry.qsl_rcvd || entry.lotw_rcvd
}

/// 同一实体只留一条：**DXCC 认可的确认优先，其次日期最早**。
fn better(candidate: &LogEntry, current: &LogEntry) -> bool {
  match (dxcc_confirmed(candidate), dxcc_confirmed(current)) {
    (true, false) => true,
    (false, true) => false,
    _ => earlier(candidate, current),
  }
}

/// 生成 DXCC 记录表，按**英文实体名**排序。
///
/// 排序键刻意不用中文名：表在 en / es 界面显示英文名，按中文码位排出来的顺序在那边
/// 看起来是乱序；中文名之间也只是码位序而不是拼音序，两种语言都不讨好。英文名是
/// 这张表交给 ARRL 时实际使用的名称（CSV 也导它），按它排序在三种界面下都说得清。
#[must_use]
pub fn dxcc_record_sheet(entries: &[LogEntry]) -> Vec<ApplyRow> {
  // 按 DXCC 号建索引：上万条日志 × 三百多个实体时，逐条线性 `find` 是 O(n × 实体数)。
  // 顺手把已查到的实体一起存下来，避免生成行时再调一次 `entity()`（又一次前缀查询）。
  let mut best: HashMap<u16, (&'static Entity, &LogEntry)> = HashMap::new();
  for entry in entries {
    let Some(entity) = entry.entity() else {
      continue;
    };
    match best.entry(entity.dxcc) {
      Entry::Occupied(mut slot) => {
        if better(entry, slot.get().1) {
          slot.insert((entity, entry));
        }
      }
      Entry::Vacant(slot) => {
        slot.insert((entity, entry));
      }
    }
  }

  let mut rows: Vec<ApplyRow> = best
    .into_values()
    .map(|(entity, entry)| ApplyRow {
      entity: entity.name,
      entity_en: entity.name_en,
      callsign: entry.callsign.clone(),
      date: entry.date.clone(),
      band: entry.band_label(),
      mode: entry.mode.clone(),
      via: apply_via(entry),
    })
    .collect();
  rows.sort_by(|a, b| (a.entity_en, a.entity).cmp(&(b.entity_en, b.entity)));
  rows
}

impl ApplyVia {
  /// 导出 CSV 用的标签：固定英文，三种界面语言下一致（这份文件是交给 ARRL 的）。
  #[must_use]
  pub const fn csv_label(self) -> &'static str {
    match self {
      Self::None => "none",
      Self::Card => "card",
      Self::Lotw => "LoTW",
      Self::Eqsl => "eQSL",
      Self::Qrz => "QRZ",
    }
  }
}

/// 会让 Excel / Sheets 把单元格当公式执行的开头字符（OWASP「CSV Injection」口径）。
const FORMULA_LEAD: [char; 7] = ['=', '+', '-', '@', '\t', '\r', '\n'];

/// CSV 字段：一律加引号，并给公式开头的值前置一个单引号。
///
/// 一律加引号是因为实体名里偶尔有逗号，不去猜；前置 `'` 是因为呼号等字段来自用户输入，
/// 不能假设它一定是字母开头——`=HYPERLINK(…)` 这类值在表格软件里会被当成公式执行。
#[must_use]
pub fn csv_field(value: &str) -> String {
  // 判前缀时同时看原串与去掉前导空白后的串：表格软件对 `" =HYPERLINK(…)"` 的处理不一致，
  // 只看首字符是已知绕过面；而 `\t` / `\r` / `\n` 本身就在前缀表里，被 `trim_start` 吃掉后
  // 反而漏判。前置的单引号仍加在原串最前面，不吃掉用户的空白。
  let text = if value.starts_with(FORMULA_LEAD) || value.trim_start().starts_with(FORMULA_LEAD) {
    format!("'{value}")
  } else {
    value.to_owned()
  };
  format!("\"{}\"", text.replace('"', "\"\""))
}

/// 申请记录表 → CSV。
///
/// 实体用**英文名**：界面按语言显示实体名，但这份 CSV 是交给 ARRL（英文）的，而且同一份
/// 导出在 zh / en / es 三种界面下应当一致。
#[must_use]
pub fn to_csv(rows: &[ApplyRow]) -> String {
  let mut out = String::from("entity,callsign,date,band,mode,qsl_via\n");
  for row in rows {
    let fields = [
      csv_field(row.entity_en),
      csv_field(&row.callsign),
      csv_field(&row.date),
      csv_field(&row.band),
      csv_field(&row.mode),
      csv_field(row.via.csv_label()),
    ];
    out.push_str(&fields.join(","));
    out.push('\n');
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  fn entry(call: &str, date: &str, freq: &str) -> LogEntry {
    LogEntry {
      callsign: call.to_owned(),
      date: date.to_owned(),
      time: "12:00".to_owned(),
      freq: freq.to_owned(),
      ..LogEntry::default()
    }
  }

  #[test]
  fn one_row_per_entity_preferring_confirmed() {
    let mut older = entry("JA1ABC", "2025-03-01", "14.025");
    older.lotw_rcvd = false;
    let mut newer = entry("JA1XYZ", "2025-06-01", "7.025");
    newer.lotw_rcvd = true;

    let rows = dxcc_record_sheet(&[older, newer]);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].callsign, "JA1XYZ");
    assert_eq!(rows[0].via, ApplyVia::Lotw);
    // 留下的那条是 7.025 MHz 的，波段自然是 40m。
    assert_eq!(rows[0].band, "40m");
  }

  #[test]
  fn unconfirmed_rows_fall_back_to_the_earliest() {
    let rows = dxcc_record_sheet(&[
      entry("JA1XYZ", "2025-06-01", "7.025"),
      entry("JA1ABC", "2025-03-01", "14.025"),
    ]);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].callsign, "JA1ABC");
    assert_eq!(rows[0].via, ApplyVia::None);
  }

  #[test]
  fn entries_without_an_entity_are_skipped() {
    let rows = dxcc_record_sheet(&[entry("", "2025-01-01", "14.025")]);
    assert!(rows.is_empty());
  }

  #[test]
  fn rows_are_sorted_by_english_entity_name() {
    // 排序键是英文名：en / es 界面显示的就是这一列，按中文码位排在那边是乱序。
    let rows = dxcc_record_sheet(&[
      entry("K1ABC", "2025-01-01", "14.025"),
      entry("JA1ABC", "2025-01-02", "14.025"),
    ]);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].entity_en, "Japan");
    assert_eq!(rows[1].entity_en, "United States");
    assert!(rows[0].entity_en <= rows[1].entity_en);
  }

  #[test]
  fn via_priority_is_lotw_then_card_then_eqsl_then_qrz() {
    let mut e = entry("JA1ABC", "2025-01-01", "14.025");
    assert_eq!(apply_via(&e), ApplyVia::None);
    e.qrz_rcvd = true;
    assert_eq!(apply_via(&e), ApplyVia::Qrz);
    e.eqsl_rcvd = true;
    assert_eq!(apply_via(&e), ApplyVia::Eqsl);
    e.qsl_rcvd = true;
    assert_eq!(apply_via(&e), ApplyVia::Card);
    e.lotw_rcvd = true;
    assert_eq!(apply_via(&e), ApplyVia::Lotw);
  }

  #[test]
  fn csv_fields_are_always_quoted_and_quotes_are_doubled() {
    assert_eq!(csv_field("BG1AAA"), "\"BG1AAA\"");
    assert_eq!(csv_field(""), "\"\"");
    assert_eq!(
      csv_field("Saint Pierre, Miquelon"),
      "\"Saint Pierre, Miquelon\""
    );
    assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
  }

  #[test]
  fn csv_fields_that_look_like_formulas_are_defused() {
    // Excel / Sheets 会把以这些字符开头的单元格当公式：呼号是用户输入，不能信。
    for lead in ["=", "+", "-", "@", "\t", "\r", "\n"] {
      let value = format!("{lead}HYPERLINK(\"http://x\")");
      let out = csv_field(&value);
      assert!(out.starts_with("\"'"), "{value:?} → {out:?} 没有前置单引号");
    }
    // 前置的是单引号而不是别的：表格软件把它当文本前缀，不显示出来。
    assert_eq!(csv_field("=1+1"), "\"'=1+1\"");
    // 前导空白不能当成「不是公式」：` =…` 在部分表格软件里照样执行。
    assert_eq!(csv_field(" =1+1"), "\"' =1+1\"");
    assert_eq!(csv_field("\t@SUM(1)"), "\"'\t@SUM(1)\"");
    // 不以这些字符开头就不动（中间出现不算）。
    assert_eq!(csv_field("a=b"), "\"a=b\"");
    assert_eq!(csv_field("14.025"), "\"14.025\"");
  }

  #[test]
  fn mixed_date_spellings_pick_the_real_earliest() {
    // `2026-9-5` 与 `2026-12-01`：逐字符比时第 6 位 `'9' > '1'`，9 月 5 日会被判成更晚，
    // 于是「日期最早」会挑中 12 月 1 日。
    let rows = dxcc_record_sheet(&[
      entry("JA1XYZ", "2026-12-01", "7.025"),
      entry("JA1ABC", "2026-9-5", "14.025"),
    ]);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].callsign, "JA1ABC", "9 月 5 日才是最早的一条");
  }

  #[test]
  fn eqsl_and_qrz_are_not_dxcc_confirmations() {
    // eQSL / QRZ 不算 DXCC 确认：应挑有纸质卡片的旧记录，而不是只被 QRZ 确认的新记录。
    let mut card = entry("JA1ABC", "2025-03-01", "14.025");
    card.qsl_rcvd = true;
    let mut qrz = entry("JA1XYZ", "2025-06-01", "7.025");
    qrz.qrz_rcvd = true;
    let rows = dxcc_record_sheet(&[qrz, card]);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].callsign, "JA1ABC");
    assert_eq!(rows[0].via, ApplyVia::Card);
  }

  #[test]
  fn csv_export_uses_english_entity_names_and_stable_labels() {
    let mut e = entry("JA1ABC", "2025-03-01", "14.025");
    e.lotw_rcvd = true;
    let csv = to_csv(&dxcc_record_sheet(&[e]));
    let mut lines = csv.lines();
    assert_eq!(lines.next(), Some("entity,callsign,date,band,mode,qsl_via"));
    let row = lines.next().expect("一行数据");
    assert!(
      row.starts_with("\"Japan\",\"JA1ABC\",\"2025-03-01\",\"20m\""),
      "{row}"
    );
    assert!(row.ends_with(",\"LoTW\""), "{row}");
    assert_eq!(lines.next(), None);
    // 中文实体名绝不能出现在导出里。
    assert!(csv.is_ascii(), "导出含非 ASCII 内容：{csv}");
  }

  #[test]
  fn an_empty_sheet_exports_only_the_header() {
    assert_eq!(to_csv(&[]), "entity,callsign,date,band,mode,qsl_via\n");
  }

  #[test]
  fn csv_via_labels_cover_every_variant() {
    for (via, label) in [
      (ApplyVia::None, "none"),
      (ApplyVia::Card, "card"),
      (ApplyVia::Lotw, "LoTW"),
      (ApplyVia::Eqsl, "eQSL"),
      (ApplyVia::Qrz, "QRZ"),
    ] {
      assert_eq!(via.csv_label(), label);
    }
  }
}
