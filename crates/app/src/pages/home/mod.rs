//! 首页：等级与备考信号、今日任务、考试中心 / 知识库 / 工具三大模块入口，并附每日一题与实时传播。
//!
//! 两套 UI 各自一份页面实现：`pixel_home`（像素风「值机台」）与
//! `kit_modern::ModernHome`（现代风）。入口 `home_page` 只做派发。

mod base_hero;
mod cards_card;
mod daily_question;
mod home_page;
mod knowledge_spectrum;
mod log_strip;
mod modules;
mod pixel_home;
mod propagation_widget;
mod pv_metric;
mod quest_board;
mod quest_row;
mod review_card;
mod spectrum_bar;
mod wanted_card;

pub use home_page::HomePage;
