//! 知识页「相关主题」映射：把零散的专题页串成知识网，便于在页脚交叉导航。

/// 相关主题映射：`(当前路由, 相关页面列表（路由, 名称）)`。
pub const RELATED: &[(&str, &[(&str, &str)])] = &[
  // ── 备考速查 ──
  (
    "/reference",
    &[
      ("/cheat-sheet", "考点速查手册"),
      ("/prefixes", "呼号前缀"),
      ("/glossary", "术语表"),
      ("/license-classes", "操作证权限"),
      ("/q-code", "简语"),
    ],
  ),
  (
    "/cheat-sheet",
    &[
      ("/reference", "考试速查"),
      ("/formulas", "公式速查"),
      ("/glossary", "术语表"),
      ("/license-classes", "操作证权限"),
      ("/browse", "题库分类浏览"),
    ],
  ),
  (
    "/formulas",
    &[
      ("/tools", "小工具"),
      ("/cheat-sheet", "考点速查手册"),
      ("/reference", "考试速查"),
    ],
  ),
  (
    "/prefixes",
    &[
      ("/reference", "考试速查"),
      ("/glossary", "术语表"),
      ("/most-wanted", "DXCC 稀有度"),
      ("/callsign", "呼号查询"),
    ],
  ),
  (
    "/glossary",
    &[
      ("/q-code", "简语"),
      ("/reference", "考试速查"),
      ("/prefixes", "呼号前缀"),
    ],
  ),
  (
    "/q-code",
    &[
      ("/glossary", "术语表"),
      ("/cw-operating", "CW 操作"),
      ("/phonetic", "字母解释法"),
    ],
  ),
  (
    "/phonetic",
    &[
      ("/q-code", "简语"),
      ("/morse", "莫尔斯电码"),
      ("/cw-operating", "CW 操作"),
    ],
  ),
  (
    "/rst",
    &[
      ("/operating", "通联实务"),
      ("/cw-operating", "CW 操作"),
      ("/morse", "莫尔斯电码"),
    ],
  ),
  (
    "/morse",
    &[
      ("/cw-operating", "CW 操作"),
      ("/phonetic", "字母解释法"),
      ("/rst", "RST 信号报告"),
    ],
  ),
  (
    "/cw-operating",
    &[
      ("/morse", "莫尔斯电码"),
      ("/rst", "RST 信号报告"),
      ("/operating", "通联实务"),
    ],
  ),
  (
    "/license-classes",
    &[
      ("/reference", "考试速查"),
      ("/license", "执照申办"),
      ("/regulations", "法规与管理"),
    ],
  ),
  // ── 模式 · 传播 ──
  (
    "/analog-modes",
    &[
      ("/modes", "数字模式"),
      ("/frequencies", "常用频率"),
      ("/transceiver", "收发信机"),
    ],
  ),
  (
    "/atv",
    &[
      ("/sstv", "SSTV 慢扫描电视"),
      ("/modes", "数字模式"),
      ("/dv-network", "数字语音组网"),
    ],
  ),
  (
    "/sstv",
    &[
      ("/atv", "业余电视"),
      ("/modes", "数字模式"),
      ("/weather-sat", "气象卫星接收"),
    ],
  ),
  (
    "/modes",
    &[
      ("/ft8", "FT8 / FT4"),
      ("/rtty", "RTTY / PSK31"),
      ("/psk-decode", "PSK31 解码"),
      ("/dv-network", "数字语音组网"),
      ("/sdr", "SDR"),
    ],
  ),
  (
    "/dv-network",
    &[
      ("/modes", "数字模式"),
      ("/repeater", "中继台与网关"),
      ("/sdr", "SDR"),
    ],
  ),
  (
    "/packet",
    &[
      ("/aprs", "APRS"),
      ("/modes", "数字模式"),
      ("/ft8", "FT8 / FT4"),
    ],
  ),
  (
    "/rtty",
    &[
      ("/modes", "数字模式"),
      ("/psk-decode", "PSK31 解码"),
      ("/ft8", "FT8 / FT4"),
      ("/frequencies", "常用频率"),
    ],
  ),
  (
    "/ft8",
    &[
      ("/rtty", "RTTY / PSK31"),
      ("/modes", "数字模式"),
      ("/wspr", "WSPR"),
    ],
  ),
  (
    "/sdr",
    &[
      ("/gnuradio", "GNU Radio"),
      ("/transceiver", "收发信机"),
      ("/modes", "数字模式"),
      ("/sdr-map", "在线 SDR 接收站"),
    ],
  ),
  (
    "/sdr-map",
    &[
      ("/sdr", "SDR 软件定义无线电"),
      ("/gnuradio", "GNU Radio"),
      ("/remote", "远程电台"),
    ],
  ),
  (
    "/gnuradio",
    &[("/sdr", "SDR"), ("/ft8", "FT8 / FT4"), ("/wspr", "WSPR")],
  ),
  (
    "/aprs",
    &[
      ("/packet", "Packet 分组无线电"),
      ("/frequencies", "常用频率"),
      ("/repeater", "中继台与网关"),
    ],
  ),
  (
    "/frequencies",
    &[
      ("/bands", "波段表"),
      ("/bandplan", "波段规划"),
      ("/propagation", "传播与电离层"),
      ("/coordination", "频率协调"),
    ],
  ),
  (
    "/coordination",
    &[
      ("/regulations", "法规与管理"),
      ("/bandplan", "波段规划"),
      ("/organizations", "国际组织与分区"),
      ("/zone-map", "CQ / ITU 分区地图"),
      ("/repeater-build", "中继台建设"),
    ],
  ),
  (
    "/propagation",
    &[
      ("/muf", "传播预测"),
      ("/special-prop", "特殊传播"),
      ("/solar", "太阳活动"),
    ],
  ),
  (
    "/special-prop",
    &[
      ("/propagation", "传播与电离层"),
      ("/eme", "EME 月面反射"),
      ("/wspr", "WSPR"),
      ("/aurora", "极光通信"),
    ],
  ),
  (
    "/aurora",
    &[
      ("/special-prop", "特殊传播"),
      ("/propagation", "传播与电离层"),
      ("/solar", "太阳活动"),
    ],
  ),
  (
    "/eme",
    &[
      ("/special-prop", "特殊传播"),
      ("/satellites", "业余卫星"),
      ("/muf", "传播预测"),
    ],
  ),
  (
    "/muf",
    &[
      ("/propagation", "传播与电离层"),
      ("/solar", "太阳活动"),
      ("/special-prop", "特殊传播"),
    ],
  ),
  (
    "/wspr",
    &[
      ("/ft8", "FT8 / FT4"),
      ("/propagation", "传播与电离层"),
      ("/special-prop", "特殊传播"),
    ],
  ),
  (
    "/weather-sat",
    &[
      ("/satellites", "业余卫星"),
      ("/sdr", "SDR"),
      ("/sstv", "SSTV 慢扫描电视"),
      ("/apt-decoder", "APT 云图解码"),
    ],
  ),
  (
    "/apt-decoder",
    &[
      ("/weather-sat", "气象卫星接收"),
      ("/sstv", "SSTV 慢扫描电视"),
      ("/sdr", "SDR"),
    ],
  ),
  (
    "/psk-decode",
    &[
      ("/modes", "数字模式"),
      ("/rtty", "RTTY / PSK31"),
      ("/psk-reporter", "PSK Reporter"),
    ],
  ),
  (
    "/psk-reporter",
    &[
      ("/psk-decode", "PSK31 解码"),
      ("/rbn", "RBN 信标网络"),
      ("/dx-spots", "DX 实时热点"),
    ],
  ),
  // ── 天线 · 设备 ──
  (
    "/antennas",
    &[
      ("/polarization", "天线极化"),
      ("/feedline", "匹配与馈线"),
      ("/antenna-array", "天线阵列 / 相控阵"),
    ],
  ),
  (
    "/antenna-array",
    &[
      ("/antennas", "天线型式"),
      ("/polarization", "天线极化"),
      ("/antenna-modeling", "天线建模"),
    ],
  ),
  (
    "/polarization",
    &[
      ("/antennas", "天线型式"),
      ("/feedline", "匹配与馈线"),
      ("/antenna-array", "天线阵列 / 相控阵"),
    ],
  ),
  (
    "/feedline",
    &[
      ("/antennas", "天线型式"),
      ("/antenna-tuning", "天线调试"),
      ("/meters", "测量仪表"),
    ],
  ),
  (
    "/antenna-diy",
    &[
      ("/antennas", "天线型式"),
      ("/antenna-modeling", "天线建模"),
      ("/nvis", "NVIS"),
    ],
  ),
  (
    "/antenna-installation",
    &[
      ("/safety", "射频安全"),
      ("/grounding", "接地与防雷"),
      ("/antennas", "天线型式"),
    ],
  ),
  (
    "/antenna-farm",
    &[
      ("/antenna-installation", "天线架设"),
      ("/grounding", "接地与防雷"),
      ("/safety", "射频安全"),
    ],
  ),
  (
    "/antenna-tuning",
    &[
      ("/feedline", "匹配与馈线"),
      ("/antenna-analyzer", "天线分析仪"),
      ("/meters", "测量仪表"),
    ],
  ),
  (
    "/antenna-analyzer",
    &[
      ("/antenna-tuning", "天线调试"),
      ("/meters", "测量仪表"),
      ("/antenna-modeling", "天线建模"),
    ],
  ),
  (
    "/antenna-modeling",
    &[
      ("/antenna-analyzer", "天线分析仪"),
      ("/antenna-diy", "天线 DIY"),
      ("/antennas", "天线型式"),
    ],
  ),
  (
    "/nvis",
    &[
      ("/propagation", "传播与电离层"),
      ("/antennas", "天线型式"),
      ("/portable", "SOTA / POTA"),
    ],
  ),
  (
    "/electronics",
    &[
      ("/filters", "滤波器与双工器"),
      ("/meters", "测量仪表"),
      ("/power", "电源与电池"),
    ],
  ),
  (
    "/filters",
    &[
      ("/electronics", "电子电路基础"),
      ("/meters", "测量仪表"),
      ("/transceiver", "收发信机"),
    ],
  ),
  (
    "/meters",
    &[
      ("/electronics", "电子电路基础"),
      ("/antenna-analyzer", "天线分析仪"),
      ("/safety", "射频安全"),
    ],
  ),
  (
    "/power",
    &[
      ("/power-supply", "电源供应"),
      ("/electronics", "电子电路基础"),
      ("/mobile", "车载 / 移动电台"),
    ],
  ),
  (
    "/power-supply",
    &[
      ("/power", "电源与电池"),
      ("/electronics", "电子电路基础"),
      ("/transceiver", "收发信机"),
    ],
  ),
  (
    "/transceiver",
    &[
      ("/receiver", "接收机指标"),
      ("/amplifier", "功率放大器"),
      ("/sdr", "SDR"),
      ("/gear", "设备评测与选购"),
    ],
  ),
  (
    "/gear",
    &[
      ("/transceiver", "收发信机"),
      ("/receiver", "接收机指标"),
      ("/power-supply", "电源供应"),
      ("/beginner", "新手入门"),
    ],
  ),
  (
    "/receiver",
    &[
      ("/transceiver", "收发信机"),
      ("/filters", "滤波器与双工器"),
      ("/sdr", "SDR"),
    ],
  ),
  (
    "/amplifier",
    &[
      ("/transceiver", "收发信机"),
      ("/power-supply", "电源供应"),
      ("/safety", "射频安全"),
    ],
  ),
  (
    "/bands",
    &[
      ("/frequencies", "常用频率"),
      ("/bandplan", "波段规划"),
      ("/propagation", "传播与电离层"),
    ],
  ),
  (
    "/bandplan",
    &[
      ("/bands", "波段表"),
      ("/frequencies", "常用频率"),
      ("/regulations", "法规与管理"),
    ],
  ),
  (
    "/microwave",
    &[
      ("/bands", "波段表"),
      ("/satellites", "业余卫星"),
      ("/antenna-array", "天线阵列 / 相控阵"),
    ],
  ),
  (
    "/mobile",
    &[
      ("/power", "电源与电池"),
      ("/antennas", "天线型式"),
      ("/safety", "射频安全"),
    ],
  ),
  // ── 通联 · 活动 ──
  (
    "/operating",
    &[
      ("/cw-operating", "CW 操作"),
      ("/rst", "RST 信号报告"),
      ("/q-code", "简语"),
    ],
  ),
  (
    "/contest",
    &[
      ("/cabrillo", "竞赛 Cabrillo"),
      ("/logging-software", "日志与竞赛软件"),
      ("/operating", "通联实务"),
    ],
  ),
  (
    "/cabrillo",
    &[
      ("/contest", "通联竞赛"),
      ("/logging-software", "日志与竞赛软件"),
      ("/log", "通联日志"),
    ],
  ),
  (
    "/awards",
    &[
      ("/most-wanted", "DXCC 稀有度"),
      ("/iota", "IOTA 海岛通联"),
      ("/dx", "DX 技巧"),
    ],
  ),
  (
    "/iota",
    &[
      ("/dx", "DX 技巧"),
      ("/dxpedition", "DX 远征"),
      ("/awards", "DX 奖状"),
    ],
  ),
  (
    "/dx",
    &[
      ("/dxpedition", "DX 远征"),
      ("/most-wanted", "DXCC 稀有度"),
      ("/propagation", "传播与电离层"),
    ],
  ),
  (
    "/dxpedition",
    &[
      ("/dx", "DX 技巧"),
      ("/most-wanted", "DXCC 稀有度"),
      ("/iota", "IOTA 海岛通联"),
    ],
  ),
  (
    "/most-wanted",
    &[
      ("/dx", "DX 技巧"),
      ("/dxpedition", "DX 远征"),
      ("/awards", "DX 奖状"),
    ],
  ),
  (
    "/qrp",
    &[
      ("/operating", "通联实务"),
      ("/portable", "SOTA / POTA"),
      ("/transceiver", "收发信机"),
    ],
  ),
  (
    "/eqsl",
    &[
      ("/qsl-card", "QSL 卡片设计"),
      ("/log", "通联日志"),
      ("/awards", "DX 奖状"),
    ],
  ),
  (
    "/qsl-card",
    &[
      ("/eqsl", "电子 QSL"),
      ("/log", "通联日志"),
      ("/awards", "DX 奖状"),
    ],
  ),
  (
    "/ardf",
    &[
      ("/portable", "SOTA / POTA"),
      ("/safety", "射频安全"),
      ("/emcomm", "应急通信"),
    ],
  ),
  (
    "/emcomm",
    &[
      ("/portable", "SOTA / POTA"),
      ("/safety", "射频安全"),
      ("/qrp", "QRP 低功率"),
    ],
  ),
  (
    "/portable",
    &[
      ("/emcomm", "应急通信"),
      ("/qrp", "QRP 低功率"),
      ("/antennas", "天线型式"),
    ],
  ),
  (
    "/grid",
    &[
      ("/grid-map", "网格地图"),
      ("/dx", "DX 技巧"),
      ("/log", "通联日志"),
      ("/callsign", "呼号查询"),
    ],
  ),
  (
    "/callsign",
    &[
      ("/prefixes", "呼号前缀"),
      ("/most-wanted", "DXCC 稀有度"),
      ("/dxcc-map", "DXCC 世界地图"),
      ("/grid", "网格定位"),
    ],
  ),
  (
    "/repeater",
    &[
      ("/repeater-build", "中继台建设"),
      ("/frequencies", "常用频率"),
      ("/dv-network", "数字语音组网"),
    ],
  ),
  (
    "/repeater-build",
    &[
      ("/repeater", "中继台与网关"),
      ("/antenna-installation", "天线架设"),
      ("/grounding", "接地与防雷"),
    ],
  ),
  (
    "/logging-software",
    &[
      ("/log", "通联日志"),
      ("/cabrillo", "竞赛 Cabrillo"),
      ("/contest", "通联竞赛"),
      ("/open-source", "开源项目与 DIY"),
    ],
  ),
  (
    "/events",
    &[
      ("/contest-calendar", "竞赛日历"),
      ("/dxpedition", "DX 远征"),
      ("/portable", "SOTA / POTA"),
    ],
  ),
  // ── 进阶 · 关于 ──
  (
    "/organizations",
    &[
      ("/regulations", "法规与管理"),
      ("/license", "执照申办"),
      ("/history", "业余无线电历史"),
      ("/zone-map", "CQ / ITU 分区地图"),
    ],
  ),
  (
    "/zone-map",
    &[
      ("/organizations", "国际组织与分区"),
      ("/callsign", "呼号查询"),
      ("/coordination", "频率协调"),
      ("/awards", "DX 奖状"),
    ],
  ),
  (
    "/safety",
    &[
      ("/grounding", "接地与防雷"),
      ("/rfi", "射频干扰排查"),
      ("/antenna-installation", "天线架设"),
    ],
  ),
  (
    "/grounding",
    &[
      ("/safety", "射频安全"),
      ("/antenna-installation", "天线架设"),
      ("/rfi", "射频干扰排查"),
    ],
  ),
  (
    "/rfi",
    &[
      ("/grounding", "接地与防雷"),
      ("/filters", "滤波器与双工器"),
      ("/safety", "射频安全"),
    ],
  ),
  (
    "/beginner",
    &[
      ("/license", "执照申办"),
      ("/reference", "考试速查"),
      ("/operating", "通联实务"),
    ],
  ),
  (
    "/swl",
    &[
      ("/frequencies", "常用频率"),
      ("/propagation", "传播与电离层"),
      ("/sdr", "SDR"),
    ],
  ),
  (
    "/license",
    &[
      ("/regulations", "法规与管理"),
      ("/license-classes", "操作证权限"),
      ("/reference", "考试速查"),
    ],
  ),
  (
    "/regulations",
    &[
      ("/license", "执照申办"),
      ("/organizations", "国际组织与分区"),
      ("/safety", "射频安全"),
    ],
  ),
  (
    "/history",
    &[
      ("/organizations", "国际组织与分区"),
      ("/beginner", "新手入门"),
      ("/regulations", "法规与管理"),
    ],
  ),
  (
    "/remote",
    &[
      ("/sdr", "SDR"),
      ("/logging-software", "日志与竞赛软件"),
      ("/transceiver", "收发信机"),
      ("/sdr-map", "在线 SDR 接收站"),
    ],
  ),
  (
    "/open-source",
    &[
      ("/sdr", "SDR 软件定义无线电"),
      ("/logging-software", "日志与竞赛软件"),
      ("/antenna-diy", "天线 DIY"),
      ("/developers", "开放 API"),
    ],
  ),
  (
    "/developers",
    &[
      ("/callsign", "呼号查询"),
      ("/grid", "网格定位"),
      ("/muf", "传播预测"),
      ("/open-source", "开源项目与 DIY"),
    ],
  ),
  (
    "/community",
    &[
      ("/operating", "通联实务"),
      ("/gear", "设备评测与选购"),
      ("/open-source", "开源项目与 DIY"),
      ("/beginner", "新手入门"),
    ],
  ),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn related_map_populated() {
    assert!(RELATED.len() >= 60);
    for (path, items) in RELATED {
      assert!(!items.is_empty(), "{path} 应有关联主题");
      for (href, label) in *items {
        assert!(
          !href.is_empty() && !label.is_empty(),
          "{path} 的关联项不完整"
        );
      }
    }
  }
}
