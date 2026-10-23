//! 呼号结构解析：前缀（国家/地区）、中国台站类别、分区号、后缀与斜杠后缀。
//!
//! 复用 [`crate::dxcc`] 的实体识别与 [`crate::reference::CALL_AREAS`] 的中国分区表。

use crate::dxcc::lookup_with_prefix;
use crate::reference::CALL_AREAS;

/// 台站类别（中国呼号第二位字母）：`(字母范围, 类别, 说明)`。
///
/// 依据工信部《业余无线电台呼号编制和核发要求》（《业余无线电台管理办法》附件 2，
/// 工业和信息化部令第 67 号，2024-03-01 施行）：
/// - G H I D A B C E F K L 用于一般业余无线电台，省级机构按此顺序依次核发；
/// - J 用于空间业余无线电台；R 用于业余中继台和业余信标台；
/// - S T Y Z 及其他字母由国家无线电管理机构保留。
///
/// 2013 年改革后第二位字母不再表示操作证书级别，只是核发顺序。BY / BZ 旧时指配给
/// 集体台，那是历史惯例，不是现行规定，所以这里不把它们写成「集体台」类别。
pub const STATION_TYPES: &[(&str, &str, &str)] = &[
  (
    "BA–BI、BK、BL",
    "个人业余电台",
    "一般业余无线电台，省级机构按 G、H、I、D、A、B、C、E、F、K、L 的顺序依次核发，BG 最常见",
  ),
  ("BJ", "空间电台", "空间业余无线电台使用，分区号固定为 1"),
  ("BR", "中继台 / 信标台", "业余中继台与业余信标台使用"),
  (
    "BS、BT、BY、BZ",
    "保留",
    "国家无线电管理机构保留；BY、BZ 旧时曾指配给集体台，现行规定没有这一用途",
  ),
];

/// 斜杠后缀含义：`(后缀, 含义, 示例)`。
///
/// 数字后缀（`/0`–`/9`）在 [`slash_of`] 里按「异地分区」单独判定，中文档里合并成一条。
/// 2024 版《业余无线电台管理办法》第三十九条管的是「在他人依法设置的电台上以实际操作人员的
/// 呼号操作」，只要求发送呼号的格式「符合国内国际相关要求」，既没有给异地设台另定写法，
/// 也没有规定具体格式；国际通行写法是后缀数字（`BG4XYZ/6`），2012 版规章写成
/// 「B + 操作地分区号 + / + 本人呼号」（`B6/BG4XYZ`）。
///
/// **解析器只认后缀写法**：`B6/BG4XYZ` 这种前置写法能认出「中国」，但类别、分区、后缀都
/// 解不出来（`parse_callsign` 把斜杠前当主体，见 `old_style_prefix_form_is_not_parsed` 钉住的现状）。
/// 表格里列出它只是说明历史写法，不代表页面能解析。
pub const SLASH_SUFFIXES: &[(&str, &str, &str)] = &[
  ("/P", "便携操作（Portable，在临时地点设台）", "BG4XYZ/P"),
  ("/M", "移动操作（Mobile，车载等移动中）", "BG4XYZ/M"),
  ("/MM", "海上移动（Maritime Mobile）", "BG4XYZ/MM"),
  ("/AM", "航空移动（Aeronautical Mobile）", "BG4XYZ/AM"),
  ("/QRP", "低功率（CW ≤5W、SSB ≤10W）", "BG4XYZ/QRP"),
  ("/QRO", "高功率", "BG4XYZ/QRO"),
  ("/0–/9", "在其他分区操作（数字表示分区号）", "BG4XYZ/6"),
];

/// 呼号结构：`(组成部分, 规则, 示例)`。
pub const CALLSIGN_STRUCTURE: &[(&str, &str, &str)] = &[
  (
    "前缀",
    "国际电联分配的国家 / 地区代号：中国大陆为 B，台湾地区为 BM–BQ、BU–BX，香港为 VR，澳门为 XX9",
    "B",
  ),
  (
    "台站类别",
    "中国呼号第二位字母：一般业余台按 G、H、I、D、A、B、C、E、F、K、L 的顺序核发，J 为空间台，R 为中继台和信标台",
    "G",
  ),
  (
    "分区号",
    "中国呼号第三位数字 0–9，对应省区；其他国家的数字多表示国内划分区",
    "4",
  ),
  (
    "后缀",
    "1–4 位字母（或字母与数字）；个人电台常见 2–3 位字母，1 位、4 位及带数字的由国家机构保留",
    "XYZ",
  ),
  (
    "斜杠后缀",
    "可选，表示操作方式（/P、/M、/QRP）或异地设台（/6）",
    "/P",
  ),
];

