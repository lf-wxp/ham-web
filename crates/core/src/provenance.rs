//! 知识溯源：时效性数据的「依据 + 核对日期」，页面据此标注「依据 XX，YYYY-MM 核对」。
//!
//! # 只收「数据确实来自该处」的条目
//!
//! 来源没把握的模块不进这张表 —— 宁可页面不标注，也不把「我们猜的出处」当成依据。
//! 每条的依据要么是法规/标准本身，要么是数据实际转录的地方。
//!
//! # 核对日期的含义
//!
//! `checked_on` 是维护者**对照依据原文**核对该域数据的月份，是对读者的承诺：改了该域的
//! 数据就要同步改日期；没有真正核对过的域不要为了「看起来齐全」盖章。卫星状态这类易变
//! 数据即使盖了章也只代表该月的状态（`source` 里已写明会随在轨情况变化）。

/// 一条溯源信息。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Provenance {
  /// 依据（法规名 / 标准号 / 数据实际来源）。
  pub source: &'static str,
  /// 核对日期（YYYY-MM）。
  pub checked_on: &'static str,
}

/// 各知识域的溯源表：`(域 key, 溯源)`。
pub const PROVENANCES: &[(&str, Provenance)] = &[
  (
    "regulations",
    Provenance {
      source: "《中华人民共和国无线电管理条例》（国务院、中央军委令第672号）与《业余无线电台管理办法》（工信部令第67号）",
      checked_on: "2026-10",
    },
  ),
  (
    "license-classes",
    Provenance {
      source: "《业余无线电台管理办法》第三十条及附件（A/B/C 类操作技术能力对应频段与功率）",
      checked_on: "2026-10",
    },
  ),
  (
    "bands",
    Provenance {
      source: "《中华人民共和国无线电频率划分规定》",
      checked_on: "2026-10",
    },
  ),
  (
    "bandplan",
    Provenance {
      source: "IARU 第三区波段计划（含各区域差异说明）",
      checked_on: "2026-10",
    },
  ),
  (
    "satellites",
    Provenance {
      source: "AMSAT 与各卫星协会公布的最新状态（卫星状态会随在轨情况变化）",
      checked_on: "2026-10",
    },
  ),
  (
    "rr",
    Provenance {
      source: "ITU《无线电规则》（Radio Regulations）",
      checked_on: "2026-10",
    },
  ),
  (
    "radio-law",
    Provenance {
      source: "官方公报发布文本（见条文页出处链接）",
      checked_on: "2026-10",
    },
  ),
  (
    "exam-comparison",
    Provenance {
      source: "各官方考纲与考试制度：中国《业余无线电台管理办法》权限口径、FCC / NCVEC 考试制度、RSGB 官方考纲（2024 v1.6b）",
      checked_on: "2026-10",
    },
  ),
];

/// 按域取溯源信息。
#[must_use]
pub fn for_domain(key: &str) -> Option<&'static Provenance> {
  PROVENANCES.iter().find(|(k, _)| *k == key).map(|(_, p)| p)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn every_entry_is_complete_and_keys_are_unique() {
    for (key, p) in PROVENANCES {
      assert!(!key.is_empty() && !p.source.is_empty(), "{key}");
      assert!(p.checked_on.len() == 7, "{key} 的核对日期应为 YYYY-MM");
      assert!(p.checked_on.starts_with("20"), "{key}");
      assert_eq!(for_domain(key), Some(p), "{key} 应该查得到");
    }
    let mut keys: Vec<&str> = PROVENANCES.iter().map(|(k, _)| *k).collect();
    let n = keys.len();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), n, "域 key 不能重复");
    assert!(for_domain("不存在的域").is_none());
  }

  #[test]
  fn the_time_sensitive_domains_are_covered() {
    // 时效性最敏感的三块（法规版本 / 卫星状态 / 频率惯例）必须都在表里。
    for key in ["regulations", "satellites", "bands", "bandplan"] {
      assert!(for_domain(key).is_some(), "{key} 缺溯源");
    }
  }
}
