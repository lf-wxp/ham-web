//! 简语速查页：业余无线电通联用语与缩语，独立于「术语表」单独展示。
//!
//! 数据取自 `data/glossary/slang.json`，本页只展示其中一级分类为「用语」的词条，
//! 并进一步分为 Q 简语、通联缩语、通联用语三组，支持「常用 / 全部」视图切换。

mod qcode_page;
mod qcode_quiz;
mod qcode_view;
mod slang_card;

pub use qcode_page::QCodePage;
