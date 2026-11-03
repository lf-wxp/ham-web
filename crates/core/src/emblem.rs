//! 成就徽章 / 怪物图鉴「纹章」的选图规则：精灵名 → 图标与色调，主题名 → 稳定的一组预设。
//!
//! 为什么在 core：这是纯字符串规则（关键词匹配、FNV 哈希），不依赖浏览器；放在 `crates/app`
//! 里就没有单测，改一个关键词会让某枚徽章悄悄换图标也没人发现。图标本身（`MIcon`）属于界面层，
//! 这里只给出与之一一对应的 [`Glyph`]，前端做一次映射。

/// 纹章里的图标种类（与前端线性图标集里同名的几个一一对应）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
  Flame,
  Trophy,
  Book,
  Doc,
  Map,
  Antenna,
  Signal,
  Gauge,
  Code,
  Shield,
  Star,
}

/// 纹章色调：只影响渐变与描边，不改变形状。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
  Gold,
  Fire,
  Blue,
  Violet,
  Green,
  Muted,
}

impl Tone {
  /// 写进 `data-tone` 的取值。
  #[must_use]
  pub const fn key(self) -> &'static str {
    match self {
      Self::Gold => "gold",
      Self::Fire => "fire",
      Self::Blue => "blue",
      Self::Violet => "violet",
      Self::Green => "green",
      Self::Muted => "muted",
    }
  }
}

/// 名字里是否带这个**整词**（按非字母数字切开比较）。
///
/// 短关键词（`log`）不能做子串匹配：`catalog` / `blog` 会误中它，徽章被配成「日志」图标。
fn has_word(name: &str, word: &str) -> bool {
  name
    .split(|c: char| !c.is_ascii_alphanumeric())
    .any(|part| part == word)
}

/// 精灵名 → 图标。
///
/// 长关键词用子串匹配：新增徽章时只要名字里带 `flame` / `trophy` 之类就能落到合适的图标上，
/// 漏配也只会退到星星，不会出现空白方块。图标与色调**各自**一套关键词（见 [`tone_for`]）：
/// 图标按「是什么」分，色调按「该给什么情绪」分，两者不必一一对应。
#[must_use]
pub fn glyph_for(name: &str) -> Glyph {
  let n = name.to_ascii_lowercase();
  let has = |k: &str| n.contains(k);
  if has("flame") || has("fire") {
    Glyph::Flame
  } else if has("trophy") || has("medal") || has("award") {
    Glyph::Trophy
  } else if has("book") {
    Glyph::Book
  } else if has_word(&n, "log") {
    Glyph::Doc
  } else if has("globe") || has("map") {
    Glyph::Map
  } else if has("antenna") || has("wave") {
    Glyph::Antenna
  } else if has("signal") || has("radio") {
    Glyph::Signal
  } else if has("gauge") || has("meter") {
    Glyph::Gauge
  } else if has("code") || has("binary") {
    Glyph::Code
  } else if has("lock") {
    Glyph::Shield
  } else {
    Glyph::Star
  }
}

/// 精灵名 → 色调：火焰是暖色、奖章是金色、天线 / 信号是绿色、上锁是灰。
#[must_use]
pub fn tone_for(name: &str) -> Tone {
  let n = name.to_ascii_lowercase();
  let has = |k: &str| n.contains(k);
  if has("flame") || has("fire") {
    Tone::Fire
  } else if has("trophy") || has("medal") || has("gold") {
    Tone::Gold
  } else if has("book") || has("star") {
    Tone::Violet
  } else if has("antenna") || has("wave") || has("signal") {
    Tone::Green
  } else if has("lock") {
    Tone::Muted
  } else {
    Tone::Blue
  }
}

/// 稳定哈希（FNV-1a 64 位）：短字符串上分布够均匀，也不依赖 `std` 哈希的实现细节，
/// 所以同一个名字在任何版本、任何平台上都落在同一格。
#[must_use]
pub fn stable_hash(seed: &str) -> u64 {
  let mut h: u64 = 0xcbf2_9ce4_8422_2325;
  for b in seed.as_bytes() {
    h ^= u64::from(*b);
    h = h.wrapping_mul(0x100_0000_01b3);
  }
  h
}

