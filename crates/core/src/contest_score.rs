//! 竞赛记分：常见国际竞赛的分数计算（供记分器使用）。

/// 计算竞赛总分：总分 = QSO 点数 × 乘数（multiplier）。
#[must_use]
pub fn score(qso_points: u32, multiplier: u32) -> u32 {
  qso_points * multiplier
}

/// CQ WPX：乘数为不同呼号前缀数，QSO 按 1 分/个（不同大陆 3 分，此处简化按 1 分计）。
#[must_use]
pub fn wpx_score(qsos: u32, prefixes: u32) -> u32 {
  score(qsos, prefixes)
}

/// CQ WW：乘数 = CQ 分区数 + 国家/实体数。
#[must_use]
pub fn cqww_score(qsos: u32, zones: u32, entities: u32) -> u32 {
  score(qsos, zones + entities)
}

/// ARRL DX：仅与非美/加实体通联计分，每 QSO 3 分，乘数为实体数。
#[must_use]
pub fn arrl_dx_score(qsos: u32, entities: u32) -> u32 {
  score(qsos * 3, entities)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn score_multiplies() {
    assert_eq!(score(10, 5), 50);
    assert_eq!(wpx_score(100, 30), 3000);
    assert_eq!(cqww_score(100, 20, 30), 5000);
    assert_eq!(arrl_dx_score(100, 50), 15000);
  }
}
