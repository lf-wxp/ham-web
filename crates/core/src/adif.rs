//! ADIF（Amateur Data Interchange Format）解析与模式映射：用于导入 / 导出通联日志。

use std::collections::HashMap;

/// 一条 ADIF 通联记录（只提取常用字段）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdifRecord {
  pub call: String,
  /// YYYYMMDD。
  pub qso_date: String,
  /// HHMM 或 HHMMSS。
  pub time_on: String,
  /// HHMM 或 HHMMSS。
  pub time_off: String,
  pub freq: String,
  /// 波段（如 `20M`）。
  pub band: String,
  /// 本地显示用的模式（已合并 `MODE` / `SUBMODE`，如 `FT4`、`PSK31`）。
  pub mode: String,
  pub rst_sent: String,
  pub rst_rcvd: String,
  /// 发射功率（W）。
  pub tx_pwr: String,
  /// 对方网格定位。
  pub gridsquare: String,
  /// 对方操作员姓名。
  pub name: String,
  /// 对方 QTH。
  pub qth: String,
  pub comment: String,
  pub qsl_sent: bool,
  /// 任一途径（纸卡 / LoTW / eQSL）已确认。
  pub qsl_rcvd: bool,
  /// 卫星名。
  pub sat_name: String,
  /// 传播方式（如 `SAT`、`EME`、`ES`）。
  pub prop_mode: String,
  pub sota_ref: String,
  pub pota_ref: String,
  /// DXCC 实体编号。
  pub dxcc: String,
  /// 实体名（`COUNTRY`）。
  pub country: String,
  /// 大洲代码（`CONT`）。
  pub cont: String,
  /// CQ 分区（`CQZ`）。
  pub cqz: String,
  /// ITU 分区（`ITUZ`）。
  pub ituz: String,
  /// 竞赛名（`CONTEST_ID`）。
  pub contest_id: String,
  /// 发出 / 收到的竞赛交换（`STX_STRING` 优先，否则 `STX`；`SRX` 同理）。
  pub stx: String,
  pub srx: String,
}

/// 读取 `<` 之后、`>` 之前的标签，返回 `(标签内容, '>' 之后的位置)`。
fn read_tag(text: &str, lt: usize) -> Option<(&str, usize)> {
  let rel = text[lt..].find('>')?;
  Some((&text[lt + 1..lt + rel], lt + rel + 1))
}

/// 从 `start` 起取 `len` 长度的值：优先按字节（多数软件的写法），落在字符中间时按字符数。
fn read_value(text: &str, start: usize, len: usize) -> (&str, usize) {
  let stop = start.saturating_add(len).min(text.len());
  if let Some(v) = text.get(start..stop) {
    return (v, stop);
  }
  let end = text[start..]
    .char_indices()
    .nth(len)
    .map_or(text.len(), |(i, _)| start + i);
  (&text[start..end], end)
}

/// 把 ADIF 文本切分为记录（字段名统一大写），`<EOH>` 之前的头部字段被丢弃。
fn parse_records(text: &str) -> Vec<HashMap<String, String>> {
  let mut records = Vec::new();
  let mut current: HashMap<String, String> = HashMap::new();
  let mut i = 0;
  while let Some(rel) = text[i..].find('<') {
    let lt = i + rel;
    let Some((inner, after)) = read_tag(text, lt) else {
      break;
    };
    let mut parts = inner.splitn(3, ':');
    let tag = parts.next().unwrap_or_default().trim().to_ascii_uppercase();
    match (tag.as_str(), parts.next()) {
      ("EOH", None) => {
        current.clear();
        i = after;
      }
      ("EOR", None) => {
        records.push(std::mem::take(&mut current));
        i = after;
      }
      (_, Some(len)) => match len.trim().parse::<usize>() {
        Ok(len) => {
          let (value, end) = read_value(text, after, len);
          current.insert(tag, value.to_owned());
          i = end;
        }
        Err(_) => i = after,
      },
      _ => i = after,
    }
  }
  records
}

/// 本地模式 → ADIF `(MODE, SUBMODE)`（ADIF 3.1 枚举，FT4 / PSK31 等是子模式）。
#[must_use]
pub fn to_adif_mode(mode: &str) -> (String, Option<String>) {
  let m = mode.trim().to_ascii_uppercase();
  let parent = match m.as_str() {
    "FT4" | "FST4" | "FST4W" | "JS8" | "Q65" => Some("MFSK"),
    "PSK31" | "PSK63" | "PSK125" | "BPSK31" | "QPSK31" => Some("PSK"),
    "USB" | "LSB" => Some("SSB"),
    "DMR" | "C4FM" | "FREEDV" | "M17" => Some("DIGITALVOICE"),
    _ => None,
  };
  match parent {
    Some(p) => (p.to_owned(), Some(m)),
    None => (m, None),
  }
}

