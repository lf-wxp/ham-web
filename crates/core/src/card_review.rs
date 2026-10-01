//! 知识卡片间隔复习：Q 简语、通联缩语、术语、字母解释法、莫尔斯字符。
//!
//! 调度沿用 SM-2 思路：每张卡有难度系数 ease；「记得」时间隔按「上次间隔与实际间隔的较大者 × ease」
//! 增长，「模糊」只小幅增长并降低 ease，「忘了」重新开始并在 10 分钟后再出现。
//! 新卡每个卡组每天最多引入 [`NEW_PER_DAY`] 张，避免一次学太多。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// 每个卡组每天最多引入的新卡数。
pub const NEW_PER_DAY: u32 = 10;
const DEFAULT_EASE: f64 = 2.5;
const MIN_EASE: f64 = 1.3;
const MAX_EASE: f64 = 3.0;
const MAX_INTERVAL_DAYS: f64 = 180.0;
/// 忘记后多久再出现（毫秒）。
const RELEARN_MS: i64 = 10 * 60 * 1000;
const DAY_MS: i64 = 24 * 60 * 60 * 1000;

const fn default_ease() -> f64 {
  DEFAULT_EASE
}

/// 卡组。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Deck {
  /// Q 简语（QRM、QSL…）。
  QCode,
  /// 通联缩语（73、CQ、DE…）。
  Abbrev,
  /// 术语表常用词条。
  Glossary,
  /// 字母解释法。
  Phonetic,
  /// 莫尔斯电码字母与数字。
  Morse,
}

impl Deck {
  pub const ALL: [Self; 5] = [
    Self::QCode,
    Self::Abbrev,
    Self::Phonetic,
    Self::Morse,
    Self::Glossary,
  ];

  #[must_use]
  pub const fn label(self) -> &'static str {
    match self {
      Self::QCode => "Q 简语",
      Self::Abbrev => "通联缩语",
      Self::Glossary => "术语",
      Self::Phonetic => "字母解释法",
      Self::Morse => "莫尔斯电码",
    }
  }

  const fn prefix(self) -> &'static str {
    match self {
      Self::QCode => "q",
      Self::Abbrev => "a",
      Self::Glossary => "g",
      Self::Phonetic => "p",
      Self::Morse => "m",
    }
  }

  /// 卡片 id：`卡组前缀:内容`，如 `q:QRM`、`m:A`。
  #[must_use]
  pub fn card_id(self, key: &str) -> String {
    format!("{}:{key}", self.prefix())
  }

  /// 由卡片 id 反查卡组。
  #[must_use]
  pub fn of_id(id: &str) -> Option<Self> {
    let prefix = id.split_once(':')?.0;
    Self::ALL.into_iter().find(|d| d.prefix() == prefix)
  }

  /// URL 参数取值。
  #[must_use]
  pub const fn param(self) -> &'static str {
    match self {
      Self::QCode => "qcode",
      Self::Abbrev => "abbrev",
      Self::Glossary => "glossary",
      Self::Phonetic => "phonetic",
      Self::Morse => "morse",
    }
  }

  #[must_use]
  pub fn from_param(s: &str) -> Option<Self> {
    Self::ALL.into_iter().find(|d| d.param() == s)
  }
}

/// 自评结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade {
  /// 忘了。
  Again,
  /// 想了一会儿 / 不太确定。
  Hard,
  /// 记得。
  Good,
}

/// 一张卡片的复习状态。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardState {
  #[serde(default = "default_ease")]
  pub ease: f64,
  /// 当前间隔（天），0 表示学习中。
  #[serde(default)]
  pub interval_days: f64,
  pub due_ms: i64,
  #[serde(default)]
  pub last_ms: i64,
  /// 累计复习次数与忘记次数。
  #[serde(default)]
  pub reps: u32,
  #[serde(default)]
  pub lapses: u32,
}

impl CardState {
  /// 间隔达到 21 天视为已熟记。
  #[must_use]
  pub fn mature(&self) -> bool {
    self.interval_days >= 21.0
  }
}

