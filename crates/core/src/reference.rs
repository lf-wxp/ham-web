//! 考试速查数据：操作证类别与权限、分区号、RST 信号报告、发射类别标识、通联英语短句。

/// 操作证类别与使用权限。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LicenseClass {
  pub class: &'static str,
  /// 频率范围。
  pub freq: &'static str,
  /// 功率上限。
  pub power: &'static str,
  /// 备注。
  pub note: &'static str,
}

/// A/B/C 类操作技术能力验证的使用权限。
pub const LICENSE_CLASSES: &[LicenseClass] = &[
  LicenseClass {
    class: "A",
    freq: "30–3000MHz（30MHz 以上）",
    power: "≤ 25W",
    note: "入门类别，覆盖 VHF/UHF 及微波频段，适合手台、车台和中继通联。",
  },
  LicenseClass {
    class: "B",
    freq: "30MHz 以下 或 30MHz 以上",
    power: "30MHz 以下 ≤ 15W（不大于 15W）；30MHz 以上 ≤ 25W",
    note: "可进行短波（HF）通联，但功率受限。",
  },
  LicenseClass {
    class: "C",
    freq: "30MHz 以下 或 30MHz 以上",
    power: "30MHz 以下 ≤ 1000W；30MHz 以上 ≤ 25W",
    note: "最高类别，短波段可使用大功率。",
  },
];

/// 业余电台分区号（呼号第三位数字）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallArea {
  pub digit: &'static str,
  pub regions: &'static str,
}

/// 0–9 分区对应的地区。
pub const CALL_AREAS: &[CallArea] = &[
  CallArea {
    digit: "1",
    regions: "北京",
  },
  CallArea {
    digit: "2",
    regions: "黑龙江 · 吉林 · 辽宁",
  },
  CallArea {
    digit: "3",
    regions: "天津 · 河北 · 内蒙古 · 山西",
  },
  CallArea {
    digit: "4",
    regions: "上海 · 山东 · 江苏",
  },
  CallArea {
    digit: "5",
    regions: "浙江 · 江西 · 福建",
  },
  CallArea {
    digit: "6",
    regions: "安徽 · 河南 · 湖北",
  },
  CallArea {
    digit: "7",
    regions: "湖南 · 广东 · 广西 · 海南",
  },
  CallArea {
    digit: "8",
    regions: "四川 · 重庆 · 贵州 · 云南",
  },
  CallArea {
    digit: "9",
    regions: "陕西 · 甘肃 · 宁夏 · 青海",
  },
  CallArea {
    digit: "0",
    regions: "新疆 · 西藏",
  },
];

/// 省级行政区与它所辖的分区号（分区号地图用）。
///
/// 只列呼号分区 1–0 覆盖的 31 个省级行政区：港澳台的呼号归入其他 DXCC 实体，
/// 不在这套分区号里。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Province {
  /// ISO 3166-2 的地名码后缀（`CN-BJ` → `BJ`），与 `public/cn-provinces.bin` 的 key 对应。
  pub code: &'static str,
  /// 简称，措辞与 [`CALL_AREAS`] 的 `regions` 一致（同一份官方分区表）。
  pub name: &'static str,
  /// 分区号（呼号第三位数字）。
  pub area: &'static str,
}

