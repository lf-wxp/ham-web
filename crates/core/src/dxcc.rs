//! DXCC 实体判断：根据呼号前缀识别所属国家 / 地区（用于日志奖项统计）。

/// 常见 DXCC 实体前缀（按最长前缀匹配，用于日志统计，非完整 DXCC 表）。
pub const DXCC_PREFIXES: &[(&str, &str)] = &[
  ("BV", "台湾"),
  ("VR2", "香港"),
  ("XX9", "澳门"),
  ("KH6", "夏威夷"),
  ("B", "中国"),
  ("JA", "日本"),
  ("HL", "韩国"),
  ("VU", "印度"),
  ("HS", "泰国"),
  ("9M", "马来西亚"),
  ("9V", "新加坡"),
  ("YB", "印度尼西亚"),
  ("DU", "菲律宾"),
  ("G", "英国"),
  ("M", "英国"),
  ("F", "法国"),
  ("DL", "德国"),
  ("DK", "德国"),
  ("I", "意大利"),
  ("EA", "西班牙"),
  ("CT", "葡萄牙"),
  ("ON", "比利时"),
  ("PA", "荷兰"),
  ("HB", "瑞士"),
  ("OE", "奥地利"),
  ("OH", "芬兰"),
  ("SM", "瑞典"),
  ("LA", "挪威"),
  ("OZ", "丹麦"),
  ("SP", "波兰"),
  ("OK", "捷克"),
  ("OM", "斯洛伐克"),
  ("HA", "匈牙利"),
  ("YO", "罗马尼亚"),
  ("RA", "俄罗斯"),
  ("UA", "俄罗斯"),
  ("UR", "乌克兰"),
  ("K", "美国"),
  ("N", "美国"),
  ("W", "美国"),
  ("VE", "加拿大"),
  ("XE", "墨西哥"),
  ("LU", "阿根廷"),
  ("PY", "巴西"),
  ("CE", "智利"),
  ("CX", "乌拉圭"),
  ("VK", "澳大利亚"),
  ("ZL", "新西兰"),
  ("ZS", "南非"),
  ("SU", "埃及"),
  ("CN", "摩洛哥"),
  ("4X", "以色列"),
  ("HZ", "沙特阿拉伯"),
];

/// 根据呼号判断 DXCC 实体（按最长前缀匹配），无法识别返回 `None`。
#[must_use]
pub fn dxcc_entity(callsign: &str) -> Option<&'static str> {
  let cs = callsign.trim().to_ascii_uppercase();
  if cs.is_empty() {
    return None;
  }
  let mut best: Option<(usize, &'static str)> = None;
  for &(prefix, entity) in DXCC_PREFIXES {
    if cs.starts_with(prefix) && best.is_none_or(|(len, _)| prefix.len() > len) {
      best = Some((prefix.len(), entity));
    }
  }
  best.map(|(_, e)| e)
}

/// DXCC 实体 → (CQ 分区, ITU 分区)。
pub const ENTITY_ZONES: &[(&str, &str, &str)] = &[
  ("中国", "23/24", "43/44"),
  ("台湾", "24", "44"),
  ("香港", "24", "44"),
  ("澳门", "24", "44"),
  ("夏威夷", "31", "61"),
  ("日本", "25", "45"),
  ("韩国", "25", "44"),
  ("印度", "22", "41"),
  ("泰国", "26", "49"),
  ("马来西亚", "28", "54"),
  ("新加坡", "28", "54"),
  ("印度尼西亚", "28", "51/54"),
  ("菲律宾", "27", "50"),
  ("英国", "14", "27"),
  ("法国", "14", "27"),
  ("德国", "14", "28"),
  ("意大利", "15", "28"),
  ("西班牙", "14", "37"),
  ("葡萄牙", "14", "37"),
  ("比利时", "14", "27"),
  ("荷兰", "14", "27"),
  ("瑞士", "14", "28"),
  ("奥地利", "15", "28"),
  ("芬兰", "15", "18"),
  ("瑞典", "14", "18"),
  ("挪威", "14", "18"),
  ("丹麦", "14", "18"),
  ("波兰", "15", "28"),
  ("捷克", "15", "28"),
  ("斯洛伐克", "15", "28"),
  ("匈牙利", "15", "28"),
  ("罗马尼亚", "20", "28"),
  ("俄罗斯", "16/17/18", "19/29/30"),
  ("乌克兰", "16", "29"),
  ("美国", "3/4/5", "6/7/8"),
  ("加拿大", "1/2/3/4", "2/3/4/9"),
  ("墨西哥", "6", "10"),
  ("阿根廷", "13", "14"),
  ("巴西", "11/13", "13/15"),
  ("智利", "12", "14"),
  ("乌拉圭", "13", "14"),
  ("澳大利亚", "29/30", "55/59"),
  ("新西兰", "32", "60"),
  ("南非", "38", "57"),
  ("埃及", "34", "38"),
  ("摩洛哥", "33", "37"),
  ("以色列", "20", "39"),
  ("沙特阿拉伯", "21", "39"),
];

/// 根据 DXCC 实体名返回 `(CQ 分区, ITU 分区)`。
#[must_use]
pub fn entity_zones(entity: &str) -> Option<(&'static str, &'static str)> {
  ENTITY_ZONES
    .iter()
    .find(|(e, _, _)| *e == entity)
    .map(|(_, cq, itu)| (*cq, *itu))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn matches_longest_prefix_first() {
    assert_eq!(dxcc_entity("BG4XXX"), Some("中国"));
    assert_eq!(dxcc_entity("BV2AA"), Some("台湾"));
    assert_eq!(dxcc_entity("JA1ABC"), Some("日本"));
    assert_eq!(dxcc_entity("K1ZZ"), Some("美国"));
    assert_eq!(dxcc_entity("DL1AA"), Some("德国"));
    assert_eq!(dxcc_entity("VR2XY"), Some("香港"));
    assert_eq!(dxcc_entity(""), None);
    assert_eq!(dxcc_entity("1A"), None);
  }

  #[test]
  fn entity_zones_lookup() {
    assert_eq!(entity_zones("中国"), Some(("23/24", "43/44")));
    assert_eq!(entity_zones("日本"), Some(("25", "45")));
    assert_eq!(entity_zones("未知"), None);
  }
}
