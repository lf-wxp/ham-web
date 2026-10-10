//! 闯关地图：每个一级分类是一关，按顺序逐关解锁，通关给 1–3 星。
//!
//! 与 `/practice`（自由练习）的分工：练习页是「想练什么练什么」，地图是「有序推进 + 反馈」。
//! 两者共用同一套作答记录，所以在地图里答错的题照样进错题本、在练习里答对的题照样涨经验。
//!
//! 目录名不叫 `map`：`pages/map/` 已经是世界地图（灰线 / DXCC）共用的绘制模块。

mod next_quest;
mod quest_map_page;
mod stage_node;

pub use quest_map_page::QuestMapPage;
