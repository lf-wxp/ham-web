//! NTS Radiogram / ICS-213 报文生成与格式。
//!
//! 应急通信（ARES / RACES）里最常用的两种报文格式：
//! - **NTS Radiogram**（ARRL 国家流量系统无线电报）：固定 preamble + 收件人地址 + 正文 + 签名，
//!   正文前后各有一个 `BT`（CW 的分隔 prosign，语音里念 `BREAK`，不计入检查），
//!   靠「检查」字段（check，正文字数）让收报站核对抄收是否漏字；
//! - **ICS-213 General Message**（FEMA 事件指挥系统通用消息表）：九宫格式的字段表，
//!   供现场各指挥单元之间传递书面消息。
//!
//! 这里只做「生成 + 格式校验」：NTS 部分按 ARRL《Public Service Communications Manual》
//! （FSD-3 优先级定义）的公开约定，ICS-213 按 FEMA 表单字段（见各结构体字段注释）。

use std::fmt::Write as _;

/// NTS 优先级。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Precedence {
  /// 常规（Routine，R）。
  #[default]
  Routine,
  /// 优先（Priority，P）。
  Priority,
  /// 福利（Welfare，W，灾难期间关于个人健康 / 安全的查询）。
  Welfare,
  /// 紧急（Emergency，E，涉及生命财产即时危险）。
  Emergency,
}

impl Precedence {
  /// preamble 里的写法：常规 / 优先 / 福利是一字母缩写，**紧急必须拼写全称**
  /// （ARRL FSD-3：Emergency is always spelled out in the preamble）。
  pub fn code(self) -> &'static str {
    match self {
      Precedence::Routine => "R",
      Precedence::Priority => "P",
      Precedence::Welfare => "W",
      Precedence::Emergency => "EMERGENCY",
    }
  }
}

/// NTS 处理指令（HX，可选）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hx {
  /// HXA：授权在指定英里数内用电话投递。
  Hxa,
  /// HXB：指定小时数内未送达则取消。
  Hxb,
  /// HXC：回报收到日期与时间。
  Hxc,
  /// HXD：回报经手转发站的身份。
  Hxd,
  /// HXE：送达站向收件人索取回复并回传。
  Hxe,
  /// HXF：保留到指定日期再投递。
  Hxf,
  /// HXG：邮件或电话投递。
  Hxg,
}

impl Hx {
  /// preamble 里的三字母指令。
  pub fn code(self) -> &'static str {
    match self {
      Hx::Hxa => "HXA",
      Hx::Hxb => "HXB",
      Hx::Hxc => "HXC",
      Hx::Hxd => "HXD",
      Hx::Hxe => "HXE",
      Hx::Hxf => "HXF",
      Hx::Hxg => "HXG",
    }
  }
}

/// 数 NTS「检查」（check）：正文的词 / 组数。
///
/// ARRL 规则：每个空格分隔的「组」算 1——纯字母词、纯数字组（如 `73`）、混合字母数字组
/// （如 `AB12C`、`304/BA`，组内的 `/` 不拆分）都各算 1。NTS 正文不用标点符号：句号写成字母
/// `X`（`X` 本身算 1 组、且不能是正文的最后一组），其它标点拼成单词（`QUERY` / `COMMA` /
/// `DASH`），它们同样各算 1 组；正文前后的 `BT`（语音 `BREAK`）不属于正文、不计入。
///
/// `split_whitespace` 正好符合「按空格分组」。对不合规的写法（单独的 `?`、词尾附着的
/// `meeting.`）这里从宽处理：单独出现的算 1 组，附着的不另算。
pub fn count_groups(text: &str) -> usize {
  text.split_whitespace().count()
}

/// NTS Radiogram 报文。
#[derive(Debug, Clone, Default)]
pub struct NtsRadiogram {
  /// 编号（由发信站分配）。
  pub number: String,
  pub precedence: Precedence,
  /// 可选处理指令。
  pub hx: Option<Hx>,
  /// 发信台站（呼号）。
  pub station: String,
  /// 发信地点（城市 + 州，如 `NEWINGTON CT`）。
  pub place: String,
  /// 可选发信时间（如 `1830`）。
  pub time: Option<String>,
  /// 发信日期（如 `JAN 15`）。
  pub date: String,
  /// 收件人（姓名 + 可选呼号）。
  pub addressee: String,
  /// 收件地址（街道 / 城市 / 邮编）。
  pub address: String,
  /// 可选电话。
  pub phone: Option<String>,
  /// 正文（多行；格式化时夹在两个 `BT` 之间原样输出）。
  pub body: String,
  /// 签名。
  pub signature: String,
}

