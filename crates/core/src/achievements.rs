//! 成就系统：由本地学习数据判定已解锁的里程碑成就。

/// 一项成就。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Achievement {
  pub id: &'static str,
  pub name: &'static str,
  pub desc: &'static str,
  pub icon: &'static str,
}

/// 全部成就。
pub const ACHIEVEMENTS: &[Achievement] = &[
  Achievement {
    id: "first-pass",
    name: "首考合格",
    desc: "第一次模拟考试合格",
    icon: "🎖️",
  },
  Achievement {
    id: "streak-7",
    name: "连续打卡 7 天",
    desc: "连续 7 天有作答",
    icon: "🔥",
  },
  Achievement {
    id: "streak-30",
    name: "连续打卡 30 天",
    desc: "连续 30 天有作答",
    icon: "🏆",
  },
  Achievement {
    id: "cover-50",
    name: "题库过半",
    desc: "某题库覆盖率超过 50%",
    icon: "📖",
  },
  Achievement {
    id: "cover-100",
    name: "题库通关",
    desc: "某题库覆盖率 100%",
    icon: "🎓",
  },
  Achievement {
    id: "challenge-10",
    name: "挑战达人",
    desc: "完成 10 次每日挑战",
    icon: "⚡",
  },
  Achievement {
    id: "challenge-30",
    name: "挑战一整月",
    desc: "完成 30 次每日挑战",
    icon: "📅",
  },
  Achievement {
    id: "correct-100",
    name: "百题斩",
    desc: "累计答对 100 题",
    icon: "💯",
  },
  Achievement {
    id: "correct-500",
    name: "五百题斩",
    desc: "累计答对 500 题",
    icon: "🌟",
  },
  Achievement {
    id: "correct-2000",
    name: "两千题斩",
    desc: "累计答对 2000 题",
    icon: "🚀",
  },
  Achievement {
    id: "log-1",
    name: "第一条通联",
    desc: "记录第一条通联日志",
    icon: "📓",
  },
  Achievement {
    id: "log-100",
    name: "百次通联",
    desc: "记录 100 条通联日志",
    icon: "📚",
  },
  Achievement {
    id: "dxcc-1",
    name: "首个 DXCC",
    desc: "通联第一个 DXCC 实体",
    icon: "🌍",
  },
  Achievement {
    id: "dxcc-50",
    name: "DX 半程",
    desc: "通联 50 个 DXCC 实体",
    icon: "🌏",
  },
  Achievement {
    id: "dxcc-100",
    name: "DXCC 世纪俱乐部",
    desc: "通联 100 个 DXCC 实体",
    icon: "👑",
  },
  Achievement {
    id: "bookmark-10",
    name: "收藏达人",
    desc: "收藏 10 道题",
    icon: "🔖",
  },
  Achievement {
    id: "bookmark-50",
    name: "收藏大师",
    desc: "收藏 50 道题",
    icon: "🏷️",
  },
  Achievement {
    id: "exam-perfect",
    name: "满分通关",
    desc: "一次模拟考试得满分",
    icon: "💯",
  },
  Achievement {
    id: "log-500",
    name: "五百通联",
    desc: "记录 500 条通联日志",
    icon: "📖",
  },
  Achievement {
    id: "grid-50",
    name: "网格猎手",
    desc: "通联 50 个不同网格",
    icon: "🗺️",
  },
  Achievement {
    id: "contest-10",
    name: "竞赛初体验",
    desc: "竞赛录入 10 条通联",
    icon: "🏆",
  },
];

/// 成就判定所需的输入摘要（由前端从本地数据构造）。
#[derive(Debug, Clone, Copy, Default)]
pub struct AchStats<'a> {
  /// 累计答对数。
  pub total_correct: u32,
  /// 各题库覆盖率（0–1）。
  pub coverages: &'a [f64],
  /// 连续打卡天数。
  pub streak: usize,
  /// 是否有模拟考试合格记录。
  pub exam_passed: bool,
  /// 是否有模拟考试满分记录。
  pub exam_perfect: bool,
  /// 每日挑战完成次数。
  pub challenge_count: usize,
  /// 日志中已通联的 DXCC 实体数。
  pub dxcc_count: usize,
  /// 通联日志条数。
  pub log_count: usize,
  /// 通联日志中不重复的网格数。
  pub grid_count: usize,
  /// 通联日志中竞赛通联数。
  pub contest_count: usize,
  /// 收藏题目数。
  pub bookmarks: usize,
}