/// 中国呼号分区表：`(分区号, 地区, 示例呼号)`。
///
/// 与 [`crate::reference::CALL_AREAS`] 同源（单测钉住），这里多一列示例供表格展示。
pub const CALL_AREA_TABLE: &[(&str, &str, &str)] = &[
  ("1", "北京", "BG1XXX"),
  ("2", "黑龙江 · 吉林 · 辽宁", "BG2XXX"),
  ("3", "天津 · 河北 · 内蒙古 · 山西", "BG3XXX"),
  ("4", "上海 · 山东 · 江苏", "BG4XXX"),
  ("5", "浙江 · 江西 · 福建", "BG5XXX"),
  ("6", "安徽 · 河南 · 湖北", "BG6XXX"),
  ("7", "湖南 · 广东 · 广西 · 海南", "BG7XXX"),
  ("8", "四川 · 重庆 · 贵州 · 云南", "BG8XXX"),
  ("9", "陕西 · 甘肃 · 宁夏 · 青海", "BG9XXX"),
  ("0", "新疆 · 西藏", "BG0XXX"),
];

/// 使用提示（呼号知识总页底部）。
pub const CALLSIGN_TIPS: &[&str] = &[
  "本地解析完全离线：不联网也能查国家 / 地区、DXCC 实体、CQ / ITU 分区与稀有度。",
  "「在线查询」再补全姓名、QTH 与网格，数据来自 Callook（美加）与 HamQTH（国际），纯静态托管时不可用。",
  "在地址后加上 `?call=BA1XX` 可分享查询结果，打开页面即自动发起一次在线查询。",
  "解析后可对照「DXCC 稀有度榜单」判断追台价值，或到「通联日志」录入该呼号。",
  "台站类别、分区号与斜杠后缀都能在本页表格里查到；全球前缀速查见「呼号前缀」页。",
];

/// 呼号规则要点。
pub const CALLSIGN_RULES: &[&str] = &[
  "国际电联《无线电规则》第 19.68 款：业余呼号由前缀、一位数字和不超过 4 个字符的后缀组成，后缀末位必须是字母；前缀为 2 个字符，或 1 个字符（仅限字母 B、F、G、I、K、M、N、R、W）。业余台可以使用数字 0 和 1；特殊场合经主管部门批准，后缀可以更长（第 19.68A 款）。",
  "中国大陆呼号形如「B + 台站类别字母 + 分区数字 + 后缀」，常见 5–6 位，如 BG4XYZ。",
  "台湾地区用 BM–BQ、BU–BX，香港用 VR，澳门用 XX9，都不按中国大陆规则解析第二位字母。",
  "呼号里的数字在中国表示分区；日本 JA1–JA0、美国 W1–W0 等表示各自的国内划分区，含义不同。",
  "斜杠后缀只描述本次操作方式，不改变呼号归属；日志里应同时记录主呼号与后缀。",
  "擅自编制、使用业余无线电台呼号属于禁止行为，要依法承担责任（《业余无线电台管理办法》第四十三条）。",
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

/// 台站类别（按中国呼号第二位字母），返回值与 [`STATION_TYPES`] 的类别列一致。
///
/// 字母含义以工信部 2024 版《呼号编制和核发要求》为准；`S T Y Z` 是「保留」，
/// 不能按旧习惯硬套成特设 / 集体台。
fn station_type_of(c: char) -> Option<&'static str> {
  match c {
    'A'..='I' | 'K' | 'L' => Some("个人业余电台"),
    'J' => Some("空间电台"),
    'R' => Some("中继台 / 信标台"),
    'S' | 'T' | 'Y' | 'Z' => Some("保留"),
    _ => None,
  }
}

