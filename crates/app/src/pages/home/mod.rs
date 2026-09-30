//! 首页：站点门户，展示考试中心 / 知识库 / 工具三大模块入口，并附每日一题与实时传播。

mod daily_question;
mod home_page;
mod propagation_widget;
mod pv_metric;

pub use home_page::HomePage;
