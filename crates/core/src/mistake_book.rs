//! 持久化错题本与累计答题统计。
//!
//! - 错题以题目内容指纹的短哈希为 key，与题号、题库、版本无关，A/B/C 重合题只记一条；
//! - 每条错题保存题目快照，题库更新或下线后仍可复习；
//! - 复习间隔按 SM-2 思路自适应：答错立即待复习并降低难度系数（ease），答对后
//!   第一次隔 1 天，之后按「上次间隔与实际间隔的较大者 × ease」增长（上限 60 天）；
//!   连续答对 [`MistakeRecord::target_streak`] 次（一般 [`MASTER_STREAK`] 次，
//!   反复答错的题最多 [`MAX_TARGET_STREAK`] 次）视为掌握并移出错题本。

use std::collections::{BTreeMap, BTreeSet, HashSet};

use serde::{Deserialize, Serialize};

use crate::bank::Bank;
use crate::categories::top_of;
use crate::fingerprint::fingerprint;
use crate::question::QuestionItem;

/// 连续答对多少次后移出错题本（答错不超过 2 次的题）。
pub const MASTER_STREAK: u8 = 3;
/// 反复答错的题最多需要连续答对的次数。
pub const MAX_TARGET_STREAK: u8 = 5;
/// 初始难度系数。
pub const DEFAULT_EASE: f64 = 2.5;
/// 难度系数下限。
pub const MIN_EASE: f64 = 1.3;
/// 每次答错扣减的难度系数。
const EASE_PENALTY: f64 = 0.2;
/// 复习间隔上限（天）。
const MAX_INTERVAL_DAYS: f64 = 60.0;
/// 错题本最多保留的条数（超出时丢弃最久未答错的）。
pub const MAX_RECORDS: usize = 1000;
const DAY_MS: i64 = 24 * 60 * 60 * 1000;

const fn default_ease() -> f64 {
  DEFAULT_EASE
}

/// 错因选项：(key, 中文名)。用于答错后的自评标注，便于按错因聚类复习。
pub const WRONG_CAUSES: &[(&str, &str)] = &[
  ("memory", "知识点没记住"),
  ("careless", "审题看错"),
  ("distractor", "被干扰项迷惑"),
  ("confused", "记混了概念"),
];

/// 题目的稳定 key：内容指纹的 FNV-1a 64 位哈希（16 位十六进制）。
#[must_use]
pub fn question_key(q: &QuestionItem) -> String {
  let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
  for b in fingerprint(q).bytes() {
    hash ^= u64::from(b);
    hash = hash.wrapping_mul(0x0100_0000_01b3);
  }
  format!("{hash:016x}")
}

/// 一条错题记录。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MistakeRecord {
  /// [`question_key`]。
  pub key: String,
  /// 题目快照。
  pub question: QuestionItem,
  /// 最近一次答错时的作答（闪卡自评「不会」时为空）。
  pub my_answer: Vec<String>,
  /// 累计答错次数。
  pub wrong_count: u32,
  /// 当前连续答对次数。
  pub streak: u8,
  /// 最近一次答错时间（毫秒时间戳）。
  pub last_wrong_ms: i64,
  /// 下次复习时间（毫秒时间戳）。
  pub due_ms: i64,
  /// 在哪些题库中答错过（A/B/C 重合题可能有多个）。
  #[serde(default)]
  pub banks: BTreeSet<Bank>,
  /// 难度系数：越小间隔增长越慢（旧记录按初始值）。
  #[serde(default = "default_ease")]
  pub ease: f64,
  /// 当前复习间隔（天），答错后归零。
  #[serde(default)]
  pub interval_days: f64,
  /// 最近一次作答时间（旧记录为 0，按最近答错时间计）。
  #[serde(default)]
  pub last_review_ms: i64,
  /// 错因（用户自评，`WRONG_CAUSES` 中的 key；空为未标注）。
  #[serde(default)]
  pub cause: Option<String>,
}

impl MistakeRecord {
  /// 需要连续答对多少次才算掌握：答错 3 次起每多错一次加 1，最多 [`MAX_TARGET_STREAK`]。
  #[must_use]
  pub fn target_streak(&self) -> u8 {
    let extra = self
      .wrong_count
      .saturating_sub(2)
      .min(u32::from(MAX_TARGET_STREAK - MASTER_STREAK));
    MASTER_STREAK + u8::try_from(extra).unwrap_or(0)
  }

