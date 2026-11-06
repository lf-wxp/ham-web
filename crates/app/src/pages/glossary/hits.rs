//! 术语在各题库出现的题数：首帧之后分批统计，结果在本次会话内缓存。
//!
//! 统计本身是「每个术语 × 约 3500 道题」的子串查找：60 张卡片一起算，在中端手机上是
//! 一两百毫秒的长任务。而题库已缓存时（第二次进入术语表），检索文本当场就能拿到，
//! 统计会挤在页面第一帧之前 —— 术语表要等它算完才出来。现在：
//!
//! - 检索文本（每题拼接 + 转小写）只在题库换了之后重建，否则复用上一次的；
//! - 统计过的术语记在缓存里，再进来直接显示；
//! - 没统计过的，由 `GlossaryView` 在首帧之后每帧统计 [`BATCH`] 个（`after_paint::each_frame`）。

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use ham_web_core::Bank;

use super::Haystacks;
use crate::data::{self, Questions};

/// 每帧统计的术语数：单个术语在中端手机上约 3ms，六个一批落在一帧的预算里。
pub(super) const BATCH: usize = 6;

/// 术语 → 各题库出现的题数。
pub(super) type HitMap = HashMap<&'static str, Vec<(Bank, usize)>>;

struct Cached {
  /// 建检索文本时用的三份题库数据：题库重新加载（换版本 / 更新）后就不是同一批了。
  /// 拿着原数据比对（`data::same_banks`），而不是只记地址：旧数据释放后新数据可能分到同一地址。
  source: Vec<Option<Questions>>,
  haystacks: Haystacks,
  hits: HitMap,
}

thread_local! {
  static CACHE: RefCell<Option<Cached>> = const { RefCell::new(None) };
}

/// 当前三个题库（加载失败的为 `None`）。题库已在内存里时立刻返回。
pub(super) async fn load_banks() -> Vec<(Bank, Option<Questions>)> {
  let mut out = Vec::with_capacity(Bank::ALL.len());
  for bank in Bank::ALL {
    out.push((bank, data::load_bank(None, bank, false).await.ok()));
  }
  out
}

fn source_of(banks: &[(Bank, Option<Questions>)]) -> Vec<Option<Questions>> {
  banks.iter().map(|(_, qs)| qs.clone()).collect()
}

/// 题库没换时，上一次的检索文本与已统计的结果。
pub(super) fn cached(banks: &[(Bank, Option<Questions>)]) -> Option<(Haystacks, HitMap)> {
  let source = source_of(banks);
  CACHE.with_borrow(|c| {
    c.as_ref()
      .filter(|c| data::same_banks(&c.source, &source))
      .map(|c| (c.haystacks.clone(), c.hits.clone()))
  })
}

/// 逐题构建检索文本并记进缓存（旧的统计结果随之作废）。
pub(super) fn build(banks: Vec<(Bank, Option<Questions>)>) -> Haystacks {
  let source = source_of(&banks);
  let haystacks: Haystacks = Arc::new(
    banks
      .into_iter()
      .map(|(bank, qs)| {
        let texts = qs.map_or_else(Vec::new, |qs| qs.iter().map(|q| q.search_text()).collect());
        (bank, texts)
      })
      .collect(),
  );
  CACHE.with_borrow_mut(|c| {
    *c = Some(Cached {
      source,
      haystacks: haystacks.clone(),
      hits: HitMap::new(),
    });
  });
  haystacks
}

/// 统计一个术语，并记进缓存（缓存已换成别的检索文本时只返回、不记）。
pub(super) fn count(haystacks: &Haystacks, term: &'static str) -> Vec<(Bank, usize)> {
  let hits = ham_web_core::glossary::bank_hits(term, haystacks);
  CACHE.with_borrow_mut(|c| {
    if let Some(c) = c.as_mut().filter(|c| Arc::ptr_eq(&c.haystacks, haystacks)) {
      c.hits.insert(term, hits.clone());
    }
  });
  hits
}