/// 31 个省级行政区 → 分区号，按 [`CALL_AREAS`] 的分区顺序排列。
pub const CHINA_PROVINCES: &[Province] = &[
  Province {
    code: "BJ",
    name: "北京",
    area: "1",
  },
  Province {
    code: "HL",
    name: "黑龙江",
    area: "2",
  },
  Province {
    code: "JL",
    name: "吉林",
    area: "2",
  },
  Province {
    code: "LN",
    name: "辽宁",
    area: "2",
  },
  Province {
    code: "TJ",
    name: "天津",
    area: "3",
  },
  Province {
    code: "HE",
    name: "河北",
    area: "3",
  },
  Province {
    code: "NM",
    name: "内蒙古",
    area: "3",
  },
  Province {
    code: "SX",
    name: "山西",
    area: "3",
  },
  Province {
    code: "SH",
    name: "上海",
    area: "4",
  },
  Province {
    code: "SD",
    name: "山东",
    area: "4",
  },
  Province {
    code: "JS",
    name: "江苏",
    area: "4",
  },
  Province {
    code: "ZJ",
    name: "浙江",
    area: "5",
  },
  Province {
    code: "JX",
    name: "江西",
    area: "5",
  },
  Province {
    code: "FJ",
    name: "福建",
    area: "5",
  },
  Province {
    code: "AH",
    name: "安徽",
    area: "6",
  },
  Province {
    code: "HA",
    name: "河南",
    area: "6",
  },
  Province {
    code: "HB",
    name: "湖北",
    area: "6",
  },
  Province {
    code: "HN",
    name: "湖南",
    area: "7",
  },
  Province {
    code: "GD",
    name: "广东",
    area: "7",
  },
  Province {
    code: "GX",
    name: "广西",
    area: "7",
  },
  Province {
    code: "HI",
    name: "海南",
    area: "7",
  },
  Province {
    code: "SC",
    name: "四川",
    area: "8",
  },
  Province {
    code: "CQ",
    name: "重庆",
    area: "8",
  },
  Province {
    code: "GZ",
    name: "贵州",
    area: "8",
  },
  Province {
    code: "YN",
    name: "云南",
    area: "8",
  },
  Province {
    code: "SN",
    name: "陕西",
    area: "9",
  },
  Province {
    code: "GS",
    name: "甘肃",
    area: "9",
  },
  Province {
    code: "NX",
    name: "宁夏",
    area: "9",
  },
  Province {
    code: "QH",
    name: "青海",
    area: "9",
  },
  Province {
    code: "XJ",
    name: "新疆",
    area: "0",
  },
  Province {
    code: "XZ",
    name: "西藏",
    area: "0",
  },
];

/// 按 ISO 3166-2 地名码后缀（如 `BJ`）查省份。
#[must_use]
pub fn province_by_code(code: &str) -> Option<&'static Province> {
  CHINA_PROVINCES.iter().find(|p| p.code == code)
}

/// 分区号 → 地图配色（HSL 字符串，SVG `fill` 可直接用）。
///
/// 色相是**固定的十档**而不是按分区号均分：1（红）与 0 在均分方案里正好隔着色相环的
/// 接缝，只差 36°，肉眼容易混；这里把 0 推到品红端（310°），与红、紫都拉开。
/// 相邻分区再交替明度（按分区在 [`CALL_AREAS`] 里的**序号**奇偶，不是数字本身 ——
/// 「0」排在最后，序号是 9），同一分区号在任何页面都得到同一颜色。
///
/// 不在表里的分区号退回第一档（红）；调用方应先确认分区号合法（地图组件只画
/// [`province_by_code`] 查得到的省）。
#[must_use]
pub fn call_area_color(area: &str) -> String {
  /// 按 [`CALL_AREAS`] 顺序排列的色相。
  const HUES: [f64; 10] = [
    0.0, 40.0, 70.0, 120.0, 160.0, 190.0, 215.0, 245.0, 275.0, 310.0,
  ];
  let idx = CALL_AREAS.iter().position(|a| a.digit == area).unwrap_or(0);
  let hue = HUES[idx.min(HUES.len() - 1)];
  crate::zone::area_hsl(hue, idx.is_multiple_of(2))
}

/// RST 信号报告的一组刻度。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RstScale {
  /// 标识（R / S / T）。
  pub key: &'static str,
  /// 名称。
  pub name: &'static str,
  /// 适用说明。
  pub note: &'static str,
  /// (数值, 含义) 列表。
  pub levels: &'static [(&'static str, &'static str)],
}

/// RST 三组刻度。
pub const RST_SCALES: &[RstScale] = &[
  RstScale {
    key: "R",
    name: "可懂度 Readability",
    note: "话音与 CW 通用",
    levels: &[
      ("1", "无法理解"),
      ("2", "偶尔能听懂个别词"),
      ("3", "相当困难，只能听懂部分"),
      ("4", "基本无困难，可以听懂"),
      ("5", "完全可懂"),
    ],
  },
  RstScale {
    key: "S",
    name: "强度 Strength",
    note: "话音与 CW 通用",
    levels: &[
      ("1", "极微弱，几乎听不到"),
      ("2", "很弱"),
      ("3", "弱"),
      ("4", "较弱"),
      ("5", "尚可"),
      ("6", "好"),
      ("7", "较强"),
      ("8", "强"),
      ("9", "极强"),
    ],
  },
  RstScale {
    key: "T",
    name: "音调 Tone",
    note: "仅用于 CW",
    levels: &[
      ("1", "极粗糙的交流声"),
      ("2", "很粗糙的交流声"),
      ("3", "粗糙、低沉的交流声"),
      ("4", "带明显交流声的音调"),
      ("5", "略有交流声"),
      ("6", "平滑音调，略有波动"),
      ("7", "接近纯音，略有交流纹波"),
      ("8", "接近纯音，略有调制痕迹"),
      ("9", "纯音"),
    ],
  },
];