  fn last_review(&self) -> i64 {
    if self.last_review_ms > 0 {
      self.last_review_ms
    } else {
      self.last_wrong_ms
    }
  }

  /// 答对后计算下次间隔（天）：首次 1 天；之后取上次间隔与实际间隔的较大者乘以 ease，
  /// 提前复习不会让间隔缩短，按时复习则正常增长。
  fn next_interval(&self, now_ms: i64) -> f64 {
    if self.interval_days <= 0.0 {
      return 1.0;
    }
    #[allow(clippy::cast_precision_loss)]
    let elapsed = (now_ms - self.last_review()).max(0) as f64 / DAY_MS as f64;
    (self.interval_days.max(elapsed) * self.ease).clamp(1.0, MAX_INTERVAL_DAYS)
  }

  /// 是否到了复习时间。
  #[must_use]
  pub const fn is_due(&self, now_ms: i64) -> bool {
    self.due_ms <= now_ms
  }

  /// 是否属于某题库；旧记录没有 `banks` 时按题目快照的 ID 前缀判断。
  #[must_use]
  pub fn in_bank(&self, bank: Bank) -> bool {
    if self.banks.is_empty() {
      Bank::of_id(self.question.id_str()) == bank
    } else {
      self.banks.contains(&bank)
    }
  }
}

/// 一次作答对错题本的影响。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordOutcome {
  /// 答错，已加入或更新错题。
  Wrong,
  /// 答对了错题本里的题：当前连续答对次数、掌握所需次数、距下次复习的天数。
  Progressed {
    streak: u8,
    target: u8,
    next_days: u16,
  },
  /// 连续答对达标，已移出错题本。
  Mastered,
  /// 答对且不在错题本中。
  Correct,
}

/// 错题本。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MistakeBook {
  pub records: Vec<MistakeRecord>,
}

impl MistakeBook {
  /// 按当前排版规则重新计算每条记录的 key（题库归一化后调用，幂等）。
  ///
  /// key 是内容指纹的哈希，题面排版一变历史 key 就失效。记录里存了题目快照，
  /// 于是可以：① 把快照文本也归一化（显示一致）；② 用快照重算 key（历史进度不丢）。
  /// 同一道题的两个指纹变体若都留过记录，合并为一条：错次数累加、到期时间取早、
  /// 最近答错时间取近、题库集合取并集。
  ///
  /// 返回发生变化（文本或 key 被改写、或被合并掉）的记录条数。
  pub fn rekey(&mut self) -> usize {
    let mut rewritten = 0usize;
    for record in &mut self.records {
      let before = record.key.clone();
      let text_changed = crate::typography::normalize_question(&mut record.question);
      let key = question_key(&record.question);
      let key_changed = key != before;
      if key_changed {
        record.key = key;
      }
      // 文本变或 key 变只算一条记录，避免对同一条记录重复计数。
      if text_changed || key_changed {
        rewritten += 1;
      }
    }

    let mut merged: Vec<MistakeRecord> = Vec::with_capacity(self.records.len());
    for record in self.records.drain(..) {
      match merged.iter_mut().find(|r| r.key == record.key) {
        None => merged.push(record),
        Some(keep) => {
          // 两个变体各自记过答错次数，合并时累加
          keep.wrong_count = keep.wrong_count.saturating_add(record.wrong_count);
          keep.streak = keep.streak.max(record.streak);
          keep.last_wrong_ms = keep.last_wrong_ms.max(record.last_wrong_ms);
          keep.last_review_ms = keep.last_review_ms.max(record.last_review_ms);
          keep.due_ms = keep.due_ms.min(record.due_ms);
          keep.interval_days = keep.interval_days.max(record.interval_days);
          keep.ease = f64::min(keep.ease, record.ease);
          keep.banks.extend(record.banks);
          if keep.cause.is_none() {
            keep.cause = record.cause;
          }
          if keep.my_answer.is_empty() {
            keep.my_answer = record.my_answer;
          }
          rewritten += 1;
        }
      }
    }
    self.records = merged;
    rewritten
  }

  /// 记录一次作答（`answer` 为空视为未作答，不做处理）。
  pub fn record(
    &mut self,
    q: &QuestionItem,
    answer: &[String],
    now_ms: i64,
  ) -> Option<RecordOutcome> {
    if answer.is_empty() {
      return None;
    }
    let correct = q.is_answer_correct(answer);
    Some(self.record_result(q, correct, answer, now_ms))
  }

