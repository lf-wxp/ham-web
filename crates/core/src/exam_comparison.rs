//! 多国题库对照：中国 A/B/C、美国 FCC、英国 Ofcom 三套认证体系的结构对照。
//!
//! # 数字的出处
//!
//! - 美国：Technician / General 各 **35 题、答对 26 题（74%）通过**，Amateur Extra 为
//!   **50 题、答对 37 题（74%）通过**（长期公开的考试制度口径，及格线均为 74%）；
//!   题库每四年更新一版，具体题数以 NCVEC 公布为准 —— 这里**不写会随题库版本变化的题数**。
//! - 英国：数字来自 RSGB 官方考纲（2024 v1.6b）的 Assessment 节，**逐字转录**：
//!   Foundation 26 题 / 60 分钟 / 73%（19 题）、Intermediate 46 题 / 90 分钟 / 61%（28 题）、
//!   Full **58 题** / 2 小时 / 60%（35 题）—— 注意 Full 是 58 题，网上常误传 62。
//! - 中国：A/B/C 类权限见 `license_classes`（以《办法》为准）；考试形式（题数 / 时长）
//!   各地机考口径不一，**不写具体数字**，以当地无线电管理机构通知为准。

/// 一套认证体系的一个等级。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level {
  /// 等级名（如 "Technician"）。
  pub name: &'static str,
  /// 通过后可用的权限摘要。
  pub privileges: &'static str,
  /// 考试形式（题数 / 时长 / 及格线；不确定就写「以官方通知为准」）。
  pub exam: &'static str,
}

/// 一套认证体系。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct System {
  /// 国家 / 地区。
  pub country: &'static str,
  /// 体系名。
  pub title: &'static str,
  /// 由哪个机构组织。
  pub authority: &'static str,
  /// 各级别。
  pub levels: &'static [Level],
  /// 官方题库 / 资料的入口（名称, URL）。
  pub source: (&'static str, &'static str),
  /// 本站收录状态说明。
  pub coverage: &'static str,
}

pub const SYSTEMS: &[System] = &[
  System {
    country: "中国",
    title: "A / B / C 类操作技术能力验证",
    authority: "无线电管理机构（考试形式以当地通知为准）",
    levels: &[
      Level {
        name: "A 类",
        privileges: "30MHz 以上 VHF/UHF（≤25W）",
        exam: "笔试机考，题数与时长以当地通知为准",
      },
      Level {
        name: "B 类",
        privileges: "A 类全部 + HF 短波（30MHz 以下 ≤15W）",
        exam: "笔试机考，题数与时长以当地通知为准",
      },
      Level {
        name: "C 类",
        privileges: "B 类频段 + 大功率（30MHz 以下 ≤1000W）",
        exam: "笔试机考，题数与时长以当地通知为准",
      },
    ],
    source: (
      "《业余无线电台管理办法》（条文见本站法规原文库）",
      "/radio-law",
    ),
    coverage: "本站已收录 A/B/C 全量题库与练习、模考（这是本站的主场）。",
  },
  System {
    country: "美国",
    title: "FCC 业余无线电执照",
    authority: "FCC 出题，NCVEC 维护题库，VE（志愿考官）执考",
    levels: &[
      Level {
        name: "Technician",
        privileges: "VHF/UHF 及部分 HF 窄段语音与数据（初级）",
        exam: "35 题，答对 26 题（74%）通过",
      },
      Level {
        name: "General",
        privileges: "大部分 HF 频段全模式",
        exam: "35 题，答对 26 题（74%）通过",
      },
      Level {
        name: "Amateur Extra",
        privileges: "全部业余频段与满功率",
        exam: "50 题，答对 37 题（74%）通过",
      },
    ],
    source: ("NCVEC 题库（ncvec.org）", "https://www.ncvec.org"),
    coverage: "本站暂未收录 FCC 题库全文 —— 数据量与翻译质量考量；此处只做结构对照。",
  },
  System {
    country: "英国",
    title: "Ofcom 业余无线电执照",
    authority: "Ofcom 发证，RSGB 组织考试",
    levels: &[
      Level {
        name: "Foundation",
        privileges: "受限功率的入门执照",
        exam: "26 题 / 60 分钟，73%（19 题）通过",
      },
      Level {
        name: "Intermediate",
        privileges: "中等功率与更多频段",
        exam: "46 题 / 90 分钟，61%（28 题）通过",
      },
      Level {
        name: "Full",
        privileges: "最高功率（400W 级）与全部业余频段",
        exam: "58 题 / 2 小时，60%（35 题）通过",
      },
    ],
    source: ("RSGB 官方考纲（2024 v1.6b）", "https://rsgb.org"),
    coverage: "本站暂未收录 Ofcom 题库全文 —— 同上，只做结构对照。",
  },
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn all_systems_are_complete() {
    assert_eq!(SYSTEMS.len(), 3);
    for s in SYSTEMS {
      assert!(!s.country.is_empty() && !s.title.is_empty() && !s.authority.is_empty());
      assert!(!s.levels.is_empty());
      for l in s.levels.iter() {
        assert!(!l.name.is_empty() && !l.privileges.is_empty() && !l.exam.is_empty());
      }
      assert!(!s.source.0.is_empty() && !s.source.1.is_empty());
    }
  }

  #[test]
  fn the_rsgb_numbers_are_transcribed_verbatim() {
    // RSGB 官方考纲（2024 v1.6b）Assessment 节的数字，逐字钉住 —— 网上常见「Full 62 题」
    // 的误传，官方是 58 题。
    let uk = &SYSTEMS[2].levels;
    assert_eq!(uk[0].exam, "26 题 / 60 分钟，73%（19 题）通过");
    assert_eq!(uk[1].exam, "46 题 / 90 分钟，61%（28 题）通过");
    assert_eq!(uk[2].exam, "58 题 / 2 小时，60%（35 题）通过");
    assert!(uk[2].exam.contains("58"), "Full 是 58 题");
  }

  #[test]
  fn fcc_exam_formats_per_level() {
    // Technician / General 各 35 题（26 题通过）；Extra 是 50 题（37 题通过）—— 别写成一样。
    let us = &SYSTEMS[1].levels;
    assert_eq!(us.len(), 3);
    assert_eq!(us[0].exam, "35 题，答对 26 题（74%）通过");
    assert_eq!(us[1].exam, "35 题，答对 26 题（74%）通过");
    assert_eq!(us[2].exam, "50 题，答对 37 题（74%）通过");
  }
}
