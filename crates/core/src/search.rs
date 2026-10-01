//! 全站知识搜索索引：聚合各静态知识模块，提供关键词搜索。

use std::sync::OnceLock;

use crate::{
  amplifier, analog_modes, antenna_analyzer, antenna_array, antenna_diy, antenna_farm,
  antenna_installation, antenna_modeling, antenna_tuning, antennas, aprs, ardf, atv, awards,
  bandplan, bands, beginner, cabrillo, cw_op, dv_network, dx, dxcc, dxpedition, electronics,
  emcomm, eme, eqsl, feedline, filters, frequencies, ft8, gnuradio, grid_system, grounding,
  history, iota, license_classes, logging_software, meters, microwave, mobile, modes, morse,
  most_wanted, muf, nvis, operating, organizations, packet, phonetic, polarization, portable,
  power, power_supply, prefixes, propagation, qrp, qsl_card, receiver, reference, regulations,
  remote, repeater, repeater_build, rfi, rst, rtty, safety, satellites, sdr, solar, special_prop,
  sstv, swl, transceiver, weather_sat, wspr,
};

/// 一条可搜索的知识条目。
#[derive(Debug, Clone)]
pub struct SearchEntry {
  /// 所属页面名。
  pub page: &'static str,
  /// 页面路由。
  pub href: &'static str,
  /// 条目标题。
  pub title: String,
  /// 条目内容。
  pub text: String,
  /// 预存小写的标题（避免每次查询重复转小写）。
  title_lower: String,
  /// 预存小写的内容。
  text_lower: String,
}

fn push(
  out: &mut Vec<SearchEntry>,
  page: &'static str,
  href: &'static str,
  title: String,
  text: String,
) {
  let title_lower = title.to_lowercase();
  let text_lower = text.to_lowercase();
  out.push(SearchEntry {
    page,
    href,
    title,
    text,
    title_lower,
    text_lower,
  });
}

/// 追加 `(标题, 内容)` 二元组。
fn pairs(
  out: &mut Vec<SearchEntry>,
  page: &'static str,
  href: &'static str,
  items: &[(&str, &str)],
) {
  for &(a, b) in items {
    push(out, page, href, a.to_owned(), b.to_owned());
  }
}

/// 追加 `(标题, 二级, 说明)` 三元组。
fn triples(
  out: &mut Vec<SearchEntry>,
  page: &'static str,
  href: &'static str,
  items: &[(&str, &str, &str)],
) {
  for &(a, b, c) in items {
    push(out, page, href, a.to_owned(), format!("{b} · {c}"));
  }
}

/// 追加四元组。
fn quads(
  out: &mut Vec<SearchEntry>,
  page: &'static str,
  href: &'static str,
  items: &[(&str, &str, &str, &str)],
) {
  for &(a, b, c, d) in items {
    push(out, page, href, a.to_owned(), format!("{b} · {c} · {d}"));
  }
}

/// 追加提示列表（标题用页面名）。
fn tips(out: &mut Vec<SearchEntry>, page: &'static str, href: &'static str, items: &[&str]) {
  for &t in items {
    push(out, page, href, page.to_owned(), t.to_owned());
  }
}