  /// 记录一次自评结果（闪卡「会 / 不会」）。
  pub fn record_result(
    &mut self,
    q: &QuestionItem,
    correct: bool,
    answer: &[String],
    now_ms: i64,
  ) -> RecordOutcome {
    let key = question_key(q);
    let bank = Bank::of_id(q.id_str());
    let pos = self.records.iter().position(|r| r.key == key);
    if !correct {
      match pos {
        Some(i) => {
          let r = &mut self.records[i];
          if r.banks.is_empty() {
            r.banks.insert(Bank::of_id(r.question.id_str()));
          }
          r.banks.insert(bank);
          r.question = q.clone();
          r.my_answer = answer.to_vec();
          r.wrong_count += 1;
          r.streak = 0;
          r.last_wrong_ms = now_ms;
          r.last_review_ms = now_ms;
          r.due_ms = now_ms;
          r.ease = (r.ease - EASE_PENALTY).max(MIN_EASE);
          r.interval_days = 0.0;
        }
        None => {
          self.records.push(MistakeRecord {
            key,
            question: q.clone(),
            my_answer: answer.to_vec(),
            wrong_count: 1,
            streak: 0,
            last_wrong_ms: now_ms,
            due_ms: now_ms,
            banks: BTreeSet::from([bank]),
            ease: DEFAULT_EASE,
            interval_days: 0.0,
            last_review_ms: now_ms,
            cause: None,
          });
          self.trim();
        }
      }
      return RecordOutcome::Wrong;
    }
    let Some(i) = pos else {
      return RecordOutcome::Correct;
    };
    let r = &mut self.records[i];
    r.streak = r.streak.saturating_add(1);
    let target = r.target_streak();
    if r.streak >= target {
      self.records.remove(i);
      return RecordOutcome::Mastered;
    }
    let days = r.next_interval(now_ms);
    r.interval_days = days;
    r.last_review_ms = now_ms;
    #[allow(clippy::cast_possible_truncation)]
    {
      r.due_ms = now_ms + (days * DAY_MS as f64).round() as i64;
    }
    RecordOutcome::Progressed {
      streak: r.streak,
      target,
      #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
      next_days: days.round() as u16,
    }
  }

  /// 手动移出一道错题。
  pub fn remove(&mut self, key: &str) {
    self.records.retain(|r| r.key != key);
  }

  /// 标注错因（`cause` 为空则清除标注）；返回是否命中该题。
  pub fn set_cause(&mut self, key: &str, cause: &str) -> bool {
    let Some(r) = self.records.iter_mut().find(|r| r.key == key) else {
      return false;
    };
    r.cause = if cause.is_empty() {
      None
    } else {
      Some(cause.to_owned())
    };
    true
  }

  /// 是否包含某题。
  #[must_use]
  pub fn contains(&self, q: &QuestionItem) -> bool {
    let key = question_key(q);
    self.records.iter().any(|r| r.key == key)
  }

  /// 到期待复习的条数。
  #[must_use]
  pub fn due_count(&self, now_ms: i64) -> usize {
    self.records.iter().filter(|r| r.is_due(now_ms)).count()
  }

  /// 按最近答错时间倒序排列的全部错题。
  #[must_use]
  pub fn sorted(&self) -> Vec<MistakeRecord> {
    let mut v = self.records.clone();
    v.sort_by_key(|r| std::cmp::Reverse(r.last_wrong_ms));
    v
  }

  /// 到期待复习的错题（最早到期的在前）。
  #[must_use]
  pub fn due(&self, now_ms: i64) -> Vec<MistakeRecord> {
    let mut v: Vec<MistakeRecord> = self
      .records
      .iter()
      .filter(|r| r.is_due(now_ms))
      .cloned()
      .collect();
    v.sort_by_key(|r| r.due_ms);
    v
  }

  /// 未来 `days` 天（含今天已到期的）每天待复习的错题数，按天返回（索引 0 = 今天已到期）。
  #[must_use]
  pub fn due_timeline(&self, now_ms: i64, days: usize) -> Vec<usize> {
    let mut out = vec![0usize; days];
    for r in &self.records {
      let delta = r.due_ms - now_ms;
      let idx = if delta <= 0 {
        0
      } else {
        ((delta + DAY_MS - 1) / DAY_MS) as usize
      };
      if idx < days {
        out[idx] += 1;
      }
    }
    out
  }