/// 判定已解锁的成就 id（按 [`ACHIEVEMENTS`] 顺序）。
#[must_use]
pub fn unlocked(s: &AchStats) -> Vec<&'static str> {
  let mut out = Vec::new();
  if s.exam_passed {
    out.push("first-pass");
  }
  if s.exam_perfect {
    out.push("exam-perfect");
  }
  if s.streak >= 7 {
    out.push("streak-7");
  }
  if s.streak >= 30 {
    out.push("streak-30");
  }
  if s.coverages.iter().any(|c| *c >= 0.5) {
    out.push("cover-50");
  }
  if s.coverages.iter().any(|c| *c >= 1.0) {
    out.push("cover-100");
  }
  if s.challenge_count >= 10 {
    out.push("challenge-10");
  }
  if s.challenge_count >= 30 {
    out.push("challenge-30");
  }
  if s.total_correct >= 100 {
    out.push("correct-100");
  }
  if s.total_correct >= 500 {
    out.push("correct-500");
  }
  if s.total_correct >= 2000 {
    out.push("correct-2000");
  }
  if s.log_count >= 1 {
    out.push("log-1");
  }
  if s.log_count >= 100 {
    out.push("log-100");
  }
  if s.log_count >= 500 {
    out.push("log-500");
  }
  if s.grid_count >= 50 {
    out.push("grid-50");
  }
  if s.contest_count >= 10 {
    out.push("contest-10");
  }
  if s.dxcc_count >= 1 {
    out.push("dxcc-1");
  }
  if s.dxcc_count >= 50 {
    out.push("dxcc-50");
  }
  if s.dxcc_count >= 100 {
    out.push("dxcc-100");
  }
  if s.bookmarks >= 10 {
    out.push("bookmark-10");
  }
  if s.bookmarks >= 50 {
    out.push("bookmark-50");
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn unlocks_by_thresholds() {
    let s = AchStats {
      total_correct: 120,
      coverages: &[0.6, 0.2],
      streak: 8,
      exam_passed: true,
      challenge_count: 3,
      ..AchStats::default()
    };
    let ids = unlocked(&s);
    assert!(ids.contains(&"first-pass"));
    assert!(ids.contains(&"streak-7"));
    assert!(!ids.contains(&"streak-30"));
    assert!(ids.contains(&"cover-50"));
    assert!(!ids.contains(&"cover-100"));
    assert!(!ids.contains(&"challenge-10"));
    assert!(ids.contains(&"correct-100"));
    assert!(!ids.contains(&"correct-500"));
    assert!(!ids.contains(&"log-1"));
    assert!(!ids.contains(&"dxcc-1"));
    assert!(!ids.contains(&"bookmark-10"));
  }

  #[test]
  fn unlocks_radio_and_log_achievements() {
    let s = AchStats {
      log_count: 120,
      dxcc_count: 100,
      bookmarks: 10,
      challenge_count: 30,
      total_correct: 2000,
      ..AchStats::default()
    };
    let ids = unlocked(&s);
    assert!(ids.contains(&"log-1"));
    assert!(ids.contains(&"log-100"));
    assert!(ids.contains(&"dxcc-1"));
    assert!(ids.contains(&"dxcc-50"));
    assert!(ids.contains(&"dxcc-100"));
    assert!(ids.contains(&"bookmark-10"));
    assert!(ids.contains(&"challenge-30"));
    assert!(ids.contains(&"correct-2000"));
  }

  #[test]
  fn achievements_are_well_formed() {
    assert!(ACHIEVEMENTS.len() >= 12);
    let mut ids = std::collections::HashSet::new();
    for a in ACHIEVEMENTS {
      assert!(ids.insert(a.id));
      assert!(!a.name.is_empty() && !a.desc.is_empty() && !a.icon.is_empty());
    }
  }
}
