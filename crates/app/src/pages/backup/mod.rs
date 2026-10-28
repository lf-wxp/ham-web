//! 数据备份：本地数据的导出 / 导入与存储占用一览。
//!
//! 原本是小工具页（`/tools`）里最后一个区块，埋在 60 多个计算器之后；
//! 存储写满、换设备、误删恢复这类场景都要求它「随手能找到」，因此独立成页。

mod backup_page;
mod backup_tool;
mod remote_sync_panel;

pub use backup_page::BackupPage;