  fn trim(&mut self) {
    if self.records.len() > MAX_RECORDS {
      self
        .records
        .sort_by_key(|r| std::cmp::Reverse(r.last_wrong_ms));
      self.records.truncate(MAX_RECORDS);
    }
  }
}

/// 按知识点（二级分类 P 码）聚合的错题统计。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TopicAgg {
  /// 官方分类码 P。
  pub code: &'static str,
  /// 知识点名。
  pub name: &'static str,
  /// 一级分类 key。
  pub top: &'static str,
  /// 一级分类名。
  pub top_name: &'static str,
  /// 该知识点下的错题数。
  pub mistakes: usize,
  /// 累计答错次数。
  pub total_wrong: u32,
}

/// 按知识点聚合错题（无分类码的忽略），按错题数降序。
#[must_use]
pub fn mistake_topics(book: &MistakeBook) -> Vec<TopicAgg> {
  let mut map: BTreeMap<&'static str, (usize, u32)> = BTreeMap::new();
  for r in &book.records {
    let Some(sub) = r
      .question
      .p_code()
      .and_then(crate::categories::sub_category)
    else {
      continue;
    };
    let e = map.entry(sub.code).or_default();
    e.0 += 1;
    e.1 += r.wrong_count;
  }
  let mut out: Vec<TopicAgg> = map
    .into_iter()
    .map(|(code, (mistakes, total_wrong))| {
      let sub = crate::categories::sub_category(code).expect("code from sub_category");
      let top_name = crate::categories::top_category(sub.top).map_or(sub.top, |t| t.name);
      TopicAgg {
        code,
        name: sub.name,
        top: sub.top,
        top_name,
        mistakes,
        total_wrong,
      }
    })
    .collect();
  out.sort_by_key(|a| std::cmp::Reverse(a.mistakes));
  out
}

/// 错题按当前复习间隔分桶的巩固度分布。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IntervalBuckets {
  /// 学习中（间隔为 0，刚答错或仍在频繁答错）。
  pub learning: usize,
  /// 短期（1–6 天）。
  pub short: usize,
  /// 中期（7–20 天）。
  pub medium: usize,
  /// 长期（≥ 21 天，接近掌握）。
  pub mature: usize,
}

impl IntervalBuckets {
  /// 错题总数。
  #[must_use]
  pub fn total(self) -> usize {
    self.learning + self.short + self.medium + self.mature
  }
}

/// 按复习间隔把错题分成四档，用于可视化「间隔重复」的巩固进度：
/// 答对会让间隔逐次增长（1 → 3 → 7 → … 天），答错则归零重新开始。
#[must_use]
pub fn interval_buckets(book: &MistakeBook) -> IntervalBuckets {
  let mut b = IntervalBuckets::default();
  for r in &book.records {
    let d = r.interval_days;
    if d <= 0.0 {
      b.learning += 1;
    } else if d < 7.0 {
      b.short += 1;
    } else if d < 21.0 {
      b.medium += 1;
    } else {
      b.mature += 1;
    }
  }
  b
}

/// 按错因聚合错题数（仅统计已标注的），按 [`WRONG_CAUSES`] 顺序返回 `(key, 名称, 数量)`。
#[must_use]
pub fn cause_stats(book: &MistakeBook) -> Vec<(&'static str, &'static str, usize)> {
  WRONG_CAUSES
    .iter()
    .map(|&(key, name)| {
      let n = book
        .records
        .iter()
        .filter(|r| r.cause.as_deref() == Some(key))
        .count();
      (key, name, n)
    })
    .collect()
}

/// 某一级分类的累计作答情况。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tally {
  pub answered: u32,
  pub correct: u32,
}

impl Tally {
  /// 正确率（0–1），未作答时为 `None`。
  #[must_use]
  pub fn rate(self) -> Option<f64> {
    (self.answered > 0).then(|| f64::from(self.correct) / f64::from(self.answered))
  }
}

/// 按一级分类累计的作答情况。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryStats {
  /// key = 一级分类 key。
  #[serde(default)]
  pub categories: BTreeMap<String, Tally>,
  /// key = 官方分类码（P 码），用于薄弱知识点下钻。
  #[serde(default)]
  pub subs: BTreeMap<String, Tally>,
  /// 全部作答次数（含无分类码的题）。
  #[serde(default)]
  pub answered: u32,
  /// 全部答对次数。
  #[serde(default)]
  pub correct: u32,
}

