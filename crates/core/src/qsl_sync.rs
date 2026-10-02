//! QSL 确认同步：把 LoTW / eQSL 等平台下载的「确认报告」（ADIF）应用到本地日志，
//! 自动把匹配到的 QSO 标记为已确认，免去逐条手工勾选。
//!
//! 报告里的每条记录按 [`LogEntry::qso_key`]（呼号 + 日期 + 时间 + 波段 + 模式）匹配本地
//! 日志，命中后对三路确认状态（纸卡 / LoTW / eQSL）取逻辑或，顺带同步「已寄出」标记。

use crate::adif::parse_adif;
use crate::logbook::{LogEntry, from_adif};

/// 一次同步的结果统计。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QslSyncResult {
  /// 报告里的记录总数。
  pub total: usize,
  /// 匹配到本地日志并更新的条数。
  pub matched: usize,
  /// 报告里有确认信息、但本地未匹配到的条数。
  pub unmatched: usize,
}

/// 把 ADIF 确认报告应用到日志，就地更新匹配记录的 QSL 确认状态，返回统计。
#[must_use]
pub fn apply_qsl_report(entries: &mut [LogEntry], report_adif: &str) -> QslSyncResult {
  let records = parse_adif(report_adif);
  let mut result = QslSyncResult {
    total: records.len(),
    ..Default::default()
  };
  for rec in records {
    let report_entry = from_adif(rec);
    let key = report_entry.qso_key();
    match entries.iter_mut().find(|e| e.qso_key() == key) {
      Some(target) => {
        target.qsl_sent |= report_entry.qsl_sent;
        target.qsl_rcvd |= report_entry.qsl_rcvd;
        target.lotw_sent |= report_entry.lotw_sent;
        target.lotw_rcvd |= report_entry.lotw_rcvd;
        target.eqsl_sent |= report_entry.eqsl_sent;
        target.eqsl_rcvd |= report_entry.eqsl_rcvd;
        result.matched += 1;
      }
      None => result.unmatched += 1,
    }
  }
  result
}

#[cfg(test)]
mod tests {
  use super::*;

  fn qso(call: &str, freq: &str, mode: &str, time: &str) -> LogEntry {
    LogEntry {
      callsign: call.into(),
      date: "2026-09-30".into(),
      time: time.into(),
      freq: freq.into(),
      mode: mode.into(),
      ..Default::default()
    }
  }

  #[test]
  fn updates_matching_qsos_and_counts_unmatched() {
    let mut log = vec![
      qso("JA1X", "14.074", "FT8", "12:34"),
      qso("W1AW", "7.010", "CW", "13:00"),
    ];
    let report = "<EOH>\n\
      <CALL:4>JA1X<QSO_DATE:8>20260930<TIME_ON:4>1234<BAND:3>20M<MODE:3>FT8<LOTW_QSL_RCVD:1>Y<EOR>\n\
      <CALL:4>DL1A<QSO_DATE:8>20260930<TIME_ON:4>1400<BAND:3>40M<MODE:2>CW<LOTW_QSL_RCVD:1>Y<EOR>\n";
    let r = apply_qsl_report(&mut log, report);
    assert_eq!((r.total, r.matched, r.unmatched), (2, 1, 1));
    assert!(log[0].lotw_rcvd);
    assert!(!log[1].lotw_rcvd);
  }

  #[test]
  fn merges_multiple_confirmations_without_losing_flags() {
    let mut log = vec![qso("JA1X", "14.074", "FT8", "12:34")];
    log[0].qsl_rcvd = true;
    let report = "<EOH>\n\
      <CALL:4>JA1X<QSO_DATE:8>20260930<TIME_ON:4>1234<BAND:3>20M<MODE:3>FT8<LOTW_QSL_RCVD:1>Y<EOR>\n";
    let _ = apply_qsl_report(&mut log, report);
    assert!(log[0].qsl_rcvd, "纸卡确认应保留");
    assert!(log[0].lotw_rcvd, "LoTW 确认应累加");
  }
}
