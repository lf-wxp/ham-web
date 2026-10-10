//! 怪物图鉴：错题的「怪物」视图。
//!
//! 每道错题是一只怪物：同一专题长得一样，生命 = 还需连续答对几次才算掌握，
//! 被「掌握」的错题会记进图鉴的已击败名单（见 `study::record_items`）。
//! 数据全部来自错题本，没有另存一份；经典列表仍在 `/mistakes`。

mod bestiary_page;
mod bestiary_summary;
mod monster_card;
mod topic_bars;

pub use bestiary_page::BestiaryPage;
