//! 术语表：业余无线电常用术语、英文缩写与通俗解释，按题目类型分类，并统计在各题库中出现的题数。

mod glossary_page;
mod glossary_view;
mod stat;
mod term_card;

pub use glossary_page::GlossaryPage;

use std::sync::Arc;

use ham_web_core::Bank;
use ham_web_core::categories::{TOP_CATEGORIES, top_category};
use ham_web_core::glossary::OTHER_CATEGORY;

use crate::cn::cn;

const PAGE: usize = 60;
const ALL: &str = "全部";
const OTHER_COLOR: &str = "#71717a";

/// 各题库题目的小写搜索文本（与分类浏览页的关键词搜索范围一致）。
type Haystacks = Arc<Vec<(Bank, Vec<String>)>>;

fn pill(active: bool, base: &str) -> String {
  cn(&[
    base,
    if active {
      "bg-primary text-primary-foreground"
    } else {
      "hover:bg-accent"
    },
  ])
}

/// 分类排序位置：按一级分类定义顺序，「其他」排最后。
fn category_pos(key: &str) -> usize {
  TOP_CATEGORIES
    .iter()
    .position(|c| c.key == key)
    .unwrap_or(TOP_CATEGORIES.len())
}

fn category_meta(key: &str) -> (&'static str, &'static str) {
  top_category(key).map_or((OTHER_CATEGORY, OTHER_COLOR), |c| (c.name, c.color))
}

/// 过短的纯 ASCII 术语（如 `K`、`73`、`DE`）做子串统计会产生大量误匹配，不统计题库出现次数。
fn is_countable(term: &str) -> bool {
  !term.is_ascii() || term.chars().count() >= 3
}

fn browse_href(bank: Bank, term: &str) -> String {
  format!(
    "/browse?bank={bank}&q={}",
    String::from(js_sys::encode_uri_component(term))
  )
}
