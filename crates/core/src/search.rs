//! 全站知识搜索索引：聚合各静态知识模块，提供关键词搜索。

use std::sync::OnceLock;

use crate::{
  amplifier, analog_modes, antenna_analyzer, antenna_array, antenna_diy, antenna_farm,
  antenna_installation, antenna_modeling, antenna_tuning, antennas, api_v1, aprs, ardf, atv,
  aurora, awards, bandplan, bands, beginner, cabrillo, community, connectors, coordination, cw_op,
  digital_comms, dsp_basics, dv_network, dx, dxcc, dxpedition, electronics, emcomm, eme, eqsl,
  events, feedline, filters, frequencies, ft8, gear, gnuradio, grid_system, grounding, history,
  iota, license_classes, logging_software, meters, microwave, mobile, modes, modulation_theory,
  morse, most_wanted, muf, nvis, open_source, operating, organizations, packet, phonetic,
  polarization, portable, power, power_supply, prefixes, propagation, qrp, qsl_card, receiver,
  reference, regulations, remote, repeater, repeater_build, rfi, rst, rtty, safety, satellites,
  sdr, sdr_map, solar, special_prop, sstv, swl, transceiver, weather_sat, wspr, zone,
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
  /// 领域标签（`dxcc:by`、`band:40m`、`mode:ft8` 等），供 `key:value` 前缀过滤。
  pub tags: Vec<String>,
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
  push_tagged(out, page, href, title, text, &[]);
}