/// 发射类别标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmissionType {
  pub code: &'static str,
  pub name: &'static str,
  pub desc: &'static str,
}

/// 常见发射类别。
pub const EMISSION_TYPES: &[EmissionType] = &[
  EmissionType {
    code: "A1A",
    name: "等幅电报",
    desc: "靠人耳接收的莫尔斯电报（CW）",
  },
  EmissionType {
    code: "J3E",
    name: "单边带话",
    desc: "抑制载波的单边带调幅话音（SSB 话）",
  },
  EmissionType {
    code: "A3E",
    name: "双边带调幅话",
    desc: "普通的 AM 调幅话",
  },
  EmissionType {
    code: "F3E",
    name: "调频话",
    desc: "VHF/UHF 手台与中继台最常用（FM 话）",
  },
  EmissionType {
    code: "G3E",
    name: "调相话",
    desc: "许多所谓调频电台实际采用的相位调制",
  },
  EmissionType {
    code: "F1B",
    name: "频移键控电报",
    desc: "由机器自动接收，如 RTTY",
  },
  EmissionType {
    code: "A2A",
    name: "音频调幅电报",
    desc: "如测向信标发射的音频摩尔斯标识",
  },
  EmissionType {
    code: "A1B",
    name: "等幅电报（自动接收）",
    desc: "由机器（译码器 / 计算机）自动接收的莫尔斯电报",
  },
  EmissionType {
    code: "J2B",
    name: "单边带电报（自动接收）",
    desc: "抑制载波单边带，用于 PSK31 等数据",
  },
  EmissionType {
    code: "R3E",
    name: "单边带话（减幅载波）",
    desc: "保留部分载波的单边带电话",
  },
  EmissionType {
    code: "A3C",
    name: "双边带传真",
    desc: "双边带调幅传真（静止图像），第三符号 C = 传真。",
  },
  EmissionType {
    code: "C3F",
    name: "残留边带电视",
    desc: "残留边带调幅，模拟电视广播的标准方式（VSB）。",
  },
  EmissionType {
    code: "G1D",
    name: "调相数据",
    desc: "相位调制的单信道数字数据（PSK 类数据模式），第三符号 D = 数据。",
  },
  EmissionType {
    code: "H3E",
    name: "单边带全载波话",
    desc: "带全载波的单边带，便于普通 AM 接收机解调。",
  },
  EmissionType {
    code: "J2D",
    name: "单边带数据",
    desc: "抑制载波单边带数据（如 PACTOR）。",
  },
  EmissionType {
    code: "J3F",
    name: "单边带电视",
    desc: "单边带电视（SSTV 常用）。",
  },
  EmissionType {
    code: "J3C",
    name: "单边带传真",
    desc: "用单边带方式传送的传真图像",
  },
  EmissionType {
    code: "F1D",
    name: "频移键控数据",
    desc: "直接 FSK 传数据，如 9600 波特分组无线电",
  },
  EmissionType {
    code: "F2D",
    name: "音频调频数据",
    desc: "音频副载波调频传数据，如 1200 波特分组无线电",
  },
  EmissionType {
    code: "F3F",
    name: "调频电视",
    desc: "用调频方式传送图像（SSTV / 业余电视）",
  },
];

