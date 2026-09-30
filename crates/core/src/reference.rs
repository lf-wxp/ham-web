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
    power: "30MHz 以下 < 15W；30MHz 以上 ≤ 25W",
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

  #[test]
  fn license_and_areas_complete() {
    assert_eq!(LICENSE_CLASSES.len(), 3);
    assert_eq!(CALL_AREAS.len(), 10);
    assert_eq!(RST_SCALES.len(), 3);
    assert!(!EMISSION_TYPES.is_empty());
    assert!(!PHRASES.is_empty());
  }

  #[test]
  fn rst_levels_are_expected_ranges() {
    assert_eq!(RST_SCALES[0].levels.len(), 5); // R 1–5
    assert_eq!(RST_SCALES[1].levels.len(), 9); // S 1–9
    assert_eq!(RST_SCALES[2].levels.len(), 9); // T 1–9
  }
}