impl NtsRadiogram {
  /// 检查（check）：正文词 / 组数。
  pub fn check(&self) -> usize {
    count_groups(&self.body)
  }

  /// 格式化完整报文。
  pub fn format(&self) -> String {
    let mut out = String::new();
    // Preamble 行：编号 优先级 [HX] 台站 检查 地点 [时间] 日期。
    out.push_str(&self.number);
    out.push(' ');
    out.push_str(self.precedence.code());
    if let Some(hx) = self.hx {
      out.push(' ');
      out.push_str(hx.code());
    }
    out.push(' ');
    out.push_str(&self.station);
    out.push(' ');
    out.push_str(&self.check().to_string());
    out.push(' ');
    out.push_str(&self.place);
    if let Some(time) = &self.time {
      out.push(' ');
      out.push_str(time);
    }
    out.push(' ');
    out.push_str(&self.date);
    out.push('\n');

    // 收件人 + 地址 + 电话。
    out.push_str(&self.addressee);
    out.push('\n');
    out.push_str(&self.address);
    out.push('\n');
    if let Some(phone) = &self.phone {
      out.push_str(phone);
      out.push('\n');
    }

    // 正文前后各一个 BT：把正文与地址、签名隔开，收报站靠它知道正文从哪到哪。
    // （句号写成字母 X 是正文里的约定，不是行前缀。）
    out.push_str("BT\n");
    for line in self.body.lines() {
      out.push_str(line);
      out.push('\n');
    }
    out.push_str("BT\n");

    // 签名。
    out.push_str(&self.signature);
    out
  }
}

/// ICS-213 通用消息表。
#[derive(Debug, Clone, Default)]
pub struct Ics213 {
  /// 事件名称（可选）。
  pub incident_name: String,
  /// 收件人（姓名 + 职位）。
  pub to: String,
  /// 发件人（姓名 + 职位）。
  pub from: String,
  /// 主题。
  pub subject: String,
  /// 日期。
  pub date: String,
  /// 时间。
  pub time: String,
  /// 消息正文。
  pub message: String,
  /// 批准人（姓名 + 职位）。
  pub approved_by: String,
  /// 回复。
  pub reply: String,
  /// 回复人（姓名 + 职位）。
  pub replied_by: String,
}

impl Ics213 {
  /// 格式化为纯文本（带字段标签，便于打印 / 复制）。
  pub fn format(&self) -> String {
    let mut out = String::new();
    out.push_str("ICS-213 GENERAL MESSAGE\n");
    if !self.incident_name.is_empty() {
      let _ = writeln!(out, "INCIDENT NAME: {}", self.incident_name);
    }
    let _ = writeln!(out, "TO: {}", self.to);
    let _ = writeln!(out, "FROM: {}", self.from);
    let _ = writeln!(out, "SUBJECT: {}", self.subject);
    let _ = writeln!(out, "DATE: {}", self.date);
    let _ = writeln!(out, "TIME: {}", self.time);
    let _ = writeln!(out, "MESSAGE:\n{}", self.message);
    let _ = writeln!(out, "APPROVED BY: {}", self.approved_by);
    let _ = writeln!(out, "REPLY:\n{}", self.reply);
    let _ = writeln!(out, "REPLIED BY: {}", self.replied_by);
    out
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn sample() -> NtsRadiogram {
    NtsRadiogram {
      number: "12".into(),
      precedence: Precedence::Routine,
      hx: None,
      station: "W1AW".into(),
      place: "NEWINGTON CT".into(),
      time: Some("1830".into()),
      date: "JAN 15".into(),
      addressee: "JOHN SMITH".into(),
      address: "123 MAIN ST ANYTOWN CT 06444".into(),
      phone: Some("555-555-1212".into()),
      body: "HAVE FUN AT THE MEETING\n73".into(),
      signature: "ARRL HEADQUARTERS".into(),
    }
  }

  #[test]
  fn check_counts_letters_numbers_and_punctuation_groups() {
    // 纯字母词 + 数字组 + 混合组 + 单独标点。
    assert_eq!(count_groups("HAVE FUN AT THE MEETING"), 5);
    assert_eq!(count_groups("73"), 1);
    assert_eq!(count_groups("AB12C 123 XYZ"), 3);
    assert_eq!(count_groups("SEE YOU ?"), 3);
    // `X` 是句号，本身算 1 组；组内的 `/` 不拆分。
    assert_eq!(count_groups("ARRIVING SATURDAY X PLEASE MEET ME"), 6);
    assert_eq!(count_groups("FREQ 146R670 CALL 304/BA"), 4);
    // 附着标点不另算。
    assert_eq!(count_groups("meeting. soon."), 2);
  }

  #[test]
  fn nts_check_counts_the_body_only() {
    let msg = sample();
    // 正文两行："HAVE FUN AT THE MEETING"(5) + "73"(1)。
    assert_eq!(msg.check(), 6);
  }

  #[test]
  fn nts_format_produces_the_expected_layout() {
    let msg = sample();
    let text = msg.format();
    let lines: Vec<&str> = text.lines().collect();
    // Preamble 行：编号 优先级 台站 检查 地点 时间 日期。
    assert_eq!(lines[0], "12 R W1AW 6 NEWINGTON CT 1830 JAN 15");
    assert_eq!(lines[1], "JOHN SMITH");
    assert_eq!(lines[2], "123 MAIN ST ANYTOWN CT 06444");
    assert_eq!(lines[3], "555-555-1212");
    // 正文夹在两个 BT 之间，没有行前缀；签名在第二个 BT 之后，没有 `=` 前缀。
    assert_eq!(lines[4], "BT");
    assert_eq!(lines[5], "HAVE FUN AT THE MEETING");
    assert_eq!(lines[6], "73");
    assert_eq!(lines[7], "BT");
    assert_eq!(lines[8], "ARRL HEADQUARTERS");
    assert_eq!(lines.len(), 9);
  }

  /// 电话是可选的：缺省时不留空行，BT 紧跟地址。
  #[test]
  fn nts_format_without_phone_has_no_blank_line() {
    let mut msg = sample();
    msg.phone = None;
    let text = msg.format();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[2], "123 MAIN ST ANYTOWN CT 06444");
    assert_eq!(lines[3], "BT");
    assert!(lines.iter().all(|l| !l.is_empty()));
  }

