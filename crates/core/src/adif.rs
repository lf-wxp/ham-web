//! ADIF（Amateur Data Interchange Format）解析与模式映射：用于导入 / 导出通联日志。

use std::collections::HashMap;

use crate::qsl_status::QslVia;

/// QRZ Logbook 确认位用的**应用自定义字段**（ADIF 允许 `APP_` 开头的自定义字段）。
///
/// 为什么要自己造一个：QRZ 的站内确认在 ADIF 里没有独立字段（与纸质卡共用 `QSL_RCVD`），
/// 照它落库会把电子确认标成「纸质已收」。带上自己的 PROGRAMID 以免与别的程序撞名 ——
/// 其它日志软件会忽略它，本工具导出再导入时原样读回。
pub const APP_QRZ_RCVD: &str = "APP_HAMEXAMWEB_QRZ_RCVD";

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
  /// 本台呼号 `STATION_CALLSIGN`（多台站日志靠它 + 本台网格判断这条属于哪个台站）。
  pub station_callsign: String,
  /// 本台网格 `MY_GRIDSQUARE`。
  pub my_gridsquare: String,
  /// 本台设备 `MY_RIG`。
  pub my_rig: String,
  /// 本台天线 `MY_ANTENNA`。
  pub my_antenna: String,
  /// 操作员 `OPERATOR`。
  pub operator: String,
  pub qsl_sent: bool,
  /// 纸卡寄出方式 `QSL_SENT_VIA`（`B` 卡片局 / `D` 直寄；其余代码按「未知」处理）。
  pub qsl_sent_via: QslVia,
  /// 纸卡已确认收到。
  pub qsl_rcvd: bool,
  /// LoTW 已上传 / 已确认。
  pub lotw_sent: bool,
  pub lotw_rcvd: bool,
  /// eQSL 已寄出 / 已确认。
  pub eqsl_sent: bool,
  pub eqsl_rcvd: bool,
  /// 本行是否**出现了** [`APP_QRZ_RCVD`] 字段。
  ///
  /// 与字段值分开：判断「这份报告能不能代表 QRZ 渠道」看的是字段在不在 ——
  /// 一份不带该字段的报告说不了 QRZ 的任何事（见 `qsl_sync` 的模块文档）。
  pub has_qrz_flag: bool,
  /// QRZ Logbook 是否已确认（[`APP_QRZ_RCVD`] 的值）。
  pub qrz_rcvd: bool,
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
  /// 美国州（`STATE`）。
  pub state: String,
  /// IOTA 岛屿编号（`IOTA`）。
  pub iota: String,
}

/// 读取 `<` 之后、`>` 之前的标签，返回 `(标签内容, '>' 之后的位置)`。
fn read_tag(text: &str, lt: usize) -> Option<(&str, usize)> {
  let rel = text[lt..].find('>')?;
  Some((&text[lt + 1..lt + rel], lt + rel + 1))
}

/// 从 `start` 起取 `len` 长度的值。
///
/// ADIF 规范说 `len` 是**字符数**，但相当多导出软件按**字节数**写。两种口径只在值里
/// 含多字节字符时才有分歧，于是先试字节口径：切出来的子串必须正好落在字符边界上，
/// **并且紧随其后（跳过空白）就是下一个标签的 `<`** 或文本结尾 —— 那才说明导出方按字节写。
///
/// 只判「落在字符边界上」是不够的：`<NAME:2>éé`（2 字符 / 4 字节，按字符写）切 2 字节
/// 恰好是第一个 `é`，边界合法却会丢掉一个字符；此时后面跟的是第二个 `é` 而不是 `<`，
/// 于是回退字符口径、拿回 `éé`。
fn read_value(text: &str, start: usize, len: usize) -> (&str, usize) {
  let byte_stop = start.saturating_add(len);
  if byte_stop <= text.len() && text.is_char_boundary(byte_stop) {
    let rest = text[byte_stop..].trim_start();
    if rest.is_empty() || rest.starts_with('<') {
      return (&text[start..byte_stop], byte_stop);
    }
  }
  let end = text[start..]
    .char_indices()
    .nth(len)
    .map_or(text.len(), |(i, _)| start + i);
  (&text[start..end], end)
}

