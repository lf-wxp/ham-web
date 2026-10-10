//! 回合战：闯关地图与怪物图鉴的战斗入口。
//!
//! 一场战斗 = 一局关卡（`?stage=`）或一队复仇的怪物（`?mode=revenge`）。
//! 规则（伤害 / 暴击 / 星级）在 `ham_web_core::rpg`，这里只做牌组构建、状态流转与视图。
//!
//! 答题仍然走 [`crate::study::record_answer`]：答错自动进错题本、答对推进错题的复习阶段，
//! 战斗只是换了一层「打怪」的表现，不另存一份作答记录。

mod battle_actions;
mod battle_feedback;
mod battle_page;
mod battle_result;
mod battle_stage;
mod deck;
mod result_row;
mod run;

pub use battle_page::BattlePage;