/// 斜杠后缀含义，数字后缀按「异地分区」单独判定。
fn slash_of(slash: &str) -> Option<&'static str> {
  let s = slash.trim().to_ascii_uppercase();
  if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) {
    return Some("在其他分区操作（数字表示分区号）");
  }
  SLASH_SUFFIXES
    .iter()
    .find(|(suffix, _, _)| suffix.trim_start_matches('/') == s)
    .map(|(_, meaning, _)| *meaning)
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
    assert_eq!(p.station_type, Some("个人业余电台"));
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

  /// 工信部 2024 版《呼号编制和核发要求》对第二位字母的分配。
  const OFFICIAL_GENERAL: [char; 11] = ['G', 'H', 'I', 'D', 'A', 'B', 'C', 'E', 'F', 'K', 'L'];

  #[test]
  fn station_types_follow_the_2024_allocation() {
    for c in OFFICIAL_GENERAL {
      assert_eq!(station_type_of(c), Some("个人业余电台"), "{c}");
    }
    assert_eq!(station_type_of('J'), Some("空间电台"));
    assert_eq!(station_type_of('R'), Some("中继台 / 信标台"));
    for c in ['S', 'T', 'Y', 'Z'] {
      assert_eq!(station_type_of(c), Some("保留"), "{c}");
    }
    // 其余字母官方只说「保留」，解析器不替它们编造类别。
    for c in ['M', 'N', 'O', 'P', 'Q', 'U', 'V', 'W', 'X'] {
      assert_eq!(station_type_of(c), None, "{c}");
    }
  }

  #[test]
  fn station_types_table_matches_the_parser() {
    // 表里每个类别名都必须是解析器真能返回的值，否则页面表格与解析结果对不上。
    for c in ('A'..='Z').filter(|c| station_type_of(*c).is_some()) {
      let name = station_type_of(c).expect("已过滤");
      assert!(
        STATION_TYPES
          .iter()
          .any(|(_, category, _)| *category == name),
        "{c} → {name} 不在 STATION_TYPES 里"
      );
    }
    // 反方向：表里的类别都能被某个字母解析出来。
    for (letters, category, _) in STATION_TYPES {
      assert!(
        ('A'..='Z').any(|c| station_type_of(c) == Some(*category)),
        "{letters} 的类别 {category} 没有任何字母能解析出来"
      );
    }
  }

  #[test]
  fn parses_reserved_and_space_letters() {
    assert_eq!(
      parse_callsign("BJ1AA").station_type,
      Some("空间电台"),
      "J 只管空间台，信标台属于 R"
    );
    assert_eq!(parse_callsign("BT4D").station_type, Some("保留"));
    assert_eq!(parse_callsign("BY4AA").station_type, Some("保留"));
    assert_eq!(parse_callsign("BK1AA").station_type, Some("个人业余电台"));
    assert_eq!(parse_callsign("BL1AA").station_type, Some("个人业余电台"));
  }

  /// `BR` / `BS` 不在 cty.csv 里（`dxcc.txt` 由它生成，不手改），所以解析不出中国实体。
  /// 官方明确 R 是中继 / 信标台字母，这是上游数据缺口而不是解析规则；这里钉住现状，
  /// 数据补上之后这条会变红，提醒把页面文案里「识别不出」的说明一并去掉。
  #[test]
  fn br_prefix_is_a_known_dxcc_data_gap() {
    let p = parse_callsign("BR4XX");
    assert_eq!(p.entity, None, "cty 补上 BR 后请更新本测试与页面说明");
    assert_eq!(p.station_type, None);
  }

  #[test]
  fn taiwan_prefixes_in_text_match_dxcc_data() {
    // 文案里写的台湾前缀必须与 DXCC 数据里台湾实体的二位前缀一致。
    let text = CALLSIGN_STRUCTURE[0].1;
    assert!(text.contains("BM–BQ、BU–BX"), "{text}");
    for p in ["BM", "BN", "BO", "BP", "BQ", "BU", "BV", "BW", "BX"] {
      let e = crate::dxcc::lookup(&format!("{p}2AA")).expect(p);
      assert_eq!(e.name, "台湾", "{p}");
    }
    // 文案里划出的边界之外（BR、BS、BT、BY）不应被识别成台湾。
    for p in ["BR", "BS", "BT", "BY"] {
      assert_ne!(
        crate::dxcc::lookup(&format!("{p}2AA")).map(|e| e.name),
        Some("台湾"),
        "{p}"
      );
    }
  }

  #[test]
  fn call_area_table_matches_reference() {
    assert_eq!(CALL_AREA_TABLE.len(), CALL_AREAS.len());
    for (row, area) in CALL_AREA_TABLE.iter().zip(CALL_AREAS.iter()) {
      assert_eq!(row.0, area.digit);
      assert_eq!(row.1, area.regions);
    }
  }

  /// 钉住现状:2012 版的前置写法 `B6/BG4XYZ` 只认得出「中国」,类别 / 分区 / 后缀都解不出。
  /// 以后若要支持它,这条会变红,提醒同时改 `SLASH_SUFFIXES` 的说明与页面文案。
  #[test]
  fn old_style_prefix_form_is_not_parsed() {
    let old = parse_callsign("B6/BG4XYZ");
    assert_eq!(old.entity, Some("中国"));
    assert_eq!(old.station_type, None);
    assert_eq!(old.area, None);
    assert_eq!(old.suffix, "");
    assert_eq!(old.slash, None);
    // 同一个台站按国际通行的后缀写法则完整解析。
    let new = parse_callsign("BG4XYZ/6");
    assert_eq!(new.station_type, Some("个人业余电台"));
    assert_eq!(new.area, Some("4"));
    assert_eq!(new.suffix, "XYZ");
    assert!(new.slash.is_some());
  }

  #[test]
  fn slash_suffix_table_is_reachable() {
    for &(suffix, meaning, _) in SLASH_SUFFIXES {
      let raw = suffix.trim_start_matches('/');
      // `/0–/9` 是数字后缀的合并说明，实际解析走数字分支。
      if !raw.chars().all(|c| c.is_ascii_alphabetic()) {
        continue;
      }
      assert_eq!(slash_of(raw), Some(meaning), "{suffix}");
    }
    assert_eq!(slash_of("9"), Some("在其他分区操作（数字表示分区号）"));
  }
}