/// 全部卡片的复习进度（持久化到本地）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Schedule {
  #[serde(default)]
  pub cards: BTreeMap<String, CardState>,
  /// 今天（本地日期 `YYYY-MM-DD`）各卡组已引入的新卡数。
  #[serde(default)]
  pub new_day: String,
  #[serde(default)]
  pub new_today: BTreeMap<Deck, u32>,
}

impl Schedule {
  /// 对一张卡打分，返回下次复习距今的天数（忘记时为 0）。
  pub fn grade(&mut self, id: &str, grade: Grade, now_ms: i64, today: &str) -> f64 {
    if !self.cards.contains_key(id) {
      self.roll_day(today);
      if let Some(deck) = Deck::of_id(id) {
        *self.new_today.entry(deck).or_default() += 1;
      }
    }
    let card = self.cards.entry(id.to_owned()).or_insert(CardState {
      ease: DEFAULT_EASE,
      interval_days: 0.0,
      due_ms: now_ms,
      last_ms: 0,
      reps: 0,
      lapses: 0,
    });
    #[allow(clippy::cast_precision_loss)]
    let elapsed = if card.last_ms > 0 {
      (now_ms - card.last_ms).max(0) as f64 / DAY_MS as f64
    } else {
      0.0
    };
    let base = card.interval_days.max(elapsed);
    let interval = match grade {
      Grade::Again => {
        if card.reps > 0 {
          card.lapses += 1;
        }
        card.ease = (card.ease - 0.2).max(MIN_EASE);
        0.0
      }
      Grade::Hard => {
        card.ease = (card.ease - 0.15).max(MIN_EASE);
        if card.interval_days <= 0.0 {
          1.0
        } else {
          (base * 1.2).clamp(1.0, MAX_INTERVAL_DAYS)
        }
      }
      Grade::Good => {
        if card.interval_days <= 0.0 {
          // 新卡或刚忘记的卡：第一次记得隔 1 天，之前从没忘过的直接 2 天
          if card.lapses == 0 && card.reps == 0 {
            2.0
          } else {
            1.0
          }
        } else {
          card.ease = (card.ease + 0.05).min(MAX_EASE);
          (base * card.ease).clamp(1.0, MAX_INTERVAL_DAYS)
        }
      }
    };
    card.interval_days = interval;
    card.reps += 1;
    card.last_ms = now_ms;
    #[allow(clippy::cast_possible_truncation)]
    {
      card.due_ms = if interval <= 0.0 {
        now_ms + RELEARN_MS
      } else {
        now_ms + (interval * DAY_MS as f64) as i64
      };
    }
    interval
  }

  fn roll_day(&mut self, today: &str) {
    if self.new_day != today {
      self.new_day = today.to_owned();
      self.new_today.clear();
    }
  }

  /// 今天该卡组还能引入多少新卡。
  #[must_use]
  pub fn new_left(&self, deck: Deck, today: &str) -> u32 {
    let used = if self.new_day == today {
      self.new_today.get(&deck).copied().unwrap_or(0)
    } else {
      0
    };
    NEW_PER_DAY.saturating_sub(used)
  }

  /// 到期的卡片数（按卡组）。
  #[must_use]
  pub fn due_by_deck(&self, now_ms: i64) -> BTreeMap<Deck, usize> {
    let mut out = BTreeMap::new();
    for (id, c) in &self.cards {
      if c.due_ms <= now_ms
        && let Some(d) = Deck::of_id(id)
      {
        *out.entry(d).or_default() += 1;
      }
    }
    out
  }

  /// 到期总数。
  #[must_use]
  pub fn due_total(&self, now_ms: i64) -> usize {
    self.cards.values().filter(|c| c.due_ms <= now_ms).count()
  }