/// 把 ADIF 文本切分为记录（字段名统一大写），`<EOH>` 之前的头部字段被丢弃。
///
/// 返回值第二项表示文本是否在中途被**截断**：出现「有 `<` 却找不到 `>`」时提前结束，
/// 此时拿到的记录数可能少于报告实际条数。静默当成「报告就这么长」会把不完整同步
/// 伪装成「没有差异」，所以要把这件事传出去。
fn parse_records(text: &str) -> (Vec<HashMap<String, String>>, bool) {
  let mut records = Vec::new();
  let mut current: HashMap<String, String> = HashMap::new();
  let mut truncated = false;
  let mut i = 0;
  while let Some(rel) = text[i..].find('<') {
    let lt = i + rel;
    let Some((inner, after)) = read_tag(text, lt) else {
      truncated = true;
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
  // 文件末尾缺 `<EOR>`（手抄清单漏了收尾）时，最后一条记录仍**完整地**留在 `current` 里：
  // 丢掉它等于把「报告里没有这条」当成结论，同步时会把它误判成待确认差异。
  // 但「读到半截标签而 break」不算 —— 那种残片字段不全，`from_adif` 出来的记录既没有
  // 日期也没有时间，保留只会凭空制造一条假的「本地缺失」；这种情况交给 `truncated` 报警。
  if !current.is_empty() && !truncated {
    records.push(current);
    truncated = true;
  }
  (records, truncated)
}

/// 本地模式 → ADIF `(MODE, SUBMODE)`（ADIF 3.1 枚举，FT4 / PSK31 等是子模式）。
///
/// [`crate::logbook::MODES`] 里的 `DATA` / `OTHER` 是**本站的兜底分类**，ADIF 3.1.4 的
/// 模式枚举里没有对应值。这里刻意保持原样导出，不替用户折成一个真实模式：
///
/// * 折成 `MFSK` / `SSB` 之类等于**凭空造一个模式**，LoTW 会照着它算模式奖状 —— 拿用户
///   的奖状进度换「文件能过」不值当；
/// * 整个省略 `MODE` 也不是「更安全」的默认：没有模式的记录同样不是一条可用记录，
///   而本站自己再导入时会丢失模式（`qso_key` 含模式，往返还会多出一条重复通联）。
///
/// 保持原样时坏消息只落在这一条记录上，且 tQSL 会明确指出是哪个模式不认识，用户改成
/// 具体模式（`FT8`、`RTTY`…）即可 —— 比悄悄替他改掉一个模式要好。
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
  // `MODE = USB / LSB` 严格说不是合法 ADIF（它们是 `SSB` 的子模式），但确实有导出工具
  // 这么写。这里一并折成 `SSB`：`qso_key` 含模式，不折的话同一次通联在本地（录入时选
  // `SSB`）与报告（`USB`）之间会被当成两条不同的记录，同步时误报「本地缺失」并补出一条
  // 重复通联。本地可选模式里本来就没有 `USB` / `LSB`（见 `logbook::MODES`），不会反向歧义。
  if matches!(mode.as_str(), "USB" | "LSB") {
    return "SSB".to_owned();
  }
  if sub.is_empty() || mode == "SSB" {
    return mode;
  }
  sub
}

/// 把一条记录的字段表转成 [`AdifRecord`]；`CALL` 为空时返回 `None`。
fn record_from_fields(mut f: HashMap<String, String>) -> Option<AdifRecord> {
  // 这两位的「字段在不在」与「字段值」都要看：报告带不带这个字段，决定它能不能代表
  // QRZ 渠道。`take` 闭包会一直占着 `f` 的可变借用，所以先取出来。
  let has_qrz_flag = f.contains_key(APP_QRZ_RCVD);
  let qrz_raw = f.remove(APP_QRZ_RCVD).unwrap_or_default();
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
    has_qrz_flag,
    qrz_rcvd: qrz_raw.trim().eq_ignore_ascii_case("Y"),
    station_callsign: take("STATION_CALLSIGN").to_ascii_uppercase(),
    my_gridsquare: take("MY_GRIDSQUARE").to_ascii_uppercase(),
    my_rig: take("MY_RIG"),
    my_antenna: take("MY_ANTENNA"),
    operator: take("OPERATOR"),
    qsl_sent: yes(take("QSL_SENT")),
    qsl_sent_via: QslVia::from_adif_code(&take("QSL_SENT_VIA")),
    qsl_rcvd: yes(take("QSL_RCVD")),
    lotw_sent: yes(take("LOTW_QSL_SENT")),
    lotw_rcvd: yes(take("LOTW_QSL_RCVD")),
    eqsl_sent: yes(take("EQSL_QSL_SENT")),
    eqsl_rcvd: yes(take("EQSL_QSL_RCVD")),
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
    state: take("STATE").to_ascii_uppercase(),
    iota: take("IOTA").to_ascii_uppercase(),
  })
}

