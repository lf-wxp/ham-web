//! ADIF（Amateur Data Interchange Format）解析：用于导入外部通联日志。

use std::collections::HashMap;

/// 一条 ADIF 通联记录（只提取常用字段）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdifRecord {
  pub call: String,
  /// YYYYMMDD。
  pub qso_date: String,
  /// HHMM。
  pub time_on: String,
  pub freq: String,
  pub mode: String,
  pub rst_sent: String,
  pub rst_rcvd: String,
  /// 对方网格定位。
  pub gridsquare: String,
  /// 对方操作员姓名。
  pub name: String,
  /// 对方 QTH。
  pub qth: String,
  pub comment: String,
}

/// 解析 `<TAG:LEN>VALUE` 字段序列。
fn parse_fields(record: &str) -> HashMap<String, String> {
  let mut map = HashMap::new();
  let bytes = record.as_bytes();
  let mut i = 0;
  while i < bytes.len() {
    if bytes[i] != b'<' {
      i += 1;
      continue;
    }
    let Some(rel_end) = record[i..].find('>') else {
      i += 1;
      continue;
    };
    let end = i + rel_end;
    let inner = &record[i + 1..end];
    let Some((tag, len_str)) = inner.split_once(':') else {
      i += 1;
      continue;
    };
    let tag = tag.trim().to_uppercase();
    let Ok(len) = len_str.trim().parse::<usize>() else {
      i += 1;
      continue;
    };
    let start = end + 1;
    let stop = (start + len).min(record.len());
    if let Some(value) = record.get(start..stop) {
      map.insert(tag, value.to_owned());
    }
    i = stop;
  }
  map
}

/// 解析 ADIF 文本，返回记录列表（`<EOH>` 后按 `<EOR>` 分段）。
#[must_use]
pub fn parse_adif(text: &str) -> Vec<AdifRecord> {
  let body = text
    .find("<EOH>")
    .map_or(text, |i| &text[i + "<EOH>".len()..]);
  body
    .split("<EOR>")
    .filter_map(|record| {
      let fields = parse_fields(record);
      let call = fields.get("CALL")?.clone();
      if call.trim().is_empty() {
        return None;
      }
      Some(AdifRecord {
        call,
        qso_date: fields.get("QSO_DATE").cloned().unwrap_or_default(),
        time_on: fields.get("TIME_ON").cloned().unwrap_or_default(),
        freq: fields.get("FREQ").cloned().unwrap_or_default(),
        mode: fields.get("MODE").cloned().unwrap_or_default(),
        rst_sent: fields.get("RST_SENT").cloned().unwrap_or_default(),
        rst_rcvd: fields.get("RST_RCVD").cloned().unwrap_or_default(),
        gridsquare: fields.get("GRIDSQUARE").cloned().unwrap_or_default(),
        name: fields.get("NAME").cloned().unwrap_or_default(),
        qth: fields.get("QTH").cloned().unwrap_or_default(),
        comment: fields.get("COMMENT").cloned().unwrap_or_default(),
      })
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_records() {
    let adif = "header\n<ADIF_VER:5>3.1.4\n<EOH>\n\
      <QSO_DATE:8>20260930<TIME_ON:4>1234<CALL:5>BG4XX<FREQ:7>14.0740<MODE:3>FT8<RST_SENT:3>599<RST_RCVD:3>599<GRIDSQUARE:6>OM89EW<NAME:3>Bob<QTH:7>Beijing<COMMENT:4>test<EOR>\n\
      <QSO_DATE:8>20260930<TIME_ON:4>1300<CALL:4>JA1X<FREQ:6>7.0740<MODE:2>CW<RST_SENT:3>599<EOR>\n";
    let records = parse_adif(adif);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].call, "BG4XX");
    assert_eq!(records[0].freq, "14.0740");
    assert_eq!(records[0].gridsquare, "OM89EW");
    assert_eq!(records[0].name, "Bob");
    assert_eq!(records[0].qth, "Beijing");
    assert_eq!(records[0].comment, "test");
    assert_eq!(records[1].call, "JA1X");
    assert!(records[1].comment.is_empty());
  }

  #[test]
  fn ignores_empty_call() {
    let adif = "<EOH>\n<CALL:0><EOR>\n<CALL:4>K1AA<EOR>\n";
    let records = parse_adif(adif);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].call, "K1AA");
  }
}