/// ITU 发射类别标识的构成：必要带宽 + 发射类别（+ 可选附加特性）。
pub const EMISSION_DESIGNATION: &[(&str, &str, &str)] = &[
  (
    "必要带宽（4 字符）",
    "3 位数字 + 1 位单位字母（H=Hz、k=kHz、M=MHz、G=GHz），小数点用单位字母代替",
    "2K70 = 2.7kHz；3K00 = 3.00kHz；500H = 500Hz",
  ),
  (
    "第 1 位 · 主载波调制",
    "A=双边带、J=单边带抑制载波、R=单边带减幅载波、F=调频、G=调相、D=幅角混合、K/L/M/P=脉冲",
    "J3E 的首位 J 表示单边带抑制载波",
  ),
  (
    "第 2 位 · 调制信号性质",
    "0=无调制、1=单路数字（无副载波）、2=单路数字（有副载波）、3=单路模拟、7/8=多路数字/模拟、9=复合",
    "J3E 的 3 表示单路模拟信号",
  ),
  (
    "第 3 位 · 信息类型",
    "A=人工电报、B=自动电报、C=传真、D=数据/遥测、E=电话、F=电视、W=组合、X=其他",
    "J3E 的 E 表示电话",
  ),
  (
    "附加特性（可选）",
    "描述复用、前向纠错、加密等附加信息",
    "J3EJN 表示带前向纠错的单边带数据",
  ),
];

/// 主要知识的来源与时效（便于核对更新）。
///
/// 时效性内容（法规版本、卫星状态、频率惯例）会随时间变化，请以来源方最新公告为准。
pub const KNOWLEDGE_SOURCES: &[(&str, &str, &str)] = &[
  (
    "法规 / 操作证",
    "《业余无线电台管理办法》（工信部令第 67 号，2024-03-01 施行）",
    "以主管部门最新公告为准",
  ),
  (
    "频率划分 / 脚注",
    "《中华人民共和国无线电频率划分规定》（2023-07-01 施行）",
    "含 ITU《无线电规则》编号脚注（如 5.282）",
  ),
  (
    "功率限值",
    "同上，第二十五条 / 第三十条",
    "B 类 30MHz 以下不大于 15W",
  ),
  (
    "射频暴露",
    "FCC 47 CFR §1.1310（MPE 限值曲线）",
    "非受控 1.34MHz 拐点、180/f²",
  ),
  ("静噪音", "EIA/TIA-603 CTCSS 标准音表", "50 个标准音"),
  (
    "数字模式",
    "WSJT-X（FT8 / FT4 / JS8 / Q65）、WSPR、AX.25 / APRS 规范",
    "以各项目最新文档为准",
  ),
  (
    "卫星状态",
    "NOAA OSPO 退役公告",
    "NOAA-18 于 2025-06-06 退役",
  ),
  (
    "天线工程",
    "ARRL Antenna Book、NEC 建模",
    "增益 / 前后比为典型量级",
  ),
];

/// 通联英语短句。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Phrase {
  pub en: &'static str,
  pub zh: &'static str,
  /// 使用场景。
  pub usage: &'static str,
}

/// 常见通联英语短句。
pub const PHRASES: &[Phrase] = &[
  Phrase {
    en: "CQ CQ CQ, this is BG4XXX calling CQ and standing by.",
    zh: "CQ CQ CQ，这里是 BG4XXX 普遍呼叫，并守听。",
    usage: "发起普遍呼叫",
  },
  Phrase {
    en: "Is the frequency in use?",
    zh: "这个频率正在被使用吗？",
    usage: "询问频率是否占用",
  },
  Phrase {
    en: "You are 59 (five nine).",
    zh: "你的信号是 59（五九）。",
    usage: "给出信号报告",
  },
  Phrase {
    en: "Please say again (repeat).",
    zh: "请再说一遍。",
    usage: "请求重复",
  },
  Phrase {
    en: "Roger / QSL, copy that.",
    zh: "收到，已抄收。",
    usage: "确认抄收",
  },
  Phrase {
    en: "This is BG4XXX, over.",
    zh: "这里是 BG4XXX，请讲。",
    usage: "结束本次发言",
  },
  Phrase {
    en: "73, thanks for the QSO, bye bye.",
    zh: "73，感谢这次通联，再见。",
    usage: "通联结束",
  },
];

#[cfg(test)]
mod tests {
  use super::*;

