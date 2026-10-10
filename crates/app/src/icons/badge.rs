//! 成就 id → 像素徽章精灵名。
//!
//! 放在 app 层而不是 core 的 `Achievement` 上：core 不依赖任何视图概念，
//! 而 emoji `icon` 字段仍被备份 / 分享等非视图路径使用，保持原样。
//! 新增成就时在这里补一行；漏了会回退到金色星星，不会出现空白。

/// 返回成就对应的精灵名（见 `sprite_data::sprite_by_name`）。
#[must_use]
pub fn badge_sprite(id: &str) -> &'static str {
  match id {
    "first-pass" | "exam-perfect" => "badge_medal",
    "streak-7" | "streak-30" => "badge_flame",
    "cover-50" | "cover-100" => "badge_book",
    "challenge-10" | "challenge-30" => "badge_trophy",
    "correct-100" | "correct-500" | "correct-2000" => "badge_star",
    "log-1" | "log-100" | "log-500" | "contest-10" => "badge_log",
    "dxcc-1" | "dxcc-50" | "dxcc-100" | "grid-50" => "badge_globe",
    "bookmark-10" | "bookmark-50" => "badge_antenna",
    _ => "badge_star",
  }
}

#[cfg(test)]
mod tests {
  use super::badge_sprite;
  use crate::icons::pixel::sprite_data::sprite_by_name;

  /// 每个成就都要映射到真实存在的精灵；并且不能全部落在回退值上（说明有人漏配）。
  #[test]
  fn every_achievement_has_a_real_badge() {
    let mut fallback = 0;
    for a in ham_web_core::achievements::ACHIEVEMENTS {
      let sprite = badge_sprite(a.id);
      assert!(
        sprite_by_name(sprite).is_some(),
        "成就 {} 的徽章 {sprite} 不存在",
        a.id
      );
      if sprite == "badge_star" && !a.id.starts_with("correct-") {
        fallback += 1;
      }
    }
    assert_eq!(fallback, 0, "有成就没有配置徽章，落在了回退星星上");
  }
}