/// ADIF `(MODE, SUBMODE)` → 本地显示模式：有子模式时优先用子模式（`USB` / `LSB` 归为 `SSB`）。
#[must_use]
pub fn from_adif_mode(mode: &str, submode: &str) -> String {
  let mode = mode.trim().to_ascii_uppercase();
  let sub = submode.trim().to_ascii_uppercase();
  if sub.is_empty() || mode == "SSB" {
    return mode;
  }
  sub
}

/// 解析 ADIF 文本，返回记录列表（`CALL` 为空的记录被忽略）。
#[must_use]
pub fn parse_adif(text: &str) -> Vec<AdifRecord> {
  parse_records(text)
    .into_iter()
    .filter_map(|mut f| {
      let mut take = |k: &str| f.remove(k).unwrap_or_default().trim().to_owned();
      let call = take("CALL").to_ascii_uppercase();
      if call.is_empty() {
        return None;
      }
      let yes = |v: String| v.eq_ignore_ascii_case("Y");
      let mode = take("MODE");
      let submode = take("SUBMODE");
      Some(AdifRecord {
        call,
        qso_date: take("QSO_DATE"),
        time_on: take("TIME_ON"),
        time_off: take("TIME_OFF"),
        freq: take("FREQ"),
        band: take("BAND").to_ascii_uppercase(),
        mode: from_adif_mode(&mode, &submode),
        rst_sent: take("RST_SENT"),
        rst_rcvd: take("RST_RCVD"),
        tx_pwr: take("TX_PWR"),
        gridsquare: take("GRIDSQUARE").to_ascii_uppercase(),
        name: take("NAME"),
        qth: take("QTH"),
        comment: take("COMMENT"),
        qsl_sent: yes(take("QSL_SENT")),
        qsl_rcvd: yes(take("QSL_RCVD")) || yes(take("LOTW_QSL_RCVD")) || yes(take("EQSL_QSL_RCVD")),
        sat_name: take("SAT_NAME"),
        prop_mode: take("PROP_MODE").to_ascii_uppercase(),
        sota_ref: take("SOTA_REF"),
        pota_ref: take("POTA_REF"),
        dxcc: take("DXCC"),
        country: take("COUNTRY"),
        cont: take("CONT").to_ascii_uppercase(),
        cqz: take("CQZ"),
        ituz: take("ITUZ"),
        contest_id: take("CONTEST_ID").to_ascii_uppercase(),
        stx: Some(take("STX_STRING"))
          .filter(|s| !s.is_empty())
          .unwrap_or_else(|| take("STX")),
        srx: Some(take("SRX_STRING"))
          .filter(|s| !s.is_empty())
          .unwrap_or_else(|| take("SRX")),
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

  #[test]
  fn handles_lowercase_markers_typed_fields_and_qsl() {
    let adif = "<adif_ver:5>3.1.4<eoh>\n\
      <call:5:S>bg4xx <mode:4>MFSK <submode:3>FT4 <band:3>20m <lotw_qsl_rcvd:1>Y <qsl_sent:1>Y <tx_pwr:3>100 <eor>\n\
      <CALL:4>K1AA <MODE:3>SSB <SUBMODE:3>USB <PROP_MODE:3>SAT <SAT_NAME:5>SO-50 <eor>\n";
    let r = parse_adif(adif);
    assert_eq!(r.len(), 2);
    assert_eq!(
      (r[0].call.as_str(), r[0].mode.as_str(), r[0].band.as_str()),
      ("BG4XX", "FT4", "20M")
    );
    assert!(r[0].qsl_rcvd && r[0].qsl_sent);
    assert_eq!(r[0].tx_pwr, "100");
    assert_eq!(
      (
        r[1].mode.as_str(),
        r[1].prop_mode.as_str(),
        r[1].sat_name.as_str()
      ),
      ("SSB", "SAT", "SO-50")
    );
  }

  #[test]
  fn reads_utf8_values_by_byte_or_char_length() {
    let by_bytes = "<CALL:4>K1AA<NAME:6>张三<EOR>";
    assert_eq!(parse_adif(by_bytes)[0].name, "张三");
    let by_chars = "<CALL:4>K1AA<NAME:2>张三<QTH:2>BJ<EOR>";
    let r = &parse_adif(by_chars)[0];
    assert_eq!((r.name.as_str(), r.qth.as_str()), ("张三", "BJ"));
  }

  #[test]
  fn mode_mapping_roundtrip() {
    assert_eq!(
      to_adif_mode("FT4"),
      ("MFSK".to_owned(), Some("FT4".to_owned()))
    );
    assert_eq!(
      to_adif_mode("psk31"),
      ("PSK".to_owned(), Some("PSK31".to_owned()))
    );
    assert_eq!(to_adif_mode("FT8"), ("FT8".to_owned(), None));
    assert_eq!(from_adif_mode("MFSK", "FT4"), "FT4");
    assert_eq!(from_adif_mode("SSB", "USB"), "SSB");
    assert_eq!(from_adif_mode("CW", ""), "CW");
  }
}