/// 预设组：6 组「图标 + 色调」。
const PRESETS: [(Glyph, Tone); 6] = [
  (Glyph::Antenna, Tone::Green),
  (Glyph::Signal, Tone::Blue),
  (Glyph::Map, Tone::Violet),
  (Glyph::Code, Tone::Blue),
  (Glyph::Gauge, Tone::Fire),
  (Glyph::Flame, Tone::Gold),
];

/// 按主题（可以是中文）稳定地取一组「图标 + 色调」。
///
/// [`glyph_for`] 靠 ASCII 关键词匹配精灵名，遇到中文主题（关卡 key、考点名）只能退回星星 ——
/// 地图上十只怪物长得一模一样。这里用 [`stable_hash`] 在预设里取一组：
/// 同一主题每次刷新都是同一枚纹章，不同主题之间又有区分度。
#[must_use]
pub fn variant_for(seed: &str) -> (Glyph, Tone) {
  PRESETS[(stable_hash(seed) % PRESETS.len() as u64) as usize]
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn glyph_follows_keywords_in_priority_order() {
    assert_eq!(glyph_for("Flame-Badge"), Glyph::Flame);
    assert_eq!(glyph_for("trophy"), Glyph::Trophy);
    assert_eq!(glyph_for("badge-book"), Glyph::Book);
    assert_eq!(glyph_for("antenna_wave"), Glyph::Antenna);
    assert_eq!(glyph_for("lock"), Glyph::Shield);
    // 优先级：火焰先于奖杯，「fire trophy」是火焰。
    assert_eq!(glyph_for("fire-trophy"), Glyph::Flame);
    // 什么都不沾：退到星星，不出空白方块。
    assert_eq!(glyph_for("zzz"), Glyph::Star);
    assert_eq!(glyph_for(""), Glyph::Star);
  }

  #[test]
  fn short_keyword_log_matches_whole_words_only() {
    assert_eq!(glyph_for("badge-log"), Glyph::Doc);
    assert_eq!(glyph_for("log"), Glyph::Doc);
    // 以前 `has("log")` 是子串匹配：catalog / blog 会被配成「日志」。
    assert_eq!(glyph_for("catalog"), Glyph::Star);
    assert_eq!(glyph_for("blog-post"), Glyph::Star);
  }

  #[test]
  fn tone_has_its_own_keyword_set() {
    assert_eq!(tone_for("flame"), Tone::Fire);
    assert_eq!(tone_for("gold-medal"), Tone::Gold);
    assert_eq!(tone_for("book"), Tone::Violet);
    assert_eq!(tone_for("signal"), Tone::Green);
    assert_eq!(tone_for("lock"), Tone::Muted);
    assert_eq!(tone_for("unknown"), Tone::Blue);
    // 图标与色调是两套关键词：`radio` 是信号图标，但色调不特殊（落到默认蓝）。
    assert_eq!(glyph_for("radio"), Glyph::Signal);
    assert_eq!(tone_for("radio"), Tone::Blue);
  }

  #[test]
  fn stable_hash_matches_published_fnv1a_vectors() {
    // FNV-1a 64 位的公开测试向量：算法被改动（常数、顺序）会立刻红，而不是悄悄让所有纹章换位。
    assert_eq!(stable_hash(""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(stable_hash("a"), 0xaf63_dc4c_8601_ec8c);
    assert_eq!(stable_hash("foobar"), 0x8594_4171_f739_67e8);
  }

  #[test]
  fn variant_is_stable_and_spreads_over_presets() {
    assert_eq!(variant_for("第一关"), variant_for("第一关"));
    let seeds = [
      "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p",
    ];
    let used: std::collections::HashSet<usize> = seeds
      .iter()
      .map(|s| (stable_hash(s) % PRESETS.len() as u64) as usize)
      .collect();
    assert!(
      used.len() >= 4,
      "16 个主题只落在 {} 组预设里，分布太集中",
      used.len()
    );
  }
}
