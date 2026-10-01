//! 错题 / 收藏打印版：A4 排版，可选显示答案（随题或集中在末尾）与解析，直接调用浏览器打印。

mod print_page;
mod question_block;

pub use print_page::PrintPage;

use std::collections::{HashMap, HashSet};

use ham_web_core::{Bank, QuestionItem};

use crate::{data, store, study};

/// 打印内容来源。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Source {
  Mistakes,
  Bookmarks,
}

impl Source {
  /// 来源的中文名称。
  pub(crate) fn title(self) -> &'static str {
    match self {
      Self::Mistakes => "错题集",
      Self::Bookmarks => "收藏集",
    }
  }
}

/// 答案展示方式。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnswerMode {
  /// 不显示答案。
  Hidden,
  /// 随题显示。
  Inline,
  /// 集中在末尾。
  End,
}

/// 打印用的一道题。
#[derive(Clone)]
pub(crate) struct Item {
  pub(crate) q: QuestionItem,
  /// 错题累计答错次数（收藏为 0）。
  pub(crate) wrong: u32,
}

/// 错题集 / 收藏集的题目（尽量换成题库中的最新版本），可按题库筛选。
pub(crate) async fn load_items(source: Source, bank: Option<Bank>) -> Vec<Item> {
  let mut by_id: HashMap<String, QuestionItem> = HashMap::new();
  for b in Bank::ALL {
    if let Ok(qs) = data::load_bank(None, b, false).await {
      for q in qs.iter() {
        if let Some(id) = q.stable_id() {
          by_id.entry(id).or_insert_with(|| q.clone());
        }
      }
    }
  }
  match source {
    Source::Mistakes => study::load_book()
      .sorted()
      .into_iter()
      .filter(|m| bank.is_none_or(|b| m.in_bank(b)))
      .map(|m| {
        let fresh = m
          .question
          .stable_id()
          .and_then(|id| by_id.get(&id).cloned());
        Item {
          q: fresh.unwrap_or(m.question),
          wrong: m.wrong_count,
        }
      })
      .collect(),
    Source::Bookmarks => {
      let ids = store::load_bookmarks();
      let mut seen = HashSet::new();
      let mut out: Vec<Item> = by_id
        .into_iter()
        .filter(|(id, q)| {
          ids.contains(id)
            && bank.is_none_or(|b| Bank::of_id(q.id_str()) == b)
            && seen.insert(id.clone())
        })
        .map(|(_, q)| Item { q, wrong: 0 })
        .collect();
      out.sort_by_key(|it| {
        let id = it.q.stable_id().unwrap_or_default();
        let (prefix, n) = id.split_once('-').unwrap_or((&id, ""));
        (prefix.to_owned(), n.parse::<u32>().unwrap_or(u32::MAX))
      });
      out
    }
  }
}

/// 题目的解析文本（去空白，空则视为无解析）。
pub(crate) fn explanation(q: &QuestionItem) -> Option<String> {
  q.explanation
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .map(ToOwned::to_owned)
}
