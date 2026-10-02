//! 知识库专题自测：把专题页路由映射到一级分类，供页面底部「自测 3 题」按分类码选题。

/// 专题页路由 → 一级分类 key（用于自测选题）。
pub const TOPIC_TOPS: &[(&str, &str)] = &[
  // 备考速查
  ("/reference", "操作"),
  ("/prefixes", "法规"),
  ("/glossary", "用语"),
  ("/q-code", "用语"),
  ("/phonetic", "用语"),
  ("/rst", "操作"),
  ("/morse", "用语"),
  ("/cw-operating", "操作"),
  ("/license-classes", "法规"),
  ("/cheat-sheet", "基础"),
  ("/formulas", "基础"),
  // 模式 · 传播
  ("/analog-modes", "调制"),
  ("/atv", "调制"),
  ("/sstv", "调制"),
  ("/modes", "调制"),
  ("/dv-network", "调制"),
  ("/packet", "调制"),
  ("/rtty", "调制"),
  ("/ft8", "调制"),
  ("/sdr", "设备"),
  ("/gnuradio", "设备"),
  ("/aprs", "调制"),
  ("/frequencies", "频率"),
  ("/propagation", "传播"),
  ("/special-prop", "传播"),
  ("/eme", "传播"),
  ("/muf", "传播"),
  ("/wspr", "传播"),
  ("/weather-sat", "传播"),
  ("/apt-decoder", "传播"),
  // 天线 · 设备
  ("/antennas", "天线"),
  ("/polarization", "天线"),
  ("/feedline", "天线"),
  ("/antenna-diy", "天线"),
  ("/antenna-installation", "天线"),
  ("/antenna-farm", "天线"),
  ("/antenna-tuning", "天线"),
  ("/antenna-analyzer", "天线"),
  ("/antenna-modeling", "天线"),
  ("/antenna-array", "天线"),
  ("/nvis", "天线"),
  ("/electronics", "基础"),
  ("/filters", "基础"),
  ("/meters", "基础"),
  ("/power", "基础"),
  ("/power-supply", "基础"),
  ("/transceiver", "设备"),
  ("/receiver", "设备"),
  ("/amplifier", "设备"),
  ("/bands", "频率"),
  ("/bandplan", "频率"),
  ("/microwave", "频率"),
  ("/mobile", "设备"),
  // 通联 · 活动
  ("/operating", "操作"),
  ("/contest", "操作"),
  ("/cabrillo", "操作"),
  ("/awards", "操作"),
  ("/iota", "操作"),
  ("/dx", "操作"),
  ("/dxpedition", "操作"),
  ("/most-wanted", "操作"),
  ("/qrp", "操作"),
  ("/eqsl", "操作"),
  ("/qsl-card", "操作"),
  ("/ardf", "操作"),
  ("/emcomm", "操作"),
  ("/portable", "操作"),
  ("/grid", "操作"),
  ("/repeater", "操作"),
  ("/repeater-build", "操作"),
  ("/logging-software", "操作"),
  // 进阶 · 关于
  ("/organizations", "法规"),
  ("/safety", "安全"),
  ("/grounding", "安全"),
  ("/rfi", "安全"),
  ("/beginner", "法规"),
  ("/swl", "操作"),
  ("/license", "法规"),
  ("/regulations", "法规"),
  ("/history", "法规"),
  ("/remote", "设备"),
];

/// 专题页路由 → 一级分类 key。
#[must_use]
pub fn page_top(route: &str) -> Option<&'static str> {
  TOPIC_TOPS
    .iter()
    .find(|(r, _)| *r == route)
    .map(|(_, top)| *top)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::categories::sub_codes_of;

  #[test]
  fn mapping_is_complete_and_valid() {
    assert!(TOPIC_TOPS.len() >= 60);
    // 每个 key 唯一，且分类存在、有可选的分类码。
    let mut seen = std::collections::HashSet::new();
    for (route, top) in TOPIC_TOPS {
      assert!(route.starts_with('/'), "{route}");
      assert!(seen.insert(route), "重复路由 {route}");
      assert!(
        crate::categories::top_category(top).is_some(),
        "未知分类 {top}（{route}）"
      );
      assert!(!sub_codes_of(top).is_empty(), "{top} 无分类码");
    }
  }

  #[test]
  fn page_top_lookup() {
    assert_eq!(page_top("/regulations"), Some("法规"));
    assert_eq!(page_top("/ohms-law"), None);
  }
}