impl CategoryStats {
  fn record(&mut self, q: &QuestionItem, correct: bool) {
    self.answered += 1;
    self.correct += u32::from(correct);
    if let Some(code) = q.p_code() {
      let t = self.subs.entry(code.to_owned()).or_default();
      t.answered += 1;
      t.correct += u32::from(correct);
      if let Some(top) = top_of(code) {
        let t = self.categories.entry(top.key.to_owned()).or_default();
        t.answered += 1;
        t.correct += u32::from(correct);
      }
    }
  }

  /// 作答不少于 `min_answered` 次的分类中正确率最低的一个。
  #[must_use]
  pub fn weakest(&self, min_answered: u32) -> Option<(&str, Tally)> {
    self
      .categories
      .iter()
      .filter(|(_, t)| t.answered >= min_answered)
      .min_by(|(_, a), (_, b)| a.rate().unwrap_or(0.0).total_cmp(&b.rate().unwrap_or(0.0)))
      .map(|(k, t)| (k.as_str(), *t))
  }

  /// 作答不少于 `min_answered` 次的分类码（P 码）中正确率最低的一个。
  #[must_use]
  pub fn weakest_sub(&self, min_answered: u32) -> Option<(&str, Tally)> {
    self
      .subs
      .iter()
      .filter(|(_, t)| t.answered >= min_answered)
      .min_by(|(_, a), (_, b)| a.rate().unwrap_or(0.0).total_cmp(&b.rate().unwrap_or(0.0)))
      .map(|(k, t)| (k.as_str(), *t))
  }

  /// 某一级分类下的分类码累计（按定义顺序）。
  #[must_use]
  pub fn subs_of(&self, top_key: &str) -> Vec<(&str, Tally)> {
    let mut out: Vec<(&str, Tally)> = Vec::new();
    for code in crate::categories::sub_codes_of(top_key) {
      if let Some(t) = self.subs.get(code) {
        out.push((code, *t));
      }
    }
    out
  }
}

/// 单个题库的累计统计与已做过的题。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BankStats {
  #[serde(flatten)]
  pub stats: CategoryStats,
  /// 做过的题（[`question_key`]），用于题库覆盖率。
  #[serde(default)]
  pub seen: BTreeSet<String>,
}

impl BankStats {
  /// 题库中做过的题数（只统计仍在 `questions` 里的题，题库改版删除的题不计）。
  #[must_use]
  pub fn covered<'a>(&self, questions: impl IntoIterator<Item = &'a QuestionItem>) -> usize {
    let keys: HashSet<String> = questions.into_iter().map(question_key).collect();
    keys.iter().filter(|k| self.seen.contains(*k)).count()
  }

  /// 是否做过这道题。
  #[must_use]
  pub fn has_seen(&self, q: &QuestionItem) -> bool {
    self.seen.contains(&question_key(q))
  }
}

/// 累计答题统计：全部题库合计（与旧版存储格式兼容）+ 按题库拆分。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudyStats {
  #[serde(flatten)]
  pub total: CategoryStats,
  /// key = 题库字母。
  #[serde(default)]
  pub banks: BTreeMap<Bank, BankStats>,
}

impl StudyStats {
  /// 记录一次作答结果，题库按题目 ID 前缀判断。
  pub fn record(&mut self, q: &QuestionItem, correct: bool) {
    self.total.record(q, correct);
    let b = self.banks.entry(Bank::of_id(q.id_str())).or_default();
    b.stats.record(q, correct);
    b.seen.insert(question_key(q));
  }

  /// 只标记做过（旧版存档迁移用，不计入正确率）。
  pub fn mark_seen(&mut self, q: &QuestionItem) {
    self
      .banks
      .entry(Bank::of_id(q.id_str()))
      .or_default()
      .seen
      .insert(question_key(q));
  }

  /// 某题库的统计。
  #[must_use]
  pub fn bank(&self, bank: Bank) -> Option<&BankStats> {
    self.banks.get(&bank)
  }

  /// 作答最多的题库（视为用户正在备考的类别）。
  #[must_use]
  pub fn main_bank(&self) -> Option<Bank> {
    self
      .banks
      .iter()
      .filter(|(_, s)| s.stats.answered > 0)
      .max_by_key(|(_, s)| s.stats.answered)
      .map(|(b, _)| *b)
  }