  /// 某个分区号包含的省份（顺序与 [`CALL_AREAS`] 的地区文字一致）。只有对齐测试用，不进公开 API。
  fn provinces_in_area(area: &str) -> Vec<&'static Province> {
    CHINA_PROVINCES.iter().filter(|p| p.area == area).collect()
  }

  #[test]
  fn license_and_areas_complete() {
    assert_eq!(LICENSE_CLASSES.len(), 3);
    assert_eq!(CALL_AREAS.len(), 10);
    assert_eq!(RST_SCALES.len(), 3);
    assert!(EMISSION_TYPES.len() >= 12);
    assert!(EMISSION_DESIGNATION.len() >= 4);
    assert!(EMISSION_TYPES.len() >= 15, "发射类别完整表至少 15 项");
    assert!(!PHRASES.is_empty());
  }

  /// 省份表必须与分区表逐字对齐：`CALL_AREAS` 的「黑龙江 · 吉林 · 辽宁」就是 2 区的三个省。
  /// 两处各写一份是会漂的，这条把它们钉死。
  #[test]
  fn provinces_match_the_call_area_table() {
    assert_eq!(CHINA_PROVINCES.len(), 31);
    for area in CALL_AREAS {
      let expected: Vec<&str> = area.regions.split(" · ").collect();
      let got: Vec<&str> = provinces_in_area(area.digit)
        .iter()
        .map(|p| p.name)
        .collect();
      assert_eq!(got, expected, "{} 区的省份与分区表不一致", area.digit);
    }
    // 每个省份的分区号都得是分区表里有的（写错数字时不会被 split 对比漏掉）。
    for p in CHINA_PROVINCES {
      assert!(
        CALL_AREAS.iter().any(|a| a.digit == p.area),
        "{} 的分区号 {} 不在分区表里",
        p.name,
        p.area
      );
    }
  }

  #[test]
  fn province_codes_are_unique_and_resolvable() {
    let mut codes: Vec<&str> = CHINA_PROVINCES.iter().map(|p| p.code).collect();
    let total = codes.len();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), total, "ISO 地名码重复");
    for p in CHINA_PROVINCES {
      assert_eq!(province_by_code(p.code).map(|x| x.name), Some(p.name));
    }
    assert!(province_by_code("TW").is_none(), "港澳台不在呼号分区表里");
  }

  #[test]
  fn call_area_colours_are_stable_and_distinct() {
    let colors: Vec<String> = CALL_AREAS
      .iter()
      .map(|a| call_area_color(a.digit))
      .collect();
    assert_eq!(colors[0], call_area_color("1"));
    let unique: std::collections::BTreeSet<&String> = colors.iter().collect();
    assert_eq!(unique.len(), colors.len(), "十个分区号必须有十种颜色");
  }

  /// 只比「互不相同」看不出色相表被改、明度奇偶口径被换：把十档的精确输出钉死。
  /// 明度按分区在表里的**序号**奇偶交替（「0」序号是 9，所以是较暗的 45%），不是数字本身。
  #[test]
  fn call_area_colours_match_the_documented_scheme() {
    let expected = [
      ("1", "hsl(0 68% 62%)"),
      ("2", "hsl(40 68% 45%)"),
      ("3", "hsl(70 68% 62%)"),
      ("4", "hsl(120 68% 45%)"),
      ("5", "hsl(160 68% 62%)"),
      ("6", "hsl(190 68% 45%)"),
      ("7", "hsl(215 68% 62%)"),
      ("8", "hsl(245 68% 45%)"),
      ("9", "hsl(275 68% 62%)"),
      ("0", "hsl(310 68% 45%)"),
    ];
    for (digit, color) in expected {
      assert_eq!(call_area_color(digit), color, "{digit} 区");
    }
    // 不在表里的分区号退回第一档（调用方应先确认分区号合法）。
    assert_eq!(call_area_color("x"), call_area_color("1"));
    assert_eq!(call_area_color(""), call_area_color("1"));
  }

  #[test]
  fn rst_levels_are_expected_ranges() {
    assert_eq!(RST_SCALES[0].levels.len(), 5); // R 1–5
    assert_eq!(RST_SCALES[1].levels.len(), 9); // S 1–9
    assert_eq!(RST_SCALES[2].levels.len(), 9); // T 1–9
  }

  #[test]
  fn emission_type_names_follow_the_third_symbol() {
    // 第三字符定名：C = 传真（facsimile）、F = 电视（television）——J3F 常被误写成
    // 「单边带传真」与 J3C 重名，这里钉住两者各自的名字。
    let by_code = |code: &str| EMISSION_TYPES.iter().find(|e| e.code == code).unwrap();
    assert_eq!(by_code("J3C").name, "单边带传真");
    assert_eq!(by_code("J3F").name, "单边带电视");
  }
}
