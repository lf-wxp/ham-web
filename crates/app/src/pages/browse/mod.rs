//! 题库分类浏览：按题目类型分类，仅显示正确答案，附解析、知识点与参考依据。

mod browse_page;
mod question_row;
mod stat;

pub use browse_page::BrowsePage;

use ham_web_core::QuestionItem;
use ham_web_core::categories::{self};

use crate::cn::cn;

const PAGE: usize = 40;
const ALL: &str = "全部";
const OTHER: &str = "其他";

fn top_of(q: &QuestionItem) -> Option<&'static str> {
  categories::sub_category(q.p_code()?).map(|s| s.top)
}

fn sub_name_of(q: &QuestionItem) -> Option<&'static str> {
  categories::sub_category(q.p_code()?).map(|s| s.name)
}

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