/// 带领域标签的入库（`dxcc:` / `band:` / `mode:` 前缀过滤的依据）。
fn push_tagged(
  out: &mut Vec<SearchEntry>,
  page: &'static str,
  href: &'static str,
  title: String,
  text: String,
  tags: &[&str],
) {
  let title_lower = title.to_lowercase();
  let text_lower = text.to_lowercase();
  out.push(SearchEntry {
    page,
    href,
    title,
    text,
    tags: tags.iter().map(|t| (*t).to_owned()).collect(),
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

/// 追加五元组（只索引前四列；第五列是 `.nec` 模板 id，不进搜索文本）。
fn quints(
  out: &mut Vec<SearchEntry>,
  page: &'static str,
  href: &'static str,
  items: &[(&str, &str, &str, &str, &str)],
) {
  for &(a, b, c, d, _) in items {
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
  triples(
    &mut out,
    "考试速查",
    "/reference",
    reference::EMISSION_DESIGNATION,
  );
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
    push_tagged(
      &mut out,
      "模拟模式",
      "/analog-modes",
      m.name.to_owned(),
      format!("{} · {} · {} · {}", m.abbr, m.emission, m.bandwidth, m.desc),
      &[&format!("mode:{}", m.abbr.to_lowercase())],
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
    push_tagged(
      &mut out,
      "数字模式",
      "/modes",
      m.name.to_owned(),
      format!("{} · {} · {}", m.abbr, m.bandwidth, m.desc),
      &[&format!("mode:{}", m.abbr.to_lowercase())],
    );
  }

  // 调制理论
  pairs(
    &mut out,
    "调制理论",
    "/modulation",
    modulation_theory::MOD_CONCEPTS,
  );
  triples(
    &mut out,
    "调制理论",
    "/modulation",
    modulation_theory::MOD_SCHEMES,
  );
  triples(
    &mut out,
    "调制理论",
    "/modulation",
    modulation_theory::MOD_BANDWIDTH,
  );
  pairs(
    &mut out,
    "调制理论",
    "/modulation",
    modulation_theory::FM_EFFECTS,
  );
  triples(
    &mut out,
    "调制理论",
    "/modulation",
    modulation_theory::DEMOD_METHODS,
  );
  tips(
    &mut out,
    "调制理论",
    "/modulation",
    modulation_theory::MOD_TIPS,
  );

  // 数字通信原理
  pairs(
    &mut out,
    "数字通信原理",
    "/digital-comms",
    digital_comms::DIGITAL_CONCEPTS,
  );
  triples(
    &mut out,
    "数字通信原理",
    "/digital-comms",
    digital_comms::DIGITAL_SCHEMES,
  );
  triples(
    &mut out,
    "数字通信原理",
    "/digital-comms",
    digital_comms::DIGITAL_FORMULAS,
  );
  triples(
    &mut out,
    "数字通信原理",
    "/digital-comms",
    digital_comms::CHANNEL_CODING,
  );
  pairs(
    &mut out,
    "数字通信原理",
    "/digital-comms",
    digital_comms::DIGITAL_STEPS,
  );
  tips(
    &mut out,
    "数字通信原理",
    "/digital-comms",
    digital_comms::DIGITAL_TIPS,
  );

  // 数字信号处理基础
  pairs(
    &mut out,
    "数字信号处理基础",
    "/dsp",
    dsp_basics::DSP_CONCEPTS,
  );
  triples(
    &mut out,
    "数字信号处理基础",
    "/dsp",
    dsp_basics::DSP_FORMULAS,
  );
  triples(
    &mut out,
    "数字信号处理基础",
    "/dsp",
    dsp_basics::WINDOW_TABLE,
  );
  triples(
    &mut out,
    "数字信号处理基础",
    "/dsp",
    dsp_basics::FILTER_TYPES,
  );
  pairs(&mut out, "数字信号处理基础", "/dsp", dsp_basics::DSP_STEPS);
  tips(&mut out, "数字信号处理基础", "/dsp", dsp_basics::DSP_TIPS);

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
  pairs(&mut out, "天线型式", "/antennas", antennas::ANTENNA_THEORY);

  // 天线匹配与馈线
  triples(&mut out, "天线匹配与馈线", "/feedline", feedline::FEEDLINES);
  pairs(&mut out, "天线匹配与馈线", "/feedline", feedline::MATCHING);
  pairs(&mut out, "天线匹配与馈线", "/feedline", feedline::MISMATCH);

  // 线材与连接器
  triples(
    &mut out,
    "线材与连接器",
    "/connectors",
    connectors::CONNECTORS,
  );
  triples(&mut out, "线材与连接器", "/connectors", connectors::CABLES);
  tips(
    &mut out,
    "线材与连接器",
    "/connectors",
    connectors::SELECTION_TIPS,
  );
  tips(
    &mut out,
    "线材与连接器",
    "/connectors",
    connectors::INSTALL_TIPS,
  );
  tips(&mut out, "线材与连接器", "/connectors", connectors::FAULTS);

  // 波段表（ITU 频段）
  for b in bands::BANDS {
    push_tagged(
      &mut out,
      "波段表",
      "/bands",
      format!("{} {}", b.name, b.freq_name),
      format!("{} · {}", b.wavelength, b.freq_range),
      &[&format!("band:{}", b.freq_abbr.to_lowercase())],
    );
  }

  // 波段规划（业余波段）
  for b in bandplan::BAND_PLANS {
    let tag = format!("band:{}", b.band.to_lowercase());
    push_tagged(
      &mut out,
      "波段规划",
      "/bandplan",
      format!("{} 波段", b.band),
      b.freq_range.to_owned(),
      &[&tag],
    );
    for seg in b.segments {
      push_tagged(
        &mut out,
        "波段规划",
        "/bandplan",
        seg.range.to_owned(),
        format!("{} · {}", b.band, seg.text),
        &[&tag],
      );
    }
  }

  // 多国题库对照
  for s in crate::exam_comparison::SYSTEMS {
    push(
      &mut out,
      "知识库",
      "/exam-comparison",
      format!("{}：{}", s.country, s.title),
      s.coverage.to_owned(),
    );
  }

  // 天线理论
  push(
    &mut out,
    "知识库",
    "/antenna-theory",
    "天线理论".to_owned(),
    "近远场边界与品质因数".to_owned(),
  );
  for &(title, formulas) in crate::antenna_theory::THEORY_SECTIONS {
    for f in formulas.iter() {
      push(
        &mut out,
        "知识库",
        "/antenna-theory",
        f.name.to_owned(),
        format!("{}：{}", title, f.desc),
      );
    }
  }

  // 频谱管理
  push(
    &mut out,
    "知识库",
    "/spectrum-admin",
    "频谱管理".to_owned(),
    "干扰查处流程与 ITU 条款结构".to_owned(),
  );
  for &(title, text) in crate::spectrum_admin::MONITORING_STEPS {
    push(
      &mut out,
      "知识库",
      "/spectrum-admin",
      title.to_owned(),
      text.to_owned(),
    );
  }

  // 测量实验室
  push(
    &mut out,
    "知识库",
    "/measurement-lab",
    "测量实验室".to_owned(),
    "驻波读数误差与 SOLT 校准的交互演示".to_owned(),
  );
  for &(title, text) in crate::meters::SWR_PITFALLS {
    push(
      &mut out,
      "知识库",
      "/measurement-lab",
      title.to_owned(),
      text.to_owned(),
    );
  }

  // DIY 项目卡片
  for project in crate::diy_projects::DIY_PROJECTS {
    push(
      &mut out,
      "知识库",
      "/diy-projects",
      project.name.to_owned(),
      project.notes.to_owned(),
    );
  }

  // 法规原文库：逐条索引（条款级检索 —— 搜「第三十七条」直接命中条文）。
  for law in crate::radio_law::laws() {
    push(
      &mut out,
      "知识库",
      "/radio-law",
      law.title.to_owned(),
      format!("{} 全文", law.title),
    );
    for article in crate::radio_law::articles(law.key) {
      // 法律原文用中文数字（「第三十七条」），用户也这么搜：标题写阿拉伯数字，
      // 内容前缀补中文写法，两种都能命中。
      let zh = crate::radio_law::to_zh_number(article.number).unwrap_or_default();
      push(
        &mut out,
        "知识库",
        "/radio-law",
        format!("{} 第{}条", law.title, article.number),
        format!("第{zh}条 {}", article.text),
      );
    }
  }

  // 设备评测文章
  for r in crate::rig_reviews::RIG_REVIEWS {
    if let Some(g) = crate::gear::by_id(r.rig_id) {
      push(
        &mut out,
        "知识库",
        "/rig-reviews",
        format!("{} {} 评测", g.brand, g.model),
        format!("{} {}", g.brand, g.model),
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
  quints(
    &mut out,
    "天线 DIY",
    "/antenna-diy",
    antenna_diy::DIY_ANTENNAS,
  );

  // 太阳活动
  triples(&mut out, "太阳活动", "/solar", solar::SOLAR_INDICES);
  pairs(&mut out, "太阳活动", "/solar", solar::CONDITIONS);
  tips(&mut out, "太阳活动", "/solar", solar::CYCLE_NOTES);

  // 收发信机（接收机关键指标由 /receiver 统一索引，避免同一数据重复命中）
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
    push_tagged(
      &mut out,
      "通联日志",
      "/log",
      e.prefix.to_owned(),
      format!("DXCC 实体：{}（{}）", e.name, e.name_en),
      &[&format!("dxcc:{}", e.prefix.to_lowercase())],
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

  // 极光通信
  pairs(&mut out, "极光通信", "/aurora", aurora::AURORA_CONCEPTS);
  triples(&mut out, "极光通信", "/aurora", aurora::AURORA_BANDS);
  pairs(&mut out, "极光通信", "/aurora", aurora::AURORA_FORECAST);
  tips(&mut out, "极光通信", "/aurora", aurora::AURORA_TIPS);

  // 在线 SDR 接收站
  for s in sdr_map::SDR_SITES {
    push(
      &mut out,
      "在线 SDR 接收站",
      "/sdr-map",
      s.name.to_owned(),
      format!("{} · {} · {}", s.kind, s.location, s.desc),
    );
  }
  for d in sdr_map::SDR_DIRECTORIES {
    push(
      &mut out,
      "在线 SDR 接收站",
      "/sdr-map",
      d.name.to_owned(),
      format!("{} · {}", d.kind, d.desc),
    );
  }
  tips(
    &mut out,
    "在线 SDR 接收站",
    "/sdr-map",
    sdr_map::SDR_MAP_TIPS,
  );

  // 开源项目与 DIY 索引
  for p in open_source::OSS_PROJECTS {
    push(
      &mut out,
      "开源项目与 DIY",
      "/open-source",
      p.name.to_owned(),
      format!("{} · {}", p.category, p.desc),
    );
  }
  for (_, name, desc) in open_source::DIY_GUIDES {
    push(
      &mut out,
      "开源项目与 DIY",
      "/open-source",
      (*name).to_owned(),
      (*desc).to_owned(),
    );
  }

  // 火腿社区
  for l in community::COMMUNITY_LINKS {
    push(
      &mut out,
      "火腿社区",
      "/community",
      l.name.to_owned(),
      format!("{} · {}", l.category, l.desc),
    );
  }

  // 活动日历
  for e in events::HAM_EVENTS {
    push(
      &mut out,
      "活动日历",
      "/events",
      e.name.to_owned(),
      format!("{} · {} · {}", e.kind, e.location, e.desc),
    );
  }

  // 频率协调
  pairs(
    &mut out,
    "频率协调",
    "/coordination",
    coordination::COORDINATION_CONCEPTS,
  );
  // 设备评测与选购
  for g in gear::GEAR {
    push(
      &mut out,
      "设备评测与选购",
      "/gear",
      format!("{} {}", g.brand, g.model),
      format!("{} · {} · {} · {}", g.tier, g.bands, g.power, g.highlight),
    );
  }
  triples(&mut out, "设备评测与选购", "/gear", gear::GEAR_PICKS);
  tips(&mut out, "设备评测与选购", "/gear", gear::GEAR_TIPS);

  // CQ / ITU 分区地图
  pairs(
    &mut out,
    "CQ / ITU 分区地图",
    "/zone-map",
    zone::ZONE_CONCEPTS,
  );
  tips(&mut out, "CQ / ITU 分区地图", "/zone-map", zone::ZONE_TIPS);
  pairs(
    &mut out,
    "频率协调",
    "/coordination",
    coordination::COORDINATION_TIERS,
  );
  triples(
    &mut out,
    "频率协调",
    "/coordination",
    coordination::IARU_BAND_DIFFS,
  );
  pairs(
    &mut out,
    "频率协调",
    "/coordination",
    coordination::COORDINATION_STEPS,
  );
  pairs(
    &mut out,
    "频率协调",
    "/coordination",
    coordination::INTERFERENCE_STEPS,
  );
  tips(
    &mut out,
    "频率协调",
    "/coordination",
    coordination::COORDINATION_TIPS,
  );

  // 开放 API
  for e in api_v1::ENDPOINTS {
    push(
      &mut out,
      "开放 API",
      "/developers",
      e.path.to_owned(),
      format!("{} · {}", e.tag, e.summary),
    );
  }

  // 工具与页面入口（导航注册表：路径 + 中文标题 + 所属分组）。
  // 排在索引最后 —— 同分时知识条目自然靠前，页面入口不挤占术语/知识的结果。
  for m in crate::registry::MODULES {
    push(
      &mut out,
      m.group.unwrap_or("页面"),
      m.href.unwrap_or(m.path),
      m.title.to_owned(),
      "页面入口".to_owned(),
    );
  }

  // 公式速查：公式名 / 表达式 / 一句话说明都是用户会搜的词（「欧姆定律」「驻波比」「P = U·I」），
  // 此前整页都不在索引里，只能靠页面名「公式速查」命中。
  // 如实说明：「单位换算」区块的名称与示例是 i18n key（`exam.units-*`），core 拿不到译文，
  // 所以这一块不进索引；`decibel` 是各计算器的数值函数，没有独立页面，同样不收。
  for g in crate::formulas::FORMULA_GROUPS {
    for f in g.items {
      push(
        &mut out,
        "公式速查",
        "/formulas",
        f.name.to_owned(),
        format!("{} · {} · {}", f.expr, f.desc, g.name),
      );
    }
  }
  out
}

#[must_use]
pub fn knowledge_index() -> &'static [SearchEntry] {
  static INDEX: OnceLock<Vec<SearchEntry>> = OnceLock::new();
  INDEX.get_or_init(build_index)
}

/// 搜索结果条数上限。
///
/// 早期版本返回**全部**命中：搜「天线」这类高频词会一次返回数百条，既拖慢渲染
/// 也让用户找不到真正想要的。这里截断到最相关的若干条。
pub const MAX_RESULTS: usize = 50;

/// 一条搜索命中（含相关度得分）。
#[derive(Debug, Clone, Copy)]
pub struct SearchHit {
  /// 命中的条目。
  pub entry: &'static SearchEntry,
  /// 相关度得分，越大越相关。
  pub score: u32,
}

/// 把查询串切成检索词：按空白与常见中英文顿句符切分。
///
/// 中文没有可用的分词词典，因此不做分词；但「天线 驻波比」这类多词查询很常见，
/// 切成两个词后按 AND 匹配，比把整串（含空格）当成一个子串去查实用得多。
///
/// 对外公开：前端（术语表 / 题库搜索、关键词高亮）必须与检索侧用同一套切词规则，
/// 否则会出现「搜得到但不高亮」「知识库有结果而题库没有」这类割裂。
#[must_use]
pub fn tokenize(query: &str) -> Vec<String> {
  query
    .split(|c: char| c.is_whitespace() || matches!(c, '、' | ',' | '，' | '；' | ';' | '/'))
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .map(str::to_lowercase)
    .collect()
}

/// 查询解析：拆成自由检索词与 `key:value` 领域前缀。
///
/// 支持的前缀（大小写不敏感）：`dxcc:`（DXCC 实体前缀）、`band:`（波段）、
/// `mode:`（模式缩写）。其余含冒号的词按普通检索词处理（不吞掉「1:1 巴伦」
/// 这类真实查询）。
#[must_use]
pub fn parse_query(query: &str) -> (Vec<String>, Vec<(String, String)>) {
  let mut terms = Vec::new();
  let mut filters = Vec::new();
  for tok in tokenize(query) {
    if let Some((key, value)) = tok.split_once(':')
      && matches!(key, "dxcc" | "band" | "mode")
      && !value.is_empty()
    {
      filters.push((key.to_owned(), value.to_owned()));
      continue;
    }
    terms.push(tok);
  }
  (terms, filters)
}

/// 本地日志里的一条 QSO 是否命中查询（检索面板「日志」分组的判定）。
///
/// 自由词在呼号 / 模式 / 波段 / 频率 / 日期 / 姓名 / QTH / 网格上做 AND 匹配。
/// 领域前缀与静态条目**同一套语义**：不同 key 之间 AND、同一 key 多次出现 OR；
/// 取值口径也与静态标签一致，因此同一个 `dxcc:by` 既能命中「DXCC 实体：中国」
/// 也能命中日志里的 BY 通联：
/// - `dxcc:` 比的是实体**主前缀**（[`LogEntry::entity`]：记录里的 DXCC 编号优先，
///   否则按呼号推断）—— 不比呼号字面前缀，否则 `dxcc:ja` 会漏掉 7J / 8N 这些日本呼号
/// - `band:` 比的是 [`LogEntry::band_label`]（频率比 `BAND` 字段可信）
/// - `mode:` 比的是记录模式（忽略大小写）
#[must_use]
pub fn qso_matches(
  e: &crate::logbook::LogEntry,
  terms: &[String],
  filters: &[(String, String)],
) -> bool {
  let hay = [
    e.callsign.as_str(),
    e.mode.as_str(),
    e.band.as_str(),
    e.freq.as_str(),
    e.date.as_str(),
    e.name.as_str(),
    e.qth.as_str(),
    e.gridsquare.as_str(),
  ]
  .join(" ")
  .to_lowercase();
  if !terms.iter().all(|t| hay.contains(t.as_str())) {
    return false;
  }
  let value_of = |key: &str| -> Option<String> {
    match key {
      "dxcc" => e.entity().map(|en| en.prefix.to_lowercase()),
      "band" => Some(e.band_label().to_lowercase()),
      "mode" => Some(e.mode.trim().to_lowercase()),
      _ => None,
    }
  };
  let mut keys: Vec<&str> = filters.iter().map(|(k, _)| k.as_str()).collect();
  keys.sort_unstable();
  keys.dedup();
  keys.iter().all(|key| {
    let Some(actual) = value_of(key) else {
      return false;
    };
    filters
      .iter()
      .filter(|(k, _)| k == key)
      .any(|(_, v)| *v == actual)
  })
}

/// 字节偏移 → 字符偏移。
///
/// `str::find` 返回的是字节偏移，而中文一个字占 3 字节：直接拿字节偏移算位置分，
/// 中文条目的附加分会比英文条目衰减快三倍，排序被无谓地打偏。`find` 的返回值一定
/// 落在字符边界上，因此这里切片是安全的。
fn char_offset(s: &str, byte_pos: usize) -> usize {
  s[..byte_pos].chars().count()
}

/// 命中位置越靠前越相关（按**字符**计位），附加分 0–9。
fn position_bonus(pos: usize) -> u32 {
  9 - (pos / 16).min(9) as u32
}

/// 单个检索词在某条目上的得分；未命中返回 `None`。
fn score_term(entry: &SearchEntry, term: &str) -> Option<u32> {
  if let Some(pos) = entry.title_lower.find(term) {
    // 标题完全等于查询词 > 标题前缀命中 > 标题中间命中
    let base = if entry.title_lower.len() == term.len() {
      100
    } else if pos == 0 {
      60
    } else {
      40
    };
    return Some(base + position_bonus(char_offset(&entry.title_lower, pos)));
  }
  if let Some(pos) = entry.text_lower.find(term) {
    let base = if pos == 0 { 20 } else { 10 };
    return Some(base + position_bonus(char_offset(&entry.text_lower, pos)));
  }
  None
}

/// 领域前缀过滤：不同 key 之间是 AND（`band:20m mode:ft8` 要两者都满足），同一个 key
/// 出现多次是 OR（`band:20m band:40m` 是「20m 或 40m」）—— 一个条目通常只带某个 key 的
/// 一个标签，同 key 按 AND 会让这种查询恒为空。无标签的条目在带前缀的查询里被过滤掉。
/// `key:value` 里 key 侧的借用视图；没有冒号（不应出现）按空串。
///
/// 做成模块级函数而不是闭包：返回 `&str` 借用输入，闭包无法表达
/// `for<'a> fn(&'a str) -> &'a str` 的高阶生命周期。
fn filter_key(f: &str) -> &str {
  match f.split_once(':') {
    Some((k, _)) => k,
    None => "",
  }
}

fn tags_match(entry: &SearchEntry, filters: &[String]) -> bool {
  // 全程借用不分配：数千条目 × 多个 filter 的查询里这条会被调用数万次，
  // 每条目都建一组 `String` 是无谓的小分配。
  let mut keys: Vec<&str> = filters.iter().map(|f| filter_key(f)).collect();
  keys.sort_unstable();
  keys.dedup();
  keys.iter().all(|key| {
    filters
      .iter()
      .filter(|f| filter_key(f) == *key)
      .any(|f| entry.tags.iter().any(|t| t == f))
  })
}

/// 某条目对整条查询的得分；有任一词未命中则整条不算命中（AND 语义）。
fn score_entry(entry: &SearchEntry, terms: &[String], filters: &[String]) -> Option<u32> {
  if !tags_match(entry, filters) {
    return None;
  }
  let mut total = 0u32;
  for term in terms {
    total += score_term(entry, term)?;
  }
  Some(total)
}

/// 按关键词搜索并按相关度排序，最多返回 `limit` 条（忽略大小写）。
///
/// 排序依据：标题命中优于正文命中；完全相等优于前缀命中优于中间命中；命中位置越靠前
/// 得分越高。`limit` 为 0 时表示不限制。
#[must_use]
pub fn search_ranked(query: &str, limit: usize) -> Vec<SearchHit> {
  let (terms, filters) = parse_query(query);
  if terms.is_empty() && filters.is_empty() {
    return Vec::new();
  }
  let filter_tags: Vec<String> = filters.iter().map(|(k, v)| format!("{k}:{v}")).collect();
  let mut hits: Vec<SearchHit> = knowledge_index()
    .iter()
    .filter_map(|entry| {
      score_entry(entry, &terms, &filter_tags).map(|score| SearchHit { entry, score })
    })
    .collect();
  // 稳定排序：同分时保持条目在索引中的原始顺序。
  hits.sort_by_key(|h| std::cmp::Reverse(h.score));
  if limit > 0 && hits.len() > limit {
    hits.truncate(limit);
  }
  hits
}

/// 按关键词搜索，返回匹配条目（按相关度排序，最多 [`MAX_RESULTS`] 条）。
#[must_use]
pub fn search(query: &str) -> Vec<&'static SearchEntry> {
  search_ranked(query, MAX_RESULTS)
    .into_iter()
    .map(|h| h.entry)
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn qso(call: &str, freq: &str, mode: &str) -> crate::logbook::LogEntry {
    crate::logbook::LogEntry {
      callsign: call.into(),
      freq: freq.into(),
      mode: mode.into(),
      date: "2026-10-01".into(),
      time: "12:00".into(),
      ..Default::default()
    }
  }

  fn hits(e: &crate::logbook::LogEntry, q: &str) -> bool {
    let (terms, filters) = parse_query(q);
    qso_matches(e, &terms, &filters)
  }

  #[test]
  fn qso_dxcc_prefix_uses_the_entity_not_the_literal_prefix() {
    let ja = qso("7J1ABC", "14.074", "FT8");
    // 7J 是日本呼号，实体主前缀是 JA：字面前缀比较会漏掉它。
    assert!(hits(&ja, "dxcc:ja"));
    assert!(!hits(&ja, "dxcc:by"));
    assert!(hits(&qso("BG4XXX", "7.050", "SSB"), "dxcc:by"));
    // 记录里的 DXCC 编号优先于呼号推断（318 = 中国）。
    let mut tagged = qso("XX9ZZZ", "14.074", "FT8");
    tagged.dxcc = "318".into();
    assert!(hits(&tagged, "dxcc:by"));
    // 认不出实体的呼号在带 dxcc: 的查询里不命中。
    assert!(!hits(&qso("ZZZ", "14.074", "FT8"), "dxcc:by"));
  }

  #[test]
  fn qso_filters_follow_the_static_index_semantics() {
    let e = qso("JA1AAA", "14.074", "FT8");
    // 波段按频率推算（BAND 字段为空也能命中）。
    assert!(hits(&e, "band:20m"));
    assert!(hits(&e, "mode:ft8 band:20m dxcc:ja"));
    // 同一 key 多次出现 = OR；旧实现按 AND 处理，这条查询对日志恒为空。
    assert!(hits(&e, "band:40m band:20m"));
    // 不同 key 之间 = AND。
    assert!(!hits(&e, "band:20m mode:cw"));
    // 自由词 AND，并可与前缀组合。
    assert!(hits(&e, "ja1 dxcc:ja"));
    assert!(!hits(&e, "bg4 dxcc:ja"));
    // 静态索引与日志用同一份口径：dxcc:ja 在两边都有命中。
    assert!(!search_ranked("dxcc:ja", 0).is_empty());
  }

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
    // 纯分隔符也应视为空查询
    assert!(search("  ").is_empty());
  }

  #[test]
  fn ranks_title_matches_above_text_matches() {
    let hits = search_ranked("驻波比", 0);
    assert!(!hits.is_empty());
    // 标题里直接就叫「驻波比」的条目应排在只在正文里提到它的前面。
    let first_is_title = hits[0].entry.title_lower.contains("驻波比");
    assert!(first_is_title, "首条标题 {}", hits[0].entry.title);
    // 得分单调不增
    for w in hits.windows(2) {
      assert!(w[0].score >= w[1].score, "{} < {}", w[0].score, w[1].score);
    }
  }

  #[test]
  fn respects_result_limit() {
    let unlimited = search_ranked("天线", 0).len();
    assert!(unlimited > 10, "结果应足够多，实际 {unlimited}");
    assert_eq!(search_ranked("天线", 5).len(), 5);
    assert_eq!(search("天线").len(), MAX_RESULTS.min(unlimited));
  }

  #[test]
  fn multi_term_query_is_conjunctive() {
    // 两个词都出现的条目，得分必然高于只出现一个词的 —— 后者直接不算命中。
    let both = search_ranked("天线 驻波比", 0);
    for h in &both {
      assert!(h.entry.title_lower.contains("天线") || h.entry.text_lower.contains("天线"));
      assert!(h.entry.title_lower.contains("驻波比") || h.entry.text_lower.contains("驻波比"));
    }
    // 不存在的词应让整条查询无结果
    assert!(search("驻波比 这个词绝对不存在于知识库").is_empty());
  }

  #[test]
  fn exact_title_match_scores_highest() {
    let hits = search_ranked("驻波比", 0);
    let exact = hits.iter().find(|h| h.entry.title == "驻波比");
    if let Some(e) = exact {
      assert!(e.score >= 100, "完全匹配得分 {}", e.score);
    }
  }

  #[test]
  fn radio_law_articles_are_found_by_chinese_and_arabic_numbers() {
    // 法律原文写「第三十七条」，用户这么搜；站内索引标题写「第37条」，两种都要命中同一条。
    for query in ["第三十七条", "办法 第37条"] {
      let hit = search_ranked(query, 0)
        .into_iter()
        .find(|h| h.entry.href == "/radio-law" && h.entry.title.ends_with("管理办法》 第37条"));
      assert!(hit.is_some(), "「{query}」应命中办法第 37 条");
    }
    // 条例与办法都有第 37 条：中文数字查询两部都会命中，不会被其中一部吞掉。
    let both = search_ranked("第三十七条", 0)
      .into_iter()
      .filter(|h| h.entry.href == "/radio-law" && h.entry.title.ends_with("第37条"))
      .count();
    assert_eq!(both, 2);
  }

  #[test]
  fn field_prefixes_filter_by_tags() {
    // dxcc: 前缀只命中带该标签的条目（DXCC 实体）。
    let hits = search_ranked("dxcc:by", 0);
    assert!(!hits.is_empty(), "dxcc:by 应有命中");
    assert!(
      hits
        .iter()
        .all(|h| h.entry.tags.iter().any(|t| t == "dxcc:by"))
    );
    // 前缀 + 自由词组合（AND）。
    assert!(!search_ranked("dxcc:by 实体", 0).is_empty());
    // band:20m 命中业余波段规划条目（主条目 + 子段都带该标签）。
    let by_abbr = search_ranked("band:20m", 0);
    assert!(!by_abbr.is_empty());
    assert!(
      by_abbr
        .iter()
        .all(|h| h.entry.tags.iter().any(|t| t == "band:20m"))
    );
    // ITU 频段缩写也支持：band:hf 命中「短波」。
    let itu = search_ranked("band:hf", 0);
    assert!(
      itu
        .iter()
        .any(|h| h.entry.href == "/bands" && h.entry.title.contains("短波"))
    );
    // mode: 前缀。
    let ft8 = search_ranked("mode:ft8", 0);
    assert!(!ft8.is_empty());
    assert!(
      ft8
        .iter()
        .all(|h| h.entry.tags.iter().any(|t| t == "mode:ft8"))
    );
    // 同 key 多次出现是 OR：20m 或 40m 的并集，每条都至少带其一。
    let either = search_ranked("band:20m band:40m", 0);
    let only_20 = search_ranked("band:20m", 0).len();
    let only_40 = search_ranked("band:40m", 0).len();
    assert!(only_20 > 0 && only_40 > 0);
    assert_eq!(
      either.len(),
      only_20 + only_40,
      "并集 = 两者之和（标签互斥）"
    );
    // 不同 key 之间仍是 AND：同一条目不会既是 dxcc:by 又是 band:20m。
    assert!(search_ranked("dxcc:by band:20m", 0).is_empty());
    // 未知前缀按自由词处理：不该 panic，也不该吞掉整条查询。
    let _ = search_ranked("1:1", 0);
    // 大小写不敏感。
    assert!(!search_ranked("DXCC:BY", 0).is_empty());
  }

  #[test]
  fn pages_and_tools_are_searchable() {
    assert!(
      search_ranked("RTTY 解码器", 0)
        .iter()
        .any(|h| h.entry.href == "/rtty-decode"),
      "工具页应可被搜索命中"
    );
    assert!(
      search_ranked("波段表", 0)
        .iter()
        .any(|h| h.entry.href == "/bands"),
      "知识页应可被搜索命中"
    );
  }

  #[test]
  fn new_pages_are_indexed_exactly_once() {
    // 这几页的条目曾误嵌在「波段规划」的 for 循环里，被重复写入（每个波段一份）：
    // 搜索结果里同一条会出现十几遍。按 (标题, 内容) 钉住不重复。
    for href in [
      "/measurement-lab",
      "/diy-projects",
      "/radio-law",
      "/rig-reviews",
    ] {
      let mut seen = std::collections::HashSet::new();
      for e in knowledge_index().iter().filter(|e| e.href == href) {
        assert!(
          seen.insert((e.title.as_str(), e.text.as_str())),
          "{href} 的索引条目重复：{}",
          e.title
        );
      }
      assert!(!seen.is_empty(), "{href} 没有进搜索索引");
    }
  }

  #[test]
  fn formulas_are_searchable_by_name_expression_and_description() {
    use crate::formulas::FORMULA_GROUPS;
    // 每条公式都至少能被自己的名字搜到，并且落在 /formulas。
    for g in FORMULA_GROUPS {
      for f in g.items {
        let hit = search(f.name)
          .into_iter()
          .any(|e| e.href == "/formulas" && e.title == f.name);
        assert!(hit, "搜「{}」应命中公式速查", f.name);
      }
    }
    // 按说明里的词、按表达式里的符号也能搜到。
    assert!(search("电压").iter().any(|e| e.href == "/formulas"));
    assert!(search("I²R").iter().any(|e| e.href == "/formulas"));
  }
}