  /// 某卡组今天的复习队列：先到期卡（最早到期在前），再按给定顺序补充新卡。
  #[must_use]
  pub fn queue(&self, deck: Deck, ids: &[String], now_ms: i64, today: &str) -> Vec<String> {
    let mut due: Vec<(&String, i64)> = ids
      .iter()
      .filter_map(|id| {
        self
          .cards
          .get(id)
          .filter(|c| c.due_ms <= now_ms)
          .map(|c| (id, c.due_ms))
      })
      .collect();
    due.sort_by_key(|&(_, d)| d);
    let fresh = ids
      .iter()
      .filter(|id| !self.cards.contains_key(*id))
      .take(self.new_left(deck, today) as usize);
    due
      .into_iter()
      .map(|(id, _)| id.clone())
      .chain(fresh.cloned())
      .collect()
  }

  /// 某卡组的学习进度：(已学, 已熟记)。
  #[must_use]
  pub fn progress(&self, ids: &[String]) -> (usize, usize) {
    let learned: Vec<&CardState> = ids.iter().filter_map(|id| self.cards.get(id)).collect();
    (learned.len(), learned.iter().filter(|c| c.mature()).count())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const DAY: &str = "2026-10-01";

  fn ids(deck: Deck, n: usize) -> Vec<String> {
    (0..n).map(|i| deck.card_id(&i.to_string())).collect()
  }

  #[test]
  fn intervals_grow_and_lapses_reset() {
    let mut s = Schedule::default();
    let id = Deck::QCode.card_id("QRM");
    assert!((s.grade(&id, Grade::Good, 0, DAY) - 2.0).abs() < 1e-9);
    // 按时复习：2 天 × (2.5 + 0.05)
    let t = 2 * DAY_MS;
    let next = s.grade(&id, Grade::Good, t, DAY);
    assert!((next - 5.1).abs() < 1e-9, "{next}");
    // 忘记：10 分钟后再出现，ease 降低，计一次 lapse
    assert!(s.grade(&id, Grade::Again, t + DAY_MS, DAY).abs() < 1e-9);
    let c = &s.cards[&id];
    assert_eq!((c.lapses, c.due_ms), (1, t + DAY_MS + RELEARN_MS));
    assert!((c.ease - 2.35).abs() < 1e-9);
    // 忘记后再记得只隔 1 天
    assert!((s.grade(&id, Grade::Good, t + DAY_MS + RELEARN_MS, DAY) - 1.0).abs() < 1e-9);
    // 模糊：小幅增长并降低 ease
    let before = s.cards[&id].ease;
    let hard = s.grade(&id, Grade::Hard, t + 2 * DAY_MS + RELEARN_MS, DAY);
    assert!((1.0..=1.3).contains(&hard), "{hard}");
    assert!(s.cards[&id].ease < before);
  }

  #[test]
  fn queue_puts_due_first_and_limits_new_cards() {
    let mut s = Schedule::default();
    let all = ids(Deck::Morse, 30);
    let q = s.queue(Deck::Morse, &all, 0, DAY);
    assert_eq!(q.len(), NEW_PER_DAY as usize);
    for id in &q[..3] {
      s.grade(id, Grade::Again, 0, DAY);
    }
    // 3 张到期卡排在最前，今天剩下 7 张新卡额度
    let later = RELEARN_MS + 1;
    let q = s.queue(Deck::Morse, &all, later, DAY);
    assert_eq!(&q[..3], &all[..3]);
    assert_eq!(q.len(), 3 + 7);
    assert_eq!(s.due_by_deck(later).get(&Deck::Morse), Some(&3));
    assert_eq!(s.due_total(0), 0);
    // 第二天新卡额度重置
    assert_eq!(s.new_left(Deck::Morse, "2026-10-02"), NEW_PER_DAY);
    assert_eq!(s.progress(&all), (3, 0));
  }

  #[test]
  fn ids_and_serde() {
    assert_eq!(Deck::of_id("g:驻波比"), Some(Deck::Glossary));
    assert_eq!(Deck::of_id("x:1"), None);
    assert_eq!(Deck::from_param("phonetic"), Some(Deck::Phonetic));
    let mut s = Schedule::default();
    s.grade("p:A", Grade::Good, 5, DAY);
    let back: Schedule = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    assert_eq!(back, s);
  }
}
