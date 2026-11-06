//! 题库分类浏览：按题目类型分类，列出全部选项并标出正确答案，附解析、知识点与参考依据。

mod browse_page;
mod question_row;
mod stat;

pub use browse_page::BrowsePage;
// 收藏集复用同一张题目卡，两处的题目展示保持一致。
pub(crate) use question_row::QuestionRow;

use ham_web_core::QuestionItem;
use ham_web_core::categories::{self};

/// 列表每次多显示几条：分类浏览与收藏集共用，「加载更多」的步长只有一个来源。
pub(crate) const PAGE: usize = 40;
const ALL: &str = "全部";
const OTHER: &str = "其他";

fn top_of(q: &QuestionItem) -> Option<&'static str> {
  categories::sub_category(q.p_code()?).map(|s| s.top)
}

fn sub_name_of(q: &QuestionItem) -> Option<&'static str> {
  categories::sub_category(q.p_code()?).map(|s| s.name)
}