/// 构建完整知识索引。
fn build_index() -> Vec<SearchEntry> {
  let mut out = Vec::new();

  // 考试速查
  for c in reference::LICENSE_CLASSES {
    push(
      &mut out,
      "考试速查",
      "/reference",
      format!("{} 类操作证", c.class),
      format!("{} · {} · {}", c.freq, c.power, c.note),
    );
  }
  for a in reference::CALL_AREAS {
    push(
      &mut out,
      "考试速查",
      "/reference",
      format!("{} 区", a.digit),
      a.regions.to_owned(),
    );
  }
  for s in reference::RST_SCALES {
    for &(n, m) in s.levels {
      push(
        &mut out,
        "考试速查",
        "/reference",
        format!("RST {}{}", s.key, n),
        format!("{} · {}", s.name, m),
      );
    }
  }
  for e in reference::EMISSION_TYPES {
    push(
      &mut out,
      "考试速查",
      "/reference",
      format!("{} {}", e.code, e.name),
      e.desc.to_owned(),
    );
  }
  for p in reference::PHRASES {
    push(
      &mut out,
      "考试速查",
      "/reference",
      p.en.to_owned(),
      format!("{} · {}", p.zh, p.usage),
    );
  }

  // 呼号前缀
  for g in prefixes::PREFIX_GROUPS {
    for p in g.prefixes {
      push(
        &mut out,
        "呼号前缀",
        "/prefixes",
        p.prefix.to_owned(),
        format!("{} · {}", g.region, p.entity),
      );
    }
  }

  // 莫尔斯电码
  for m in morse::LETTERS.iter().chain(morse::DIGITS) {
    push(
      &mut out,
      "莫尔斯电码",
      "/morse",
      format!("{} {}", m.ch, m.code),
      "国际摩尔斯电码字符".to_owned(),
    );
  }

  // 字母解释法
  for p in phonetic::PHONETIC {
    push(
      &mut out,
      "字母解释法",
      "/phonetic",
      format!("{} - {}", p.letter, p.word),
      p.pronunciation.to_owned(),
    );
  }

  // RST 信号报告
  pairs(&mut out, "RST 信号报告", "/rst", rst::READABILITY);
  pairs(&mut out, "RST 信号报告", "/rst", rst::SIGNAL_STRENGTH);
  pairs(&mut out, "RST 信号报告", "/rst", rst::TONE);
  pairs(&mut out, "RST 信号报告", "/rst", rst::RST_EXAMPLES);

  // 模拟模式
  for m in analog_modes::ANALOG_MODES {
    push(
      &mut out,
      "模拟模式",
      "/analog-modes",
      m.name.to_owned(),
      format!("{} · {} · {} · {}", m.abbr, m.emission, m.bandwidth, m.desc),
    );
  }
  tips(
    &mut out,
    "模拟模式",
    "/analog-modes",
    analog_modes::SIDEBAND_RULES,
  );
  pairs(
    &mut out,
    "模拟模式",
    "/analog-modes",
    analog_modes::ANALOG_VS_DIGITAL,
  );

  // 数字模式
  for m in modes::DIGITAL_MODES {
    push(
      &mut out,
      "数字模式",
      "/modes",
      m.name.to_owned(),
      format!("{} · {} · {}", m.abbr, m.bandwidth, m.desc),
    );
  }

  // 常用频率
  for g in frequencies::FREQ_GROUPS {
    for &(f, u) in g.freqs {
      push(
        &mut out,
        "常用频率",
        "/frequencies",
        f.to_owned(),
        format!("{} · {}", g.category, u),
      );
    }
  }

  // 传播与电离层
  triples(
    &mut out,
    "传播与电离层",
    "/propagation",
    propagation::LAYERS,
  );
  pairs(
    &mut out,
    "传播与电离层",
    "/propagation",
    propagation::PROPAGATION_MODES,
  );
  pairs(
    &mut out,
    "传播与电离层",
    "/propagation",
    propagation::CONCEPTS,
  );

  // 特殊传播
  triples(
    &mut out,
    "特殊传播",
    "/special-prop",
    special_prop::SPECIAL_MODES,
  );

  // 业余卫星
  for s in satellites::SATELLITES {
    push(
      &mut out,
      "业余卫星",
      "/satellites",
      s.name.to_owned(),
      format!(
        "{} · 上行 {} · 下行 {} · {}",
        s.kind, s.uplink, s.downlink, s.note
      ),
    );
  }
  tips(
    &mut out,
    "业余卫星",
    "/satellites",
    satellites::SATELLITE_TIPS,
  );
  pairs(
    &mut out,
    "业余卫星",
    "/satellites",
    satellites::TRACKING_SOFTWARE,
  );

  // 天线型式
  for a in antennas::ANTENNAS {
    push(
      &mut out,
      "天线型式",
      "/antennas",
      a.name.to_owned(),
      format!("{} · 增益 {} · {}", a.abbr, a.gain, a.desc),
    );
  }

  // 天线匹配与馈线
  triples(&mut out, "天线匹配与馈线", "/feedline", feedline::FEEDLINES);
  pairs(&mut out, "天线匹配与馈线", "/feedline", feedline::MATCHING);
  pairs(&mut out, "天线匹配与馈线", "/feedline", feedline::MISMATCH);

  // 波段表
  for b in bands::BANDS {
    push(
      &mut out,
      "波段表",
      "/bands",
      format!("{} {}", b.name, b.freq_name),
      format!("{} · {}", b.wavelength, b.freq_range),
    );
  }

  // 波段规划
  for b in bandplan::BAND_PLANS {
    push(
      &mut out,
      "波段规划",
      "/bandplan",
      format!("{} 波段", b.band),
      b.freq_range.to_owned(),
    );
    for &(range, mode) in b.segments {
      push(
        &mut out,
        "波段规划",
        "/bandplan",
        range.to_owned(),
        format!("{} · {}", b.band, mode),
      );
    }
  }

  // 通联实务
  pairs(&mut out, "通联实务", "/operating", operating::CONTACT_STEPS);
  pairs(&mut out, "通联实务", "/operating", operating::LOG_FIELDS);
  pairs(&mut out, "通联实务", "/operating", operating::QSL_FIELDS);
  tips(&mut out, "通联实务", "/operating", operating::REPEATER_TIPS);
  tips(
    &mut out,
    "通联实务",
    "/operating",
    operating::GROUNDING_TIPS,
  );

  // 射频安全
  pairs(&mut out, "射频安全", "/safety", safety::SAR_CONCEPTS);
  pairs(&mut out, "射频安全", "/safety", safety::EXPOSURE_LIMITS);
  pairs(&mut out, "射频安全", "/safety", safety::SAFETY_DISTANCE);
  tips(&mut out, "射频安全", "/safety", safety::SAFETY_TIPS);
  tips(&mut out, "射频安全", "/safety", safety::ANTENNA_SAFETY);

  // 电子电路基础
  triples(
    &mut out,
    "电子电路基础",
    "/electronics",
    electronics::COMPONENTS,
  );
  pairs(
    &mut out,
    "电子电路基础",
    "/electronics",
    electronics::CIRCUITS,
  );
  pairs(
    &mut out,
    "电子电路基础",
    "/electronics",
    electronics::FORMULAS,
  );

  // 测量仪表
  triples(&mut out, "测量仪表", "/meters", meters::METERS);

  // 电源与电池
  triples(&mut out, "电源与电池", "/power", power::BATTERIES);
  tips(&mut out, "电源与电池", "/power", power::POWER_TIPS);

  // DX 奖状
  quads(&mut out, "DX 奖状", "/awards", awards::AWARDS);

  // APRS
  pairs(&mut out, "APRS", "/aprs", aprs::APRS_CONCEPTS);
  pairs(&mut out, "APRS", "/aprs", aprs::APRS_FREQS);
  pairs(&mut out, "APRS", "/aprs", aprs::APRS_USES);

  // SDR
  pairs(&mut out, "SDR 无线电", "/sdr", sdr::SDR_CONCEPTS);
  pairs(&mut out, "SDR 无线电", "/sdr", sdr::SDR_SOFTWARE);
  pairs(&mut out, "SDR 无线电", "/sdr", sdr::WEB_SDR);

  // 应急通信
  pairs(&mut out, "应急通信", "/emcomm", emcomm::EMCOMM_CONCEPTS);
  pairs(&mut out, "应急通信", "/emcomm", emcomm::EMCOMM_FREQS);
  tips(&mut out, "应急通信", "/emcomm", emcomm::EMCOMM_TIPS);

  // 新手入门
  pairs(&mut out, "新手入门", "/beginner", beginner::BEGINNER_STEPS);
  triples(&mut out, "新手入门", "/beginner", beginner::RIG_TYPES);
  tips(&mut out, "新手入门", "/beginner", beginner::FIRST_QSO_TIPS);

  // 国际组织与分区
  triples(
    &mut out,
    "国际组织与分区",
    "/organizations",
    organizations::ORGS,
  );
  pairs(
    &mut out,
    "国际组织与分区",
    "/organizations",
    organizations::ZONES,
  );

  // 无线电测向
  pairs(&mut out, "无线电测向", "/ardf", ardf::ARDF_CONCEPTS);
  pairs(&mut out, "无线电测向", "/ardf", ardf::ARDF_BANDS);

  // 天线 DIY
  quads(
    &mut out,
    "天线 DIY",
    "/antenna-diy",
    antenna_diy::DIY_ANTENNAS,
  );

  // 太阳活动
  triples(&mut out, "太阳活动", "/solar", solar::SOLAR_INDICES);
  pairs(&mut out, "太阳活动", "/solar", solar::CONDITIONS);
  tips(&mut out, "太阳活动", "/solar", solar::CYCLE_NOTES);

  // 收发信机
  triples(
    &mut out,
    "收发信机",
    "/transceiver",
    transceiver::RECEIVER_METRICS,
  );
  pairs(
    &mut out,
    "收发信机",
    "/transceiver",
    transceiver::TRANSCEIVER_CONCEPTS,
  );
  tips(
    &mut out,
    "收发信机",
    "/transceiver",
    transceiver::BUYING_TIPS,
  );

  // DX 技巧
  pairs(&mut out, "DX 技巧", "/dx", dx::DX_CONCEPTS);
  tips(&mut out, "DX 技巧", "/dx", dx::DX_TIPS);

  // 电子 QSL
  triples(&mut out, "电子 QSL", "/eqsl", eqsl::EQSL_SERVICES);
  tips(&mut out, "电子 QSL", "/eqsl", eqsl::EQSL_NOTES);

  // 网格定位
  triples(&mut out, "网格定位", "/grid", grid_system::GRID_LEVELS);
  tips(&mut out, "网格定位", "/grid", grid_system::GRID_NOTES);

  // 历史
  pairs(
    &mut out,
    "业余无线电历史",
    "/history",
    history::HISTORY_TIMELINE,
  );

  // 传播预测
  pairs(&mut out, "传播预测", "/muf", muf::MUF_CONCEPTS);
  pairs(&mut out, "传播预测", "/muf", muf::BAND_CHOICE);

  // 户外便携
  triples(
    &mut out,
    "户外便携操作",
    "/portable",
    portable::PORTABLE_PROGRAMS,
  );
  tips(
    &mut out,
    "户外便携操作",
    "/portable",
    portable::PORTABLE_TIPS,
  );

  // CW 操作
  pairs(
    &mut out,
    "CW 操作",
    "/cw-operating",
    cw_op::CW_ABBREVIATIONS,
  );
  tips(&mut out, "CW 操作", "/cw-operating", cw_op::CW_TIPS);

  // 天线架设
  triples(
    &mut out,
    "天线架设",
    "/antenna-installation",
    antenna_installation::SUPPORT_TYPES,
  );
  tips(
    &mut out,
    "天线架设",
    "/antenna-installation",
    antenna_installation::INSTALL_TIPS,
  );

  // FT8
  pairs(&mut out, "FT8 / FT4", "/ft8", ft8::FT8_CONCEPTS);
  pairs(&mut out, "FT8 / FT4", "/ft8", ft8::FT8_FREQS);
  tips(&mut out, "FT8 / FT4", "/ft8", ft8::FT8_TIPS);

  // 中继台
  pairs(
    &mut out,
    "中继台与网关",
    "/repeater",
    repeater::REPEATER_CONCEPTS,
  );
  triples(
    &mut out,
    "中继台与网关",
    "/repeater",
    repeater::DIGITAL_GATEWAYS,
  );
  pairs(
    &mut out,
    "中继台与网关",
    "/repeater",
    repeater::INTERNET_GATEWAYS,
  );

  // WSPR
  pairs(&mut out, "WSPR 信标", "/wspr", wspr::WSPR_CONCEPTS);
  tips(&mut out, "WSPR 信标", "/wspr", wspr::WSPR_NOTES);

  // 日志软件
  triples(
    &mut out,
    "日志与竞赛软件",
    "/logging-software",
    logging_software::LOGGING_SOFTWARE,
  );
  tips(
    &mut out,
    "日志与竞赛软件",
    "/logging-software",
    logging_software::SOFTWARE_NOTES,
  );

  // 微波
  triples(
    &mut out,
    "微波通信",
    "/microwave",
    microwave::MICROWAVE_BANDS,
  );
  pairs(
    &mut out,
    "微波通信",
    "/microwave",
    microwave::MICROWAVE_CONCEPTS,
  );
  tips(
    &mut out,
    "微波通信",
    "/microwave",
    microwave::MICROWAVE_TIPS,
  );

  // 远程
  pairs(&mut out, "远程电台", "/remote", remote::REMOTE_CONCEPTS);
  tips(&mut out, "远程电台", "/remote", remote::REMOTE_TIPS);

  // QSL 卡片
  pairs(
    &mut out,
    "QSL 卡片设计",
    "/qsl-card",
    qsl_card::QSL_REQUIRED,
  );
  tips(
    &mut out,
    "QSL 卡片设计",
    "/qsl-card",
    qsl_card::QSL_DESIGN_TIPS,
  );

  // EME
  pairs(&mut out, "EME 月面反射", "/eme", eme::EME_CONCEPTS);
  triples(&mut out, "EME 月面反射", "/eme", eme::EME_REQUIREMENTS);
  tips(&mut out, "EME 月面反射", "/eme", eme::EME_TIPS);

  // 天线调试
  pairs(
    &mut out,
    "天线调试",
    "/antenna-tuning",
    antenna_tuning::TUNING_STEPS,
  );
  tips(
    &mut out,
    "天线调试",
    "/antenna-tuning",
    antenna_tuning::TUNING_TIPS,
  );

  // DXCC 实体
  for e in dxcc::entities() {
    push(
      &mut out,
      "通联日志",
      "/log",
      e.prefix.to_owned(),
      format!("DXCC 实体：{}（{}）", e.name, e.name_en),
    );
  }

  // 射频干扰与电磁兼容
  pairs(&mut out, "射频干扰排查", "/rfi", rfi::RFI_SOURCES);
  pairs(&mut out, "射频干扰排查", "/rfi", rfi::RFI_SOLUTIONS);
  tips(&mut out, "射频干扰排查", "/rfi", rfi::RFI_TIPS);

  // QRP 低功率操作
  pairs(&mut out, "QRP 低功率", "/qrp", qrp::QRP_CONCEPTS);
  pairs(&mut out, "QRP 低功率", "/qrp", qrp::QRP_RIGS);
  tips(&mut out, "QRP 低功率", "/qrp", qrp::QRP_TIPS);

  // DX 远征
  pairs(
    &mut out,
    "DX 远征",
    "/dxpedition",
    dxpedition::DXPED_CONCEPTS,
  );
  tips(&mut out, "DX 远征", "/dxpedition", dxpedition::DXPED_TIPS);

  // 法规与管理
  pairs(
    &mut out,
    "法规与管理",
    "/regulations",
    regulations::LICENSE_LEVELS,
  );
  pairs(
    &mut out,
    "法规与管理",
    "/regulations",
    regulations::REGULATIONS,
  );
  tips(
    &mut out,
    "法规与管理",
    "/regulations",
    regulations::REG_TIPS,
  );

  // 天线农场
  pairs(
    &mut out,
    "天线农场",
    "/antenna-farm",
    antenna_farm::FARM_FACTORS,
  );
  tips(
    &mut out,
    "天线农场",
    "/antenna-farm",
    antenna_farm::FARM_TIPS,
  );

  // NVIS 近垂直入射天波
  pairs(
    &mut out,
    "NVIS 近垂直入射天波",
    "/nvis",
    nvis::NVIS_CONCEPTS,
  );
  tips(&mut out, "NVIS 近垂直入射天波", "/nvis", nvis::NVIS_TIPS);

  // RTTY / PSK31
  triples(&mut out, "RTTY / PSK31", "/rtty", rtty::RTTY_MODES);
  pairs(&mut out, "RTTY / PSK31", "/rtty", rtty::RTTY_FREQS);
  tips(&mut out, "RTTY / PSK31", "/rtty", rtty::RTTY_TIPS);

  // IOTA 海岛通联
  pairs(&mut out, "IOTA 海岛通联", "/iota", iota::IOTA_CONCEPTS);
  tips(&mut out, "IOTA 海岛通联", "/iota", iota::IOTA_TIPS);

  // SDR 硬件与 GNU Radio
  triples(
    &mut out,
    "SDR 硬件与 GNU Radio",
    "/gnuradio",
    gnuradio::SDR_HARDWARE,
  );
  pairs(
    &mut out,
    "SDR 硬件与 GNU Radio",
    "/gnuradio",
    gnuradio::GNU_RADIO_CONCEPTS,
  );
  tips(
    &mut out,
    "SDR 硬件与 GNU Radio",
    "/gnuradio",
    gnuradio::GNU_RADIO_TIPS,
  );

  // SWL 短波监听
  triples(&mut out, "SWL 短波监听", "/swl", swl::SWL_TARGETS);
  pairs(&mut out, "SWL 短波监听", "/swl", swl::SWL_BANDS);
  tips(&mut out, "SWL 短波监听", "/swl", swl::SWL_TIPS);

  // 功率放大器
  pairs(&mut out, "功率放大器", "/amplifier", amplifier::PA_CONCEPTS);
  tips(&mut out, "功率放大器", "/amplifier", amplifier::PA_TIPS);

  // 业余电视
  pairs(&mut out, "业余电视 ATV / DATV", "/atv", atv::ATV_CONCEPTS);
  tips(&mut out, "业余电视 ATV / DATV", "/atv", atv::ATV_TIPS);

  // 滤波器与双工器
  triples(
    &mut out,
    "滤波器与双工器",
    "/filters",
    filters::FILTER_TYPES,
  );
  tips(&mut out, "滤波器与双工器", "/filters", filters::FILTER_TIPS);

  // 天线建模
  triples(
    &mut out,
    "天线建模软件",
    "/antenna-modeling",
    antenna_modeling::MODELING_SOFTWARE,
  );
  pairs(
    &mut out,
    "天线建模软件",
    "/antenna-modeling",
    antenna_modeling::MODELING_STEPS,
  );
  tips(
    &mut out,
    "天线建模软件",
    "/antenna-modeling",
    antenna_modeling::MODELING_TIPS,
  );

  // DXCC 稀有度
  pairs(
    &mut out,
    "DXCC 稀有度榜单",
    "/most-wanted",
    most_wanted::WANTED_CONCEPTS,
  );
  tips(
    &mut out,
    "DXCC 稀有度榜单",
    "/most-wanted",
    most_wanted::WANTED_TIPS,
  );

  // 中继台建设
  pairs(
    &mut out,
    "中继台建设与维护",
    "/repeater-build",
    repeater_build::REPEATER_BUILD,
  );
  tips(
    &mut out,
    "中继台建设与维护",
    "/repeater-build",
    repeater_build::REPEATER_MAINT,
  );

  // Cabrillo
  pairs(
    &mut out,
    "竞赛 Cabrillo 日志",
    "/cabrillo",
    cabrillo::CABRILLO_CONCEPTS,
  );
  tips(
    &mut out,
    "竞赛 Cabrillo 日志",
    "/cabrillo",
    cabrillo::CABRILLO_TIPS,
  );

  // 天线极化
  pairs(
    &mut out,
    "天线极化",
    "/polarization",
    polarization::POLARIZATION_TYPES,
  );
  tips(
    &mut out,
    "天线极化",
    "/polarization",
    polarization::POLARIZATION_TIPS,
  );

  // 数字语音组网
  triples(
    &mut out,
    "数字语音组网",
    "/dv-network",
    dv_network::DV_NETWORKS,
  );
  tips(
    &mut out,
    "数字语音组网",
    "/dv-network",
    dv_network::DV_NETWORK_TIPS,
  );

  // 接地与防雷
  pairs(
    &mut out,
    "接地与防雷",
    "/grounding",
    grounding::GROUND_TYPES,
  );
  pairs(
    &mut out,
    "接地与防雷",
    "/grounding",
    grounding::GROUNDING_PRACTICE,
  );
  pairs(
    &mut out,
    "接地与防雷",
    "/grounding",
    grounding::SURGE_DEVICES,
  );
  tips(
    &mut out,
    "接地与防雷",
    "/grounding",
    grounding::GROUNDING_TIPS,
  );

  // 天线分析仪与史密斯圆图
  triples(
    &mut out,
    "天线分析仪与史密斯圆图",
    "/antenna-analyzer",
    antenna_analyzer::ANALYZER_TOOLS,
  );
  pairs(
    &mut out,
    "天线分析仪与史密斯圆图",
    "/antenna-analyzer",
    antenna_analyzer::SMITH_CONCEPTS,
  );
  pairs(
    &mut out,
    "天线分析仪与史密斯圆图",
    "/antenna-analyzer",
    antenna_analyzer::ANALYZER_USES,
  );
  tips(
    &mut out,
    "天线分析仪与史密斯圆图",
    "/antenna-analyzer",
    antenna_analyzer::ANALYZER_TIPS,
  );

  // 电源供应
  triples(
    &mut out,
    "电源供应",
    "/power-supply",
    power_supply::SUPPLY_TYPES,
  );
  pairs(
    &mut out,
    "电源供应",
    "/power-supply",
    power_supply::SUPPLY_CONCEPTS,
  );
  tips(
    &mut out,
    "电源供应",
    "/power-supply",
    power_supply::SUPPLY_TIPS,
  );

  // SSTV 慢扫描电视
  pairs(&mut out, "SSTV 慢扫描电视", "/sstv", sstv::SSTV_CONCEPTS);
  pairs(&mut out, "SSTV 慢扫描电视", "/sstv", sstv::SSTV_FREQS);
  triples(&mut out, "SSTV 慢扫描电视", "/sstv", sstv::SSTV_MODES);
  tips(&mut out, "SSTV 慢扫描电视", "/sstv", sstv::SSTV_TIPS);

  // 气象卫星接收
  for s in weather_sat::WEATHER_SATS {
    push(
      &mut out,
      "气象卫星接收",
      "/weather-sat",
      s.name.to_owned(),
      format!("{} · {} · {}", s.signal, s.freq, s.note),
    );
  }
  pairs(
    &mut out,
    "气象卫星接收",
    "/weather-sat",
    weather_sat::WEATHER_CONCEPTS,
  );
  tips(
    &mut out,
    "气象卫星接收",
    "/weather-sat",
    weather_sat::WEATHER_TIPS,
  );

  // Packet 分组无线电
  pairs(
    &mut out,
    "Packet 分组无线电",
    "/packet",
    packet::PACKET_CONCEPTS,
  );
  pairs(
    &mut out,
    "Packet 分组无线电",
    "/packet",
    packet::PACKET_APPS,
  );
  tips(
    &mut out,
    "Packet 分组无线电",
    "/packet",
    packet::PACKET_TIPS,
  );

  // 车载/移动电台
  pairs(
    &mut out,
    "车载/移动电台",
    "/mobile",
    mobile::MOBILE_CONCEPTS,
  );
  pairs(&mut out, "车载/移动电台", "/mobile", mobile::MOBILE_INSTALL);
  tips(&mut out, "车载/移动电台", "/mobile", mobile::MOBILE_TIPS);

  // A/B/C 类操作证权限
  pairs(
    &mut out,
    "A/B/C 类操作证权限",
    "/license-classes",
    license_classes::CLASS_USAGE,
  );
  pairs(
    &mut out,
    "A/B/C 类操作证权限",
    "/license-classes",
    license_classes::BAND_PERMISSIONS,
  );
  tips(
    &mut out,
    "A/B/C 类操作证权限",
    "/license-classes",
    license_classes::CLASS_TIPS,
  );

  // 接收机关键指标
  triples(
    &mut out,
    "接收机关键指标",
    "/receiver",
    receiver::RECEIVER_METRICS,
  );
  pairs(
    &mut out,
    "接收机关键指标",
    "/receiver",
    receiver::NOISE_BASICS,
  );
  tips(
    &mut out,
    "接收机关键指标",
    "/receiver",
    receiver::RECEIVER_TIPS,
  );

  // 天线阵列与相控阵
  pairs(
    &mut out,
    "天线阵列与相控阵",
    "/antenna-array",
    antenna_array::ARRAY_CONCEPTS,
  );
  pairs(
    &mut out,
    "天线阵列与相控阵",
    "/antenna-array",
    antenna_array::ARRAY_TYPES,
  );
  tips(
    &mut out,
    "天线阵列与相控阵",
    "/antenna-array",
    antenna_array::ARRAY_TIPS,
  );

  out
}

/// 返回全站知识索引（惰性构建一次）。
#[must_use]
pub fn knowledge_index() -> &'static [SearchEntry] {
  static INDEX: OnceLock<Vec<SearchEntry>> = OnceLock::new();
  INDEX.get_or_init(build_index)
}

/// 按关键词搜索，返回匹配条目（忽略大小写）。
#[must_use]
pub fn search(query: &str) -> Vec<&'static SearchEntry> {
  let q = query.trim();
  if q.is_empty() {
    return Vec::new();
  }
  let q = q.to_lowercase();
  knowledge_index()
    .iter()
    .filter(|e| e.title_lower.contains(&q) || e.text_lower.contains(&q))
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn index_is_populated() {
    assert!(knowledge_index().len() >= 200, "知识索引条目应足够多");
  }

  #[test]
  fn search_matches_title_and_text() {
    assert!(!search("驻波比").is_empty());
    assert!(!search("DXCC").is_empty());
    assert!(!search("三极管").is_empty());
    assert!(search("").is_empty());
  }
}
