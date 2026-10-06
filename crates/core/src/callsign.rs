//! 呼号结构解析：前缀（国家/地区）、中国台站类别、分区号、后缀与斜杠后缀。
//!
//! 复用 [`crate::dxcc`] 的实体识别与 [`crate::reference::CALL_AREAS`] 的中国分区表。

use crate::dxcc::lookup_with_prefix;
use crate::reference::CALL_AREAS;

/// 台站类别（中国呼号第二位字母）。
pub const STATION_TYPES: &[(&str, &str)] = &[
  ("BA–BH", "个人业余电台（呼号序列 A–H，BG 最常见）"),
  ("BI", "个人业余电台（呼号序列 I）"),
  ("BJ", "信标台 / 空间电台"),
  ("BR", "中继台"),
  ("BT", "特设电台"),
  ("BY", "集体电台 / 俱乐部电台"),
];

/// 斜杠后缀含义。
pub const SLASH_SUFFIXES: &[(&str, &str)] = &[
  ("/P", "便携操作（Portable，在临时地点设台）"),
  ("/M", "移动操作（Mobile，车载等移动中）"),
  ("/MM", "海上移动（Maritime Mobile）"),
  ("/AM", "航空移动（Aeronautical Mobile）"),
  ("/QRP", "低功率（CW ≤5W、SSB ≤10W）"),
  ("/QRO", "高功率"),
  ("/0–/9", "在其他分区操作（数字表示分区号）"),
];

/// 解析结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallsignParts {
  /// 匹配到的前缀，如 `B`、`BV`、`JA`。
  pub prefix: Option<&'static str>,
  /// 前缀对应的国家/地区。
  pub entity: Option<&'static str>,
  /// 台站类别（中国呼号第二位）。
  pub station_type: Option<&'static str>,
  /// 分区号（中国呼号第三位，0–9）。
  pub area: Option<&'static str>,
  /// 分区对应的地区。
  pub area_regions: Option<&'static str>,
  /// 后缀（前缀与分区之后的字符）。
  pub suffix: String,
  /// 斜杠后缀含义。
  pub slash: Option<&'static str>,
}

/// DXCC 中国实体编号。
const CHINA_DXCC: u16 = 318;

/// 台站类别（按中国呼号第二位字母）。
fn station_type_of(c: char) -> Option<&'static str> {
  match c {
    'A'..='H' => Some("个人业余电台（BA–BH）"),
    'I' => Some("个人业余电台（BI）"),
    'J' => Some("信标台 / 空间电台（BJ）"),
    'R' => Some("中继台（BR）"),
    'T' => Some("特设电台（BT）"),
    'Y' => Some("集体电台 / 俱乐部电台（BY）"),
    _ => None,
  }
}

/// 斜杠后缀含义。
fn slash_of(slash: &str) -> Option<&'static str> {
  let s = slash.trim().to_ascii_uppercase();
  match s.as_str() {
    "P" => Some("便携操作（Portable，在临时地点设台）"),
    "M" => Some("移动操作（Mobile，车载等移动中）"),
    "MM" => Some("海上移动（Maritime Mobile）"),
    "AM" => Some("航空移动（Aeronautical Mobile）"),
    "QRP" => Some("低功率（CW ≤5W、SSB ≤10W）"),
    "QRO" => Some("高功率"),
    _ if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) => {
      Some("在其他分区操作（数字表示分区号）")
    }
    _ => None,
  }
}

/// 解析一个呼号，返回各部分含义。
#[must_use]
pub fn parse_callsign(input: &str) -> CallsignParts {
  let cs = input.trim().to_ascii_uppercase();

  // 分离斜杠后缀。
  let (body, slash) = match cs.split_once('/') {
    Some((b, s)) => (b.to_owned(), slash_of(s)),
    None => (cs.clone(), None),
  };

  let matched = lookup_with_prefix(&cs);
  let prefix = matched.map(|(p, _)| p).filter(|p| body.starts_with(p));
  let entity = matched.map(|(_, e)| e.name);
  let is_china = matched.is_some_and(|(_, e)| e.dxcc == CHINA_DXCC) && body.starts_with('B');

  let chars: Vec<char> = body.chars().collect();
  let mut station_type = None;
  let mut area = None;
  let mut area_regions = None;
  let mut suffix = String::new();

  if let Some(p) = prefix {
    // 中国呼号：B 开头，第二位字母（类别）+ 第三位数字（分区）。
    if is_china && chars.len() >= 3 {
      if let Some(&c) = chars.get(1)
        && c.is_ascii_alphabetic()
      {
        station_type = station_type_of(c);
      }
      if let Some(&d) = chars.get(2)
        && d.is_ascii_digit()
        && let Some(a) = CALL_AREAS.iter().find(|a| a.digit == d.to_string())
      {
        area = Some(a.digit);
        area_regions = Some(a.regions);
      }
      suffix = chars.iter().skip(3).collect();
    } else if body.len() > p.len() {
      suffix = body[p.len()..].to_owned();
    }
  } else if !body.is_empty() {
    suffix = body;
  }

  CallsignParts {
    prefix,
    entity,
    station_type,
    area,
    area_regions,
    suffix,
    slash,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_china_callsign() {
    let p = parse_callsign("BG4XYZ");
    assert_eq!(p.prefix, Some("BG"));
    assert_eq!(p.entity, Some("中国"));
    assert_eq!(p.station_type, Some("个人业余电台（BA–BH）"));
    assert_eq!(p.area, Some("4"));
    assert_eq!(p.area_regions, Some("上海 · 山东 · 江苏"));
    assert_eq!(p.suffix, "XYZ");
  }

  #[test]
  fn parses_other_region() {
    let p = parse_callsign("JA1ABC");
    assert_eq!(p.entity, Some("日本"));
    assert_eq!(p.station_type, None);
    assert_eq!(p.suffix, "1ABC");
  }

  #[test]
  fn parses_slash_suffix() {
    assert_eq!(
      parse_callsign("BG4XYZ/P").slash,
      Some("便携操作（Portable，在临时地点设台）")
    );
    assert_eq!(
      parse_callsign("K1ZZ/QRP").slash,
      Some("低功率（CW ≤5W、SSB ≤10W）")
    );
    assert_eq!(
      parse_callsign("BG4XYZ/7").slash,
      Some("在其他分区操作（数字表示分区号）")
    );
  }
}