  /// 指定题库（`None` 为全部）的分类统计。
  #[must_use]
  pub fn categories_of(&self, bank: Option<Bank>) -> Option<&CategoryStats> {
    match bank {
      None => Some(&self.total),
      Some(b) => self.bank(b).map(|s| &s.stats),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::question::{Codes, QuestionOption, QuestionType};

  fn q(text: &str, answer: &str) -> QuestionItem {
    QuestionItem {
      id: Some("A-1".into()),
      codes: Codes {
        j: Some("LY0001".into()),
        p: Some("1.1.1".into()),
      },
      question: text.into(),
      options: vec![
        QuestionOption {
          key: "A".into(),
          text: "x".into(),
        },
        QuestionOption {
          key: "B".into(),
          text: "y".into(),
        },
      ],
      answer_keys: vec![answer.into()],
      kind: QuestionType::Single,
      pages: None,
      image_url: None,
      explanation: None,
    }
  }

  fn ans(k: &str) -> Vec<String> {
    vec![k.to_owned()]
  }

  #[test]
  fn key_depends_on_content_only() {
    let a = q("题目", "A");
    let mut b = a.clone();
    b.id = Some("C-99".into());
    assert_eq!(question_key(&a), question_key(&b));
    assert_ne!(question_key(&a), question_key(&q("另一题", "A")));
  }

  #[test]
  fn rekey_survives_typography_change() {
    // 旧数据里的题面没有归一化排版
    let legacy = q("使用 FT8模式 时", "A");
    let mut book = MistakeBook::default();
    assert_eq!(
      book.record(&legacy, &ans("B"), 0),
      Some(RecordOutcome::Wrong)
    );
    let old_key = book.records[0].key.clone();

    // 归一化后：key 重算、快照文本也归一化，记录仍认得归一化后的题目
    let current = q("使用 FT8 模式 时", "A");
    assert_ne!(old_key, question_key(&current));
    assert!(book.rekey() > 0);
    assert_eq!(book.records.len(), 1);
    assert_eq!(book.records[0].question.question, "使用 FT8 模式 时");
    assert_eq!(book.records[0].key, question_key(&current));
    assert!(book.contains(&current));
    assert_eq!(book.records[0].wrong_count, 1);
    // 幂等
    assert_eq!(book.rekey(), 0);
  }

  #[test]
  fn rekey_merges_variant_records() {
    // 同一道题的空格变体各留过一条记录
    let a = q("使用 FT8模式", "A");
    let b = q("使用 FT8 模式", "A");
    let mut book = MistakeBook::default();
    book.record(&a, &ans("B"), 0);
    book.record(&b, &ans("B"), 100);
    assert_eq!(book.records.len(), 2);
    assert!(book.rekey() > 0);
    assert_eq!(book.records.len(), 1);
    assert_eq!(book.records[0].wrong_count, 2);
    assert_eq!(book.records[0].last_wrong_ms, 100);
    assert!(book.contains(&b));
  }

  #[test]
  fn wrong_then_mastered_after_streak() {
    let mut book = MistakeBook::default();
    let item = q("题目", "A");
    assert_eq!(book.record(&item, &ans("B"), 0), Some(RecordOutcome::Wrong));
    assert_eq!(book.due_count(0), 1);
    assert_eq!(
      book.record(&item, &ans("A"), 10),
      Some(RecordOutcome::Progressed {
        streak: 1,
        target: 3,
        next_days: 1
      })
    );
    assert_eq!(book.due_count(10), 0);
    assert_eq!(book.due_count(10 + DAY_MS), 1);
    // 按时（1 天后）复习：间隔 1 × 2.5
    let t = 10 + DAY_MS;
    assert_eq!(
      book.record(&item, &ans("A"), t),
      Some(RecordOutcome::Progressed {
        streak: 2,
        target: 3,
        next_days: 3
      })
    );
    assert_eq!(book.records[0].due_ms, t + DAY_MS * 5 / 2);
    assert_eq!(
      book.record(&item, &ans("A"), t + 3 * DAY_MS),
      Some(RecordOutcome::Mastered)
    );
    assert!(book.records.is_empty());
  }

  #[test]
  fn interval_grows_with_elapsed_time_and_ease() {
    let mut book = MistakeBook::default();
    let item = q("题目", "A");
    book.record(&item, &ans("B"), 0);
    book.record(&item, &ans("A"), 0);
    // 拖了 10 天才复习：按实际间隔 10 天 × 2.5
    let outcome = book.record(&item, &ans("A"), 10 * DAY_MS);
    assert_eq!(
      outcome,
      Some(RecordOutcome::Progressed {
        streak: 2,
        target: 3,
        next_days: 25
      })
    );
    // 提前复习不缩短间隔
    let mut early = MistakeBook::default();
    early.record(&item, &ans("B"), 0);
    early.record(&item, &ans("A"), 0);
    let outcome = early.record(&item, &ans("A"), 60_000);
    assert_eq!(
      outcome,
      Some(RecordOutcome::Progressed {
        streak: 2,
        target: 3,
        next_days: 3
      })
    );
  }

  #[test]
  fn repeated_mistakes_lower_ease_and_need_longer_streak() {
    let mut book = MistakeBook::default();
    let item = q("题目", "A");
    for t in 0..8 {
      book.record(&item, &ans("B"), t);
    }
    let r = &book.records[0];
    assert!((r.ease - MIN_EASE).abs() < 1e-9);
    assert_eq!(r.target_streak(), MAX_TARGET_STREAK);
    for n in 1..MAX_TARGET_STREAK {
      assert!(matches!(
        book.record(&item, &ans("A"), 100 + i64::from(n)),
        Some(RecordOutcome::Progressed { streak, target: MAX_TARGET_STREAK, .. }) if streak == n
      ));
    }
    assert_eq!(
      book.record(&item, &ans("A"), 200),
      Some(RecordOutcome::Mastered)
    );
  }

  #[test]
  fn legacy_records_get_defaults() {
    let json = r#"{"records":[{"key":"k","question":{"question":"q","options":[],"answer_keys":["A"],"type":"single"},"my_answer":["B"],"wrong_count":1,"streak":1,"last_wrong_ms":5,"due_ms":9}]}"#;
    let book: MistakeBook = match serde_json::from_str(json) {
      Ok(b) => b,
      Err(e) => panic!("legacy mistake book should parse: {e}"),
    };
    let r = &book.records[0];
    assert!((r.ease - DEFAULT_EASE).abs() < 1e-9);
    assert_eq!(
      (r.interval_days, r.last_review_ms, r.last_review()),
      (0.0, 0, 5)
    );
  }

  #[test]
  fn wrong_resets_streak_and_counts() {
    let mut book = MistakeBook::default();
    let item = q("题目", "A");
    book.record(&item, &ans("B"), 0);
    book.record(&item, &ans("A"), 1);
    book.record(&item, &ans("B"), 2);
    let r = &book.records[0];
    assert_eq!((r.wrong_count, r.streak, r.due_ms), (2, 0, 2));
  }

  #[test]
  fn correct_unknown_question_is_not_added() {
    let mut book = MistakeBook::default();
    assert_eq!(
      book.record(&q("题目", "A"), &ans("A"), 0),
      Some(RecordOutcome::Correct)
    );
    assert_eq!(book.record(&q("题目", "A"), &[], 0), None);
    assert!(book.records.is_empty());
  }

  #[test]
  fn stats_accumulate_by_top_category() {
    let mut s = StudyStats::default();
    let item = q("题目", "A");
    s.record(&item, true);
    s.record(&item, false);
    let top = top_of("1.1.1").expect("category exists").key;
    let expected = Tally {
      answered: 2,
      correct: 1,
    };
    assert_eq!(s.total.categories[top], expected);
    assert_eq!((s.total.answered, s.total.correct), (2, 1));
    assert_eq!(
      s.bank(Bank::A).expect("bank A").stats.categories[top],
      expected
    );
    assert!(s.bank(Bank::B).is_none());
    assert_eq!(s.main_bank(), Some(Bank::A));
    assert_eq!(s.total.weakest(2), Some((top, expected)));
    assert_eq!(s.total.weakest(3), None);
  }

  #[test]
  fn records_track_banks() {
    let mut book = MistakeBook::default();
    let a = q("题目", "A");
    let mut b = a.clone();
    b.id = Some("B-7".into());
    book.record(&a, &ans("B"), 0);
    book.record(&b, &ans("B"), 1);
    assert_eq!(book.records.len(), 1);
    let r = &book.records[0];
    assert!(r.in_bank(Bank::A) && r.in_bank(Bank::B) && !r.in_bank(Bank::C));

    let mut legacy = r.clone();
    legacy.banks.clear();
    legacy.question.id = Some("C-1".into());
    assert!(legacy.in_bank(Bank::C) && !legacy.in_bank(Bank::A));
  }

  #[test]
  fn aggregates_mistakes_by_topic() {
    let mut book = MistakeBook::default();
    let mut a = q("题目A", "A");
    a.codes.p = Some("1.1.1".into());
    let mut b = q("题目B", "A");
    b.codes.p = Some("1.1.2".into());
    book.record(&a, &ans("B"), 0);
    book.record(&a, &ans("B"), 1);
    book.record(&b, &ans("B"), 2);
    let topics = mistake_topics(&book);
    assert_eq!(topics.len(), 2);
    // 按错题数降序：1.1.1 错 1 题（累计 2 次）、1.1.2 错 1 题（累计 1 次）。
    assert_eq!(topics[0].code, "1.1.1");
    assert_eq!(topics[0].mistakes, 1);
    assert_eq!(topics[0].total_wrong, 2);
    assert_eq!(topics[0].top_name, "无线电法规与管理");
    assert_eq!(topics[1].code, "1.1.2");
  }

  #[test]
  fn cause_annotation_and_stats() {
    let mut book = MistakeBook::default();
    let a = q("题目A", "A");
    let b = q("题目B", "A");
    book.record(&a, &ans("B"), 0);
    book.record(&b, &ans("B"), 1);
    let ka = question_key(&a);
    let kb = question_key(&b);
    assert!(book.set_cause(&ka, "memory"));
    assert!(book.set_cause(&kb, "confused"));
    assert!(!book.set_cause("nope", "memory"));
    let stats = cause_stats(&book);
    assert_eq!(stats[0], ("memory", "知识点没记住", 1));
    assert_eq!(stats[3], ("confused", "记混了概念", 1));
    // 清除标注。
    assert!(book.set_cause(&ka, ""));
    let stats = cause_stats(&book);
    assert_eq!(stats[0].2, 0);
  }

  #[test]
  fn interval_buckets_group_by_interval() {
    let mut book = MistakeBook::default();
    let mk = |interval: f64, text: &str| MistakeRecord {
      key: question_key(&q(text, "A")),
      question: q(text, "A"),
      my_answer: vec![],
      wrong_count: 1,
      streak: 0,
      last_wrong_ms: 0,
      due_ms: 0,
      banks: BTreeSet::new(),
      ease: DEFAULT_EASE,
      interval_days: interval,
      last_review_ms: 0,
      cause: None,
    };
    book.records.push(mk(0.0, "一"));
    book.records.push(mk(0.0, "二"));
    book.records.push(mk(3.0, "三"));
    book.records.push(mk(10.0, "四"));
    book.records.push(mk(30.0, "五"));
    let b = interval_buckets(&book);
    assert_eq!(b.learning, 2);
    assert_eq!(b.short, 1);
    assert_eq!(b.medium, 1);
    assert_eq!(b.mature, 1);
    assert_eq!(b.total(), 5);
  }

  #[test]
  fn coverage_counts_current_questions_only() {
    let mut s = StudyStats::default();
    let one = q("一", "A");
    let two = q("二", "A");
    let gone = q("已删除", "A");
    s.record(&one, true);
    s.mark_seen(&gone);
    let bank = s.bank(Bank::A).expect("bank A");
    assert!(bank.has_seen(&one) && !bank.has_seen(&two));
    assert_eq!(bank.covered([&one, &two]), 1);
    assert_eq!(bank.stats.answered, 1);
  }

  #[test]
  fn stats_json_is_backward_compatible() {
    let legacy = r#"{"categories":{"x":{"answered":3,"correct":2}},"answered":3,"correct":2}"#;
    let s: StudyStats = serde_json::from_str(legacy).expect("legacy format");
    assert_eq!(s.total.answered, 3);
    assert!(s.banks.is_empty());

    let mut s = s;
    s.record(&q("题目", "A"), true);
    let json = serde_json::to_string(&s).expect("serialize");
    let back: StudyStats = serde_json::from_str(&json).expect("roundtrip");
    assert_eq!(back, s);
    assert!(json.contains(r#""banks":{"A":"#));
  }
}