  /// 检查只数正文：BT 与签名都不计入。
  #[test]
  fn check_ignores_bt_and_signature() {
    let msg = sample();
    let text = msg.format();
    // 报头里的检查是 6（正文两行 5 + 1），而整份报文的词数远多于此。
    assert!(text.lines().next().unwrap().contains(" 6 "));
    assert!(text.split_whitespace().count() > 6);
  }

  #[test]
  fn nts_format_omits_optional_hx_and_time() {
    let mut msg = sample();
    msg.hx = None;
    msg.time = None;
    let text = msg.format();
    assert!(text.starts_with("12 R W1AW 6 NEWINGTON CT JAN 15\n"));
  }

  #[test]
  fn nts_format_includes_hx_when_present() {
    let mut msg = sample();
    msg.hx = Some(Hx::Hxa);
    msg.time = None;
    let text = msg.format();
    assert!(text.starts_with("12 R HXA W1AW 6 NEWINGTON CT JAN 15\n"));
  }

  #[test]
  fn precedence_codes_are_letters_except_emergency_which_is_spelled_out() {
    assert_eq!(Precedence::Routine.code(), "R");
    assert_eq!(Precedence::Priority.code(), "P");
    assert_eq!(Precedence::Welfare.code(), "W");
    // ARRL FSD-3：紧急在报头里永远拼写全称。
    assert_eq!(Precedence::Emergency.code(), "EMERGENCY");
  }

  #[test]
  fn emergency_precedence_is_spelled_out_in_the_preamble() {
    let mut msg = sample();
    msg.precedence = Precedence::Emergency;
    msg.time = None;
    assert!(
      msg
        .format()
        .starts_with("12 EMERGENCY W1AW 6 NEWINGTON CT JAN 15\n")
    );
  }

  #[test]
  fn hx_codes_are_hxa_through_hxg() {
    assert_eq!(Hx::Hxa.code(), "HXA");
    assert_eq!(Hx::Hxg.code(), "HXG");
  }

  #[test]
  fn ics213_format_has_the_standard_field_labels() {
    let msg = Ics213 {
      incident_name: "FLOOD".into(),
      to: "NET CONTROL".into(),
      from: "FIELD STATION 2".into(),
      subject: "WATER LEVEL".into(),
      date: "2026-10-17".into(),
      time: "18:30".into(),
      message: "Water rising fast. Request assistance.".into(),
      approved_by: "INCIDENT COMMANDER".into(),
      reply: "Team dispatched.".into(),
      replied_by: "NET CONTROL".into(),
    };
    let text = msg.format();
    assert!(text.contains("INCIDENT NAME: FLOOD\n"));
    assert!(text.contains("TO: NET CONTROL\n"));
    assert!(text.contains("SUBJECT: WATER LEVEL\n"));
    assert!(text.contains("APPROVED BY: INCIDENT COMMANDER\n"));
    assert!(text.contains("REPLIED BY: NET CONTROL\n"));
  }
}