/// 解析 ADIF 文本，返回记录列表与「文本是否在中途被截断」。
///
/// `truncated` 为真表示出现「有 `<` 却找不到 `>`」而提前结束：条数可能少于报告实际条数，
/// 调用方应当提示用户，而不是把不完整的结果当成完整报告。
#[must_use]
pub fn parse_adif_report(text: &str) -> (Vec<AdifRecord>, bool) {
  let (fields, truncated) = parse_records(text);
  let records = fields.into_iter().filter_map(record_from_fields).collect();
  (records, truncated)
}

/// 解析 ADIF 文本，返回记录列表（`CALL` 为空的记录被忽略）。
#[must_use]
pub fn parse_adif(text: &str) -> Vec<AdifRecord> {
  parse_adif_report(text).0
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
    assert!(r[0].lotw_rcvd && r[0].qsl_sent);
    assert!(!r[0].qsl_rcvd, "LoTW 确认应独立于纸卡 QSL");
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
  fn multibyte_value_written_by_char_count_is_not_truncated() {
    // 2 字符 / 4 字节：按字节切 2 恰好落在字符边界（第一个 `é` 之后），只看边界会丢字符。
    let by_chars = "<CALL:4>K1AA<NAME:2>éé<QTH:2>BJ<EOR>";
    let r = &parse_adif(by_chars)[0];
    assert_eq!((r.name.as_str(), r.qth.as_str()), ("éé", "BJ"));
  }

  #[test]
  fn reports_truncated_text_instead_of_pretending_it_ended() {
    // 末尾的 `<COMM` 没有 `>`：报告被截断，第二行连 `<EOR>` 都没读全。
    let (records, truncated) = parse_adif_report("<CALL:4>K1AA<EOR>\n<CALL:4>JA1X<COMM");
    assert_eq!(records.len(), 1);
    assert!(truncated, "缺少 `>` 的报告必须标记为被截断");

    let (records, truncated) = parse_adif_report("<CALL:4>K1AA<EOR>");
    assert_eq!(records.len(), 1);
    assert!(!truncated, "完整报告不应标记为截断");
  }

  #[test]
  fn a_record_whose_eor_is_missing_is_kept_and_flagged() {
    // 末尾缺 `<EOR>` 但字段完整：最后一条要留下（丢掉它会让同步把「报告里没有」
    // 当成结论），同时报「被截断」让调用方自己决定信不信这份报告。
    let (records, truncated) =
      parse_adif_report("<CALL:4>K1AA<EOR>\n<CALL:4>JA1X<QSO_DATE:8>20260101");
    assert_eq!(records.len(), 2);
    assert!(truncated, "缺 `<EOR>` 的报告同样不完整");
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
    // `DATA` / `OTHER` 是本站的兜底分类，ADIF 枚举里没有对应值。刻意**原样**导出，
    // 不折成某个真实模式 —— 折了等于凭空造一个模式，LoTW 会照着它算模式奖状
    // （理由见 `to_adif_mode` 的文档）。
    assert_eq!(to_adif_mode("DATA"), ("DATA".to_owned(), None));
    assert_eq!(to_adif_mode("other"), ("OTHER".to_owned(), None));
    assert_eq!(from_adif_mode("MFSK", "FT4"), "FT4");
    assert_eq!(from_adif_mode("SSB", "USB"), "SSB");
    assert_eq!(from_adif_mode("CW", ""), "CW");
    // `MODE = USB / LSB`（不合法但真实存在）折成 `SSB`，否则同一次通联会被同步逻辑
    // 当成「本地缺失」并补出一条重复记录（`qso_key` 含模式）。
    assert_eq!(from_adif_mode("USB", ""), "SSB");
    assert_eq!(from_adif_mode("lsb", "LSB"), "SSB");
  }
}
